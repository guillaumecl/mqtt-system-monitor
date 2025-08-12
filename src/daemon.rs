use crate::configuration::Configuration;
use crate::home_assistant::{RegistrationDescriptor, Sensor};
use crate::status::{NetworkStatus, StatusMessage};
use log::{debug, error, info, trace};
use rumqttc::{AsyncClient, ClientError, MqttOptions, QoS};
use std::collections::HashMap;
use std::error::Error;
use sysinfo::{
    Component, Components, CpuRefreshKind, MemoryRefreshKind, Networks, RefreshKind, System,
};
use tokio::signal::unix::SignalKind;
use tokio::task;
use tokio::time::sleep;

/// Daemon that periodically sends reports to MQTT
pub struct Daemon {
    config: Configuration,
    mqtt_config: MqttOptions,
    registration_descriptor: RegistrationDescriptor,

    system: System,
    network: Networks,
    temp_component: Option<Component>,
}

impl Daemon {
    /// Constructs a daemon from the specified configuration
    ///
    /// ```
    /// use mqtt_system_monitor::{Configuration, Daemon};
    ///
    /// let config = Configuration::load("conf/mqtt-system-monitor.conf").expect("Cannot load configuration");
    /// let mut daemon = Daemon::new(config);
    ///
    /// // later, run daemon.run() in an async function
    /// ```
    pub fn new(config: Configuration) -> Daemon {
        info!("Daemon for {} starting", config.mqtt.entity);

        let system = System::new_with_specifics(
            RefreshKind::nothing()
                .with_cpu(CpuRefreshKind::everything())
                .with_memory(MemoryRefreshKind::nothing().with_ram()),
        );

        let network = Networks::new_with_refreshed_list();

        let components = Components::new_with_refreshed_list();

        let mut mqtt_config =
            MqttOptions::new(&config.mqtt.entity, &config.mqtt.host, config.mqtt.port);
        mqtt_config.set_credentials(&config.mqtt.user, &config.mqtt.password);

        info!(
            "Connecting to MQTT broker {}:{}",
            config.mqtt.host, config.mqtt.port
        );

        Daemon {
            mqtt_config,
            registration_descriptor: RegistrationDescriptor::new(&config.mqtt.entity),
            system,
            network,
            temp_component: Self::select_temp_component(
                components,
                config.sensors.temperature.as_deref(),
            ),
            config,
        }
    }

    /// Selects the temperature component that corresponds to the configured sensor
    ///
    /// Returns `None` if not configured or if nothing is found.
    fn select_temp_component(components: Components, temp_id: Option<&str>) -> Option<Component> {
        let temp_id = temp_id?;
        Vec::from(components)
            .into_iter()
            .find(|c| c.id() == Some(temp_id))
    }

    /// Updates the data and returns a status message
    pub fn update_data(self: &mut Daemon) -> StatusMessage {
        if self.registration_descriptor.has_sensor(Sensor::CpuUsage) {
            self.system.refresh_cpu_usage();
        }
        if self.registration_descriptor.has_sensor(Sensor::MemoryUsage) {
            self.system.refresh_memory();
        }

        if !self.config.sensors.network.is_empty() {
            self.network.refresh(true);
        }

        let component = &mut self.temp_component;
        if self
            .registration_descriptor
            .has_sensor(Sensor::CpuTemperature)
            && let Some(c) = component
        {
            c.refresh();
        }

        StatusMessage {
            available: "ON",
            cpu_usage: Some(self.system.global_cpu_usage()),
            memory_usage: Some(
                100.0 * (self.system.used_memory() as f32 / self.system.total_memory() as f32),
            ),
            cpu_temp: component.as_ref().and_then(|c| c.temperature()),
            network: self.select_network(),
        }
    }

    /// Selects the current network values according to the configured interfaces
    fn select_network(&self) -> HashMap<String, NetworkStatus> {
        let mut map = HashMap::new();
        for interface in &self.config.sensors.network {
            if let Some((_, network_data)) = self.network.iter().find(|n| n.0 == interface) {
                map.insert(
                    interface.clone(),
                    NetworkStatus {
                        tx: self.rate(network_data.transmitted()),
                        rx: self.rate(network_data.received()),
                    },
                );
            };
        }

        map
    }

    fn rate(&self, diff: u64) -> f64 {
        (diff / self.config.mqtt.update_period) as f64 / 1024.0
    }

    /// Registers the configured sensors in the descriptor
    pub fn register_sensors(&mut self) {
        self.registration_descriptor
            .add_component(Sensor::Available);
        self.registration_descriptor.add_component(Sensor::CpuUsage);
        self.registration_descriptor
            .add_component(Sensor::MemoryUsage);
        if self.temp_component.is_some() {
            self.registration_descriptor
                .add_component(Sensor::CpuTemperature);
        }
        for interface in &self.config.sensors.network {
            debug!("Adding interface {interface}");
            self.registration_descriptor
                .add_component(Sensor::NetTx(interface.clone()));
            self.registration_descriptor
                .add_component(Sensor::NetRx(interface.clone()));
        }
    }

    /// Runs the main loop that periodically sends the MQTT events
    pub async fn run(self: &mut Daemon) {
        self.register_sensors();

        let (client, mut event_loop) = AsyncClient::new(self.mqtt_config.clone(), 1);

        task::spawn(async move {
            while let Ok(notification) = event_loop.poll().await {
                trace!("MQTT notification received: {notification:?}");
            }
        });

        self.main_loop(client).await.unwrap_or_else(|e| {
            error!("MQTT main loop failed: {e}");
        });
    }

    /// Single iteration of the main loop
    async fn main_loop(self: &mut Daemon, client: AsyncClient) -> Result<(), Box<dyn Error>> {
        let mut cycles_counter = 0;
        let expire_cycles = 60 / self.config.mqtt.update_period - 1;
        let sleep_period = std::time::Duration::from_secs(self.config.mqtt.update_period);
        let mut terminal_signal = tokio::signal::unix::signal(SignalKind::terminate())?;
        let topic = self.registration_descriptor.state_topic().to_string();

        self.publish_registration(&client).await?;
        sleep(std::time::Duration::from_secs(1)).await;

        loop {
            cycles_counter = (cycles_counter + 1) % expire_cycles;
            if cycles_counter == 0 {
                self.publish_registration(&client).await?;
            }

            self.publish_update(&client, &topic).await?;
            tokio::select! {
                _ = sleep(sleep_period) => {},
                _ = tokio::signal::ctrl_c() => {
                    debug!("Ctrl-C received");
                    break;
                },
                _ = terminal_signal.recv() => {
                    debug!("Interrupt received");
                    break;
                }
            }
        }

        Daemon::publish(&client, topic, &StatusMessage::off().to_string()).await?;

        sleep(std::time::Duration::from_secs(1)).await;

        Ok(())
    }

    // Publish an update to MQTT
    async fn publish_update(
        self: &mut Daemon,
        client: &AsyncClient,
        topic: &str,
    ) -> Result<(), Box<dyn Error>> {
        Daemon::publish(client, topic, &self.update_data().to_string()).await?;

        Ok(())
    }

    /// Returns the registration descriptor
    pub fn registration_descriptor(&self) -> &RegistrationDescriptor {
        &self.registration_descriptor
    }

    async fn publish_registration(&self, client: &AsyncClient) -> Result<(), ClientError> {
        let prefix = &self.config.mqtt.registration_prefix;
        let descriptor = self.registration_descriptor();

        Daemon::publish(
            client,
            descriptor.discovery_topic(prefix),
            &descriptor.to_string(),
        )
        .await
    }

    // Publish a message to MQTT
    async fn publish<S>(client: &AsyncClient, topic: S, data: &str) -> Result<(), ClientError>
    where
        S: Into<String> + std::fmt::Display,
    {
        debug!("Publishing to topic {topic} : {data}");
        client.publish(topic, QoS::AtLeastOnce, false, data).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rate() {
        let config = Configuration::load("conf/mqtt-system-monitor.conf")
            .expect("Failed to load default config");
        let mut daemon = Daemon::new(config);

        daemon.config.mqtt.update_period = 1;
        assert_eq!(daemon.rate(1024), 1.0);

        daemon.config.mqtt.update_period = 10;
        // The total received was increased by 20 KiBytes, divided by the update of 10 is 2 KiBytes/s
        assert_eq!(daemon.rate(2 * 1024 * 10), 2.0);
    }
}
