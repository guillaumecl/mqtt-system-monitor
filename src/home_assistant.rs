use serde::Serialize;
use std::collections::HashMap;
use std::fmt;
use strum_macros::EnumIter;

/// Contains the different types of sensors that are available
#[derive(Debug, PartialEq, EnumIter)]
pub enum Sensor {
    /// Sends the CPU usage in %
    CpuUsage,

    /// Sends the CPU temperature in °C
    CpuTemperature,

    /// Sends the memory usage in %
    MemoryUsage,

    /// Sends the download network rate in KiB/s
    NetRx,

    /// Sends the upload network rate in KiB/s
    NetTx,
}

impl Sensor {
    /// Name of the sensor type as sent in the status.
    pub fn as_str(&self) -> &'static str {
        match self {
            Sensor::CpuUsage => "cpu_usage",
            Sensor::CpuTemperature => "cpu_temp",
            Sensor::MemoryUsage => "memory_usage",
            Sensor::NetRx => "net_rx",
            Sensor::NetTx => "net_tx",
        }
    }
}

/// Registration descriptor sent to Home Assistant
///
/// This describes the device and its components (the sensors that are configured)
#[derive(Serialize, Debug)]
pub struct RegistrationDescriptor {
    /// Device sent to Home Assistant
    device: Device,

    /// Describes the origin of the messages, in this case `mqtt-system-monitor`
    origin: Origin,

    /// Configured device components
    components: HashMap<&'static str, DeviceComponent>,

    /// Topic that is sent to MQTT when the state changes
    state_topic: String,
}

/// Device sent to Home Assistant
#[derive(Serialize, Debug)]
pub struct Device {
    /// Name of the device. This corresponds to the `entity` configuration field
    name: String,

    /// Identifier of the device. This corresponds to the `entity` configuration field
    identifiers: String,
}

/// Describes the origin of the messages, in this case `mqtt-system-monitor`
#[derive(Serialize, Debug)]
pub struct Origin {
    /// Name of the origin, always `mqtt-system-monitor`
    name: &'static str,

    /// Version of `mqtt-system-monitor`
    sw_version: &'static str,

    /// URL of `mqtt-system-monitor`
    url: &'static str,
}

/// Configured device component
#[derive(Serialize, Debug)]
pub struct DeviceComponent {
    /// Name of the component, shown in Home Assistant and is converted into the entity ID
    name: &'static str,

    /// Type of platform. Always `sensor`
    platform: &'static str,

    /// Device class helps Home Assistant to know how to interpret the reported values.
    ///
    /// See <https://www.home-assistant.io/integrations/sensor#device-class> for possible values here
    device_class: Option<&'static str>,

    /// An icon for certain sensors that are too generic (for example when `device_class` is `None`)
    #[serde(skip_serializing_if = "Option::is_none")]
    icon: Option<&'static str>,

    /// Describes how Home Assistant stores the data. It is usually `measurement`
    state_class: &'static str,

    /// Unit used in the report
    unit_of_measurement: &'static str,

    /// Unique ID for the component. This is constructed from the entity and the sensor type
    unique_id: String,

    /// Tells Home Assistant where to find the value in the JSON payload
    value_template: &'static str,

    /// How long to keep the data when Home Assistant doesn't receive any data, in seconds
    expire_after: u64,
}

impl RegistrationDescriptor {
    /// Creates a new registration descriptor, with no sensor configured.
    ///
    /// ## Example
    ///
    /// ```
    /// use mqtt_system_monitor::{ RegistrationDescriptor, Sensor };
    ///
    /// let mut descriptor = RegistrationDescriptor::new("test_entity");
    /// assert!(!descriptor.has_sensor(Sensor::CpuUsage));
    ///
    /// descriptor.add_component(Sensor::CpuUsage);
    /// assert!(descriptor.has_sensor(Sensor::CpuUsage));
    /// ```
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

    /// Adds a component to the descriptor
    ///
    /// ## Example
    ///
    /// ```
    /// use mqtt_system_monitor::{ RegistrationDescriptor, Sensor };
    ///
    /// let mut descriptor = RegistrationDescriptor::new("test_entity");
    /// assert!(!descriptor.has_sensor(Sensor::CpuUsage));
    ///
    /// descriptor.add_component(Sensor::CpuUsage);
    /// assert!(descriptor.has_sensor(Sensor::CpuUsage));
    /// ```
    pub fn add_component(&mut self, sensor: Sensor) {
        self.components.insert(
            sensor.as_str(),
            DeviceComponent::new(sensor, self.device.name.as_str()),
        );
    }

    /// Returns `true` if the sensor is configured
    pub fn has_sensor(&self, sensor: Sensor) -> bool {
        self.components.contains_key(sensor.as_str())
    }

    /// Removes the sensor from this descriptor
    pub fn remove_sensor(&mut self, sensor: Sensor) {
        self.components.remove(sensor.as_str());
    }

    /// Discovery topic for this sensor if individual updates are sent
    pub fn discovery_topic(&self, prefix: &str) -> String {
        format!("{prefix}/device/{}/config", self.device.name)
    }
}

impl fmt::Display for RegistrationDescriptor {
    /// Formats the descriptor in JSON format
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let Ok(descriptor) = serde_json::to_string(&self) else {
            return Err(fmt::Error);
        };
        write!(f, "{descriptor}")
    }
}

impl DeviceComponent {
    /// Creates a new device component from a sensor type
    pub fn new(sensor: Sensor, entity: &str) -> DeviceComponent {
        match sensor {
            Sensor::CpuTemperature => Self::cpu_temperature(entity),
            Sensor::CpuUsage => Self::cpu_usage(entity),
            Sensor::MemoryUsage => Self::memory_usage(entity),
            Sensor::NetRx => Self::net_rx(entity),
            Sensor::NetTx => Self::net_tx(entity),
        }
    }

    /// Manually creates a CPU temperature sensor
    fn cpu_temperature(entity: &str) -> DeviceComponent {
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

    /// Manually creates a CPU usage sensor
    fn cpu_usage(entity: &str) -> DeviceComponent {
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

    /// Manually creates a Memory usage sensor
    fn memory_usage(entity: &str) -> DeviceComponent {
        DeviceComponent {
            name: "Memory usage",
            platform: "sensor",
            device_class: None,
            state_class: "measurement",
            icon: Some("mdi:memory"),
            unit_of_measurement: "%",
            unique_id: format!("{entity}_memory_usage"),
            value_template: "{{ value_json.memory_usage }}",
            expire_after: 60,
        }
    }

    /// Manually creates a Network RX sensor
    fn net_rx(entity: &str) -> DeviceComponent {
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

    /// Manually creates a Network TX sensor
    fn net_tx(entity: &str) -> DeviceComponent {
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
    use crate::{DeviceComponent, StatusMessage};
    use serde_json::Value;
    use std::collections::HashMap;
    use strum::IntoEnumIterator;

    #[test]
    fn test_registration() {
        let entity = "test_entity";
        let mut descriptor = RegistrationDescriptor::new(entity);

        Sensor::iter().for_each(|sensor| descriptor.add_component(sensor));

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
    }

    /// Test that all sensors can be created
    #[test]
    fn test_sensors() {
        let entity = "test_entity";
        let status: StatusMessage = Default::default();
        let status_json: HashMap<String, Value> =
            serde_json::from_str(&status.to_string()).expect("Cannot read default status");

        for sensor in Sensor::iter() {
            let name = sensor.as_str();
            let component = DeviceComponent::new(sensor, entity);

            assert_eq!(component.unique_id, format!("{entity}_{name}"));
            assert_eq!(
                component.value_template,
                format!("{{{{ value_json.{name} }}}}").as_str()
            );

            assert_eq!(component.state_class, "measurement");

            assert!(status_json.contains_key(name));
        }
    }
}
