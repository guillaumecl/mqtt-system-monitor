use crate::configuration::Configuration;
use crate::home_assistant::{RegistrationDescriptor, Sensor};
use crate::status::StatusMessage;
use log::{debug, error, info, trace};
use rumqttc::{AsyncClient, ClientError, MqttOptions, QoS};
use std::error::Error;
use sysinfo::{
    Component, Components, CpuRefreshKind, MemoryRefreshKind, Networks, RefreshKind, System,
};
use tokio::signal::unix::SignalKind;
use tokio::task;
use tokio::time::sleep;

pub struct Daemon {
    config: Configuration,
    mqtt_config: MqttOptions,
    registration_descriptor: RegistrationDescriptor,

    system: System,
    network: Networks,
    temp_component: Option<Component>,

    last_total_transmitted: Option<u64>,
    last_total_received: Option<u64>,
}

impl Daemon {
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
            last_total_transmitted: None,
            last_total_received: None,
            config,
        }
    }

    fn select_temp_component(components: Components, temp_name: Option<&str>) -> Option<Component> {
        let mut cmps = Vec::from(components);
        let temp_label = temp_name?;

        while let Some(c) = cmps.pop() {
            if c.label() == temp_label {
                return Some(c);
            }
        }
        None
    }

    pub fn update_data(self: &mut Daemon) -> StatusMessage {
        self.system.refresh_cpu_usage();

        self.network.refresh(true);
        let (net_tx, net_rx) = self.select_network();

        let component = &mut self.temp_component;
        if let Some(c) = component {
            c.refresh();
        }

        StatusMessage {
            cpu_usage: self.system.global_cpu_usage(),
            disk_usage: None,
            cpu_temp: component.as_ref().and_then(|c| c.temperature()),
            net_tx: Self::update_rate(
                &mut self.last_total_transmitted,
                net_tx,
                self.config.mqtt.update_period,
            ),
            net_rx: Self::update_rate(
                &mut self.last_total_received,
                net_rx,
                self.config.mqtt.update_period,
            ),
        }
    }

    fn select_network(&mut self) -> (Option<u64>, Option<u64>) {
        if let Some(network) = &self.config.sensors.network.as_deref() {
            for (interface, net) in &self.network {
                if network == interface {
                    return (Some(net.total_transmitted()), Some(net.total_received()));
                }
            }
        };

        (None, None)
    }

    fn update_rate(
        last_val: &mut Option<u64>,
        cur: Option<u64>,
        update_period: u64,
    ) -> Option<f64> {
        let cur = cur?;
        let last = *last_val;
        *last_val = Some(cur);

        if let Some(last) = last
            && last <= cur
        {
            Some(((cur - last) / update_period) as f64 / 1024.0)
        } else {
            None
        }
    }

    pub fn register_sensors(&mut self) {
        let entity = self.config.mqtt.entity.as_str();
        self.registration_descriptor
            .add_component(Sensor::CpuUsage, entity);
        if self.temp_component.is_some() {
            self.registration_descriptor
                .add_component(Sensor::CpuTemperature, entity);
        }
        if self.config.sensors.network.is_some() {
            self.registration_descriptor
                .add_component(Sensor::NetTx, entity);
            self.registration_descriptor
                .add_component(Sensor::NetRx, entity);
        }
    }

    pub async fn run(self: &mut Daemon) {
        self.register_sensors();

        let (client, mut eventloop) = AsyncClient::new(self.mqtt_config.clone(), 1);

        task::spawn(async move {
            while let Ok(notification) = eventloop.poll().await {
                trace!("MQTT notification received: {notification:?}");
            }
        });

        self.main_loop(client).await.unwrap_or_else(|e| {
            error!("MQTT main loop failed: {e}");
        });
    }

    async fn main_loop(self: &mut Daemon, client: AsyncClient) -> Result<(), Box<dyn Error>> {
        let mut cycles_counter = 0;

        let expire_cycles = 60 / self.config.mqtt.update_period - 1;
        let sleep_period = std::time::Duration::from_secs(self.config.mqtt.update_period);
        let mut terminal_signal = tokio::signal::unix::signal(SignalKind::terminate())?;

        let topic = format!("mqtt-system-monitor/{}/state", self.config.mqtt.entity);
        loop {
            if cycles_counter == 0 {
                let prefix = &self.config.mqtt.registration_prefix;
                let descriptor = self.registration_descriptor();

                Daemon::publish(
                    &client,
                    descriptor.discovery_topic(prefix),
                    descriptor.to_string(),
                )
                .await?;
            }
            cycles_counter = (cycles_counter + 1) % expire_cycles;

            Daemon::publish(&client, &topic, self.update_data().to_string()).await?;

            tokio::select! {
                _ = sleep(sleep_period) => {},
                _ = tokio::signal::ctrl_c() => {
                    debug!("Ctrl-C received");
                    return Ok(())
                },
                _ = terminal_signal.recv() => {
                    debug!("Interrupt received");
                    return Ok(())
                }
            };
        }
    }

    pub fn registration_descriptor(&self) -> &RegistrationDescriptor {
        &self.registration_descriptor
    }

    async fn publish<S>(client: &AsyncClient, topic: S, data: String) -> Result<(), ClientError>
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
    fn test_update_rate() {
        let mut start: Option<u64> = None;

        // As long as we don't have any data to send, the start stays at None
        assert_eq!(Daemon::update_rate(&mut start, None, 10), None);
        assert_eq!(start, None);

        // At first iteration we return None because the rate is not known yet
        assert_eq!(Daemon::update_rate(&mut start, Some(10), 10), None);
        assert_eq!(start, Some(10));

        // The total received was increased by 20 KiBytes, divided by the update of 10 is 2 KiBytes/s
        assert_eq!(
            Daemon::update_rate(&mut start, Some(10 + 2 * 1024 * 10), 10),
            Some(2.0)
        );
        assert_eq!(start, Some(10 + 2 * 1024 * 10));
    }
}
