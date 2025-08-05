use serde::Serialize;
use std::collections::HashMap;
use std::fmt;

#[derive(Debug)]
pub enum Sensor {
    CpuUsage,
    CpuTemperature,
    NetRx,
    NetTx,
}

impl Sensor {
    pub fn as_str(&self) -> &'static str {
        match self {
            Sensor::CpuUsage => "cpu_usage",
            Sensor::CpuTemperature => "cpu_temp",
            Sensor::NetRx => "net_rx",
            Sensor::NetTx => "net_tx",
        }
    }
}

#[derive(Serialize, Debug)]
pub struct RegistrationDescriptor {
    device: Device,
    origin: Origin,
    components: HashMap<&'static str, DeviceComponent>,
    state_topic: String,
}

#[derive(Serialize, Debug)]
pub struct Device {
    name: String,
    identifiers: String,
}

#[derive(Serialize, Debug)]
pub struct Origin {
    name: &'static str,
    sw_version: &'static str,
    url: &'static str,
}

#[derive(Serialize, Debug)]
pub struct DeviceComponent {
    name: &'static str,
    platform: &'static str,
    device_class: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    icon: Option<&'static str>,
    state_class: &'static str,
    unit_of_measurement: &'static str,
    unique_id: String,
    value_template: &'static str,
    expire_after: u64,
}

impl RegistrationDescriptor {
    pub fn new(entity: &str) -> RegistrationDescriptor {
        let version = env!("CARGO_PKG_VERSION");
        let package_name = env!("CARGO_PKG_NAME");
        let url = env!("CARGO_PKG_HOMEPAGE");

        RegistrationDescriptor {
            device: Device {
                name: entity.to_string(),
                identifiers: entity.to_string(),
            },
            origin: Origin {
                name: package_name,
                sw_version: version,
                url,
            },
            components: Default::default(),
            state_topic: format!("mqtt-system-monitor/{entity}/state"),
        }
    }

    pub fn add_component(&mut self, sensor: Sensor, entity: &str) {
        self.components
            .insert(sensor.as_str(), DeviceComponent::new(sensor, entity));
    }

    pub fn has_sensor(&self, sensor: Sensor) -> bool {
        self.components.contains_key(sensor.as_str())
    }

    pub fn remove_sensor(&mut self, sensor: Sensor) {
        self.components.remove(sensor.as_str());
    }

    pub fn discovery_topic(&self, prefix: &str) -> String {
        format!("{prefix}/device/{}/config", self.device.name)
    }
}

impl fmt::Display for RegistrationDescriptor {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let Ok(descriptor) = serde_json::to_string(&self) else {
            return Err(fmt::Error);
        };
        write!(f, "{descriptor}")
    }
}

impl DeviceComponent {
    pub fn new(sensor: Sensor, entity: &str) -> DeviceComponent {
        match sensor {
            Sensor::CpuTemperature => Self::cpu_temperature(entity),
            Sensor::CpuUsage => Self::cpu_usage(entity),
            Sensor::NetRx => Self::net_rx(entity),
            Sensor::NetTx => Self::net_tx(entity),
        }
    }
    pub fn cpu_temperature(entity: &str) -> DeviceComponent {
        DeviceComponent {
            name: "CPU temperature",
            platform: "sensor",
            device_class: Some("temperature"),
            icon: None,
            state_class: "measurement",
            unit_of_measurement: "°C",
            unique_id: format!("{entity}_cpu_temp"),
            value_template: "{{ value_json.cpu_temp }}",
            expire_after: 60,
        }
    }

    pub fn cpu_usage(entity: &str) -> DeviceComponent {
        DeviceComponent {
            name: "CPU usage",
            platform: "sensor",
            device_class: None,
            state_class: "measurement",
            icon: Some("mdi:cpu-64-bit"),
            unit_of_measurement: "%",
            unique_id: format!("{entity}_cpu_usage"),
            value_template: "{{ value_json.cpu_usage }}",
            expire_after: 60,
        }
    }

    pub fn net_rx(entity: &str) -> DeviceComponent {
        DeviceComponent {
            name: "Network RX rate",
            platform: "sensor",
            device_class: Some("data_rate"),
            state_class: "measurement",
            icon: None,
            unit_of_measurement: "KiB/s",
            unique_id: format!("{entity}_net_rx"),
            value_template: "{{ value_json.net_rx }}",
            expire_after: 60,
        }
    }

    pub fn net_tx(entity: &str) -> DeviceComponent {
        DeviceComponent {
            name: "Network TX rate",
            platform: "sensor",
            device_class: Some("data_rate"),
            state_class: "measurement",
            icon: None,
            unit_of_measurement: "KiB/s",
            unique_id: format!("{entity}_net_tx"),
            value_template: "{{ value_json.net_tx }}",
            expire_after: 60,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::home_assistant::{RegistrationDescriptor, Sensor};

    #[test]
    fn test_registration() -> Result<(), Box<dyn std::error::Error>> {
        let entity = "test_entity";
        let mut descriptor = RegistrationDescriptor::new(entity);

        descriptor.add_component(Sensor::CpuUsage, entity);
        descriptor.add_component(Sensor::CpuTemperature, entity);
        descriptor.add_component(Sensor::NetTx, entity);
        descriptor.add_component(Sensor::NetRx, entity);

        assert_eq!(descriptor.device.name, "test_entity");
        assert_eq!(descriptor.device.identifiers, "test_entity");

        assert_eq!(
            descriptor.state_topic,
            "mqtt-system-monitor/test_entity/state"
        );

        for component in &descriptor.components {
            assert_eq!(component.1.unique_id, format!("{entity}_{}", component.0));
            assert_eq!(
                component.1.value_template,
                format!("{{{{ value_json.{} }}}}", component.0).as_str()
            );
            assert_eq!(component.1.state_class, "measurement");
        }

        let cpu_usage = descriptor
            .components
            .get("cpu_usage")
            .expect("component cpu_usage not found");

        assert_eq!(cpu_usage.device_class, None);

        Ok(())
    }
}
