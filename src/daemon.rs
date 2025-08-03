use crate::configuration::Configuration;
use crate::status::StatusMessage;
use log::{debug, error, info, trace};
use rumqttc::{AsyncClient, ClientError, MqttOptions, QoS};
use std::sync::atomic::{AtomicBool, Ordering};
use sysinfo::{
    Component, Components, CpuRefreshKind, MemoryRefreshKind, Networks, RefreshKind, System,
};
use tokio::task;
use tokio::time::sleep;

pub struct Daemon {
    config: Configuration,
    mqtt_config: MqttOptions,

    stop: AtomicBool,

    system: System,
    network: Networks,
    temp_component: Option<Component>,

    last_total_transmitted: u64,
    last_total_received: u64,
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
            stop: AtomicBool::new(false),
            system,
            network,
            temp_component: Self::select_temp_component(
                components,
                config.sensors.temperature.as_deref(),
            ),
            last_total_transmitted: 0,
            last_total_received: 0,
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

    fn update_rate(last_val: &mut u64, cur: Option<u64>, update_period: u64) -> Option<f64> {
        let cur = cur?;
        let last = *last_val;
        *last_val = cur;

        if last > 0 && last <= cur {
            Some(((cur - last) / update_period) as f64 / 1024.0)
        } else {
            None
        }
    }

    pub async fn run(self: &mut Daemon) {
        let mut cycles_counter = 0;
        let mut register: bool;

        let expire_cycles = 60 / self.config.mqtt.update_period - 1;
        let sleep_period = std::time::Duration::from_secs(self.config.mqtt.update_period);

        let topic = format!("mqtt-system-monitor/{}/state", self.config.mqtt.entity);

        let (client, mut eventloop) = AsyncClient::new(self.mqtt_config.clone(), 1);

        task::spawn(async move {
            while let Ok(notification) = eventloop.poll().await {
                trace!("MQTT notification received: {notification:?}");
            }
        });

        while !self.stop.load(Ordering::Relaxed) {
            register = cycles_counter == 0;

            if Daemon::publish(&client, &topic, self.update_data().to_string())
                .await
                .is_err()
            {
                break;
            }

            if register {
                let registration_message = self.registration_message();

                if Daemon::publish(&client, registration_message.0, registration_message.1)
                    .await
                    .is_err()
                {
                    break;
                };
            }

            if cycles_counter == expire_cycles {
                cycles_counter = 0;
            } else {
                cycles_counter += 1;
            }

            sleep(sleep_period).await;
        }
    }

    pub fn registration_message(&self) -> (String, String) {
        let id = &self.config.mqtt.entity;
        let prefix = &self.config.mqtt.registration_prefix;
        let version = env!("CARGO_PKG_VERSION");
        let package_name = env!("CARGO_PKG_NAME");
        let url = env!("CARGO_PKG_HOMEPAGE");

        (
            format!("{prefix}/device/{}/config", self.config.mqtt.entity),
            format!(
                r#"{{
  "device": {{
    "name": "{id}",
    "identifiers": "{id}"
  }},
  "origin": {{
    "name": "{package_name}",
    "sw_version": "{version}",
    "url": "{url}"
  }},
  "components": {{
    "cpu_temp": {{
      "name": "{id} CPU temperature",
      "platform": "sensor",
      "device_class": "temperature",
      "state_class": "measurement",
      "unit_of_measurement": "°C",
      "unique_id": "cpu_temp",
      "value_template": "{{{{ value_json.cpu_temp }}}}",
      "expire_after": 60
    }},
    "cpu_usage": {{
      "name": "{id} CPU usage",
      "platform": "sensor",
      "device_class": null,
      "icon": "mdi:cpu-64-bit",
      "state_class": "measurement",
      "unit_of_measurement": "%",
      "unique_id": "cpu_usage",
      "value_template": "{{{{ value_json.cpu_usage }}}}",
      "expire_after": 60
    }},
    "net_rx": {{
      "name": "{id} Network RX rate",
      "platform": "sensor",
      "device_class": "data_rate",
      "state_class": "measurement",
      "unit_of_measurement": "KiB/s",
      "unique_id": "net_rx",
      "value_template": "{{{{ value_json.net_rx }}}}",
      "expire_after": 60
    }},
    "net_tx": {{
      "name": "{id} Network TX rate",
      "platform": "sensor",
      "device_class": "data_rate",
      "state_class": "measurement",
      "unit_of_measurement": "KiB/s",
      "unique_id": "net_tx",
      "value_template": "{{{{ value_json.net_tx }}}}",
      "expire_after": 60
    }},
    "disk_usage": {{
      "name": "{id} Disk usage",
      "platform": "sensor",
      "device_class": "data_size",
      "state_class": "measurement",
      "unit_of_measurement": "B",
      "unique_id": "disk_usage",
      "value_template": "{{{{ value_json.disk_usage }}}}",
      "expire_after": 60
    }}
  }},
  "state_topic": "mqtt-system-monitor/{id}/state"
}}"#
            ),
        )
    }

    async fn publish<S>(client: &AsyncClient, topic: S, data: String) -> Result<(), ClientError>
    where
        S: Into<String> + std::fmt::Display,
    {
        debug!("Publishing to topic {topic} : {data}");
        match client.publish(topic, QoS::AtLeastOnce, false, data).await {
            Err(message) => {
                error!("MQTT publish error: {message}");

                Err(message)
            }
            _ => Ok(()),
        }
    }
}

impl Drop for Daemon {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_update_rate() {
        let mut start = 0;

        assert_eq!(Daemon::update_rate(&mut start, None, 10), None);

        // At first iteration we return None because the rate is not known yet
        assert_eq!(Daemon::update_rate(&mut start, Some(10), 10), None);
        assert_eq!(start, 10);

        // The total received was increased by 20 KiBytes, divided by the update of 10 is 2 KiBytes/s
        assert_eq!(
            Daemon::update_rate(&mut start, Some(10 + 2 * 1024 * 10), 10),
            Some(2.0)
        );
        assert_eq!(start, 10 + 2 * 1024 * 10);
    }
}
