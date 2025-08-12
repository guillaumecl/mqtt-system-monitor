use convert_case::{Case, Casing};
use serde::Serialize;
use std::collections::HashMap;
use std::fmt;
use strum_macros::EnumIter;

/// Contains the different types of sensors that are available
#[derive(Debug, PartialEq, EnumIter)]
pub enum Sensor {
    /// Tells if Home Assistant is available
    Available,

    /// Sends the CPU usage in %
    CpuUsage,

    /// Sends a temperature in °C
    Temperature(String, String),

    /// Sends the memory usage in %
    MemoryUsage,

    /// Sends the download network rate in KiB/s
    NetRx(String),

    /// Sends the upload network rate in KiB/s
    NetTx(String),
}

impl Sensor {
    /// Name of the sensor type as sent in the status.
    pub fn as_string(&self) -> String {
        match self {
            Sensor::Available => "available".to_string(),
            Sensor::CpuUsage => "cpu_usage".to_string(),
            Sensor::Temperature(id, _) => format!("{id}_temp"),
            Sensor::MemoryUsage => "memory_usage".to_string(),
            Sensor::NetRx(interface) => format!("{interface}_net_rx"),
            Sensor::NetTx(interface) => format!("{interface}_net_tx"),
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
    components: HashMap<String, DeviceComponent>,

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
    name: Option<String>,

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
    #[serde(skip_serializing_if = "Option::is_none")]
    state_class: Option<&'static str>,

    /// Unit used in the report
    #[serde(skip_serializing_if = "Option::is_none")]
    unit_of_measurement: Option<&'static str>,

    /// Unique ID for the component. This is constructed from the entity and the sensor type
    unique_id: String,

    /// Tells Home Assistant where to find the value in the JSON payload
    value_template: String,

    /// How long to keep the data when Home Assistant doesn't receive any data, in seconds
    #[serde(skip_serializing_if = "Option::is_none")]
    expire_after: Option<u64>,
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
    pub fn new(name: &str) -> RegistrationDescriptor {
        let version = env!("CARGO_PKG_VERSION");
        let package_name = env!("CARGO_PKG_NAME");
        let url = env!("CARGO_PKG_HOMEPAGE");
        let entity = name.to_case(Case::Snake);

        RegistrationDescriptor {
            device: Device {
                name: name.to_string(),
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
            sensor.as_string(),
            DeviceComponent::new(sensor, self.device.identifiers.as_str()),
        );
    }

    /// Returns `true` if the sensor is configured
    pub fn has_sensor(&self, sensor: Sensor) -> bool {
        self.components.contains_key(&sensor.as_string())
    }

    /// Removes the sensor from this descriptor
    pub fn remove_sensor(&mut self, sensor: Sensor) {
        self.components.remove(&sensor.as_string());
    }

    /// Discovery topic for this sensor if individual updates are sent
    pub fn discovery_topic(&self, prefix: &str) -> String {
        format!("{prefix}/device/{}/config", self.device.identifiers)
    }

    /// Discovery topic for this sensor if individual updates are sent
    pub fn state_topic(&self) -> &str {
        &self.state_topic
    }

    /// Returns the registration descriptor
    pub fn components(&self) -> &HashMap<String, DeviceComponent> {
        &self.components
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
            Sensor::Available => Self::available(entity),
            Sensor::CpuUsage => Self::cpu_usage(entity),
            Sensor::MemoryUsage => Self::memory_usage(entity),
            Sensor::NetRx(interface) => Self::net_rx(entity, &interface),
            Sensor::NetTx(interface) => Self::net_tx(entity, &interface),
            Sensor::Temperature(id, label) => Self::temperature(entity, &id, &label),
        }
    }

    /// Manually creates a CPU temperature sensor
    fn available(entity: &str) -> DeviceComponent {
        DeviceComponent {
            name: None,
            platform: "binary_sensor",
            device_class: Some("connectivity"),
            icon: None,
            state_class: None,
            unit_of_measurement: None,
            unique_id: format!("{entity}_available"),
            value_template: "{{ value_json.available }}".to_string(),
            expire_after: None,
        }
    }

    /// Manually creates a CPU usage sensor
    fn cpu_usage(entity: &str) -> DeviceComponent {
        DeviceComponent {
            name: Some("CPU usage".to_string()),
            platform: "sensor",
            device_class: None,
            state_class: Some("measurement"),
            icon: Some("mdi:cpu-64-bit"),
            unit_of_measurement: Some("%"),
            unique_id: format!("{entity}_cpu_usage"),
            value_template: "{{ value_json.cpu_usage }}".to_string(),
            expire_after: Some(60),
        }
    }

    /// Manually creates a Memory usage sensor
    fn memory_usage(entity: &str) -> DeviceComponent {
        DeviceComponent {
            name: Some("Memory usage".to_string()),
            platform: "sensor",
            device_class: None,
            state_class: Some("measurement"),
            icon: Some("mdi:memory"),
            unit_of_measurement: Some("%"),
            unique_id: format!("{entity}_memory_usage"),
            value_template: "{{ value_json.memory_usage }}".to_string(),
            expire_after: Some(60),
        }
    }

    /// Manually creates a temperature sensor
    fn temperature(entity: &str, id: &str, label: &str) -> DeviceComponent {
        DeviceComponent {
            name: Some(label.to_string()),
            platform: "sensor",
            device_class: Some("temperature"),
            icon: None,
            state_class: Some("measurement"),
            unit_of_measurement: Some("°C"),
            unique_id: format!("{entity}_{id}_temp"),
            value_template: format!(
                "{{{{ value_json.temperature.{id} if value_json.temperature and value_json.temperature.{id} else None }}}}"
            ),
            expire_after: Some(60),
        }
    }

    /// Manually creates a Network RX sensor
    fn net_rx(entity: &str, interface: &str) -> DeviceComponent {
        DeviceComponent {
            name: Some(format!("{interface} Network RX rate")),
            platform: "sensor",
            device_class: Some("data_rate"),
            state_class: Some("measurement"),
            icon: None,
            unit_of_measurement: Some("KiB/s"),
            unique_id: format!("{entity}_{interface}_net_rx"),
            value_template: format!(
                "{{{{ value_json.network.{interface}.rx if value_json.network.{interface} else None }}}}"
            ),
            expire_after: Some(60),
        }
    }

    /// Manually creates a Network TX sensor
    fn net_tx(entity: &str, interface: &str) -> DeviceComponent {
        DeviceComponent {
            name: Some(format!("{interface} Network TX rate")),
            platform: "sensor",
            device_class: Some("data_rate"),
            state_class: Some("measurement"),
            icon: None,
            unit_of_measurement: Some("KiB/s"),
            unique_id: format!("{entity}_{interface}_net_tx"),
            value_template: format!(
                "{{{{ value_json.network.{interface}.tx if value_json.network.{interface} else None }}}}"
            ),
            expire_after: Some(60),
        }
    }

    pub fn value_template(&self) -> &str {
        &self.value_template
    }
}

#[cfg(test)]
mod tests {
    use crate::DeviceComponent;
    use crate::home_assistant::{RegistrationDescriptor, Sensor};
    use strum::IntoEnumIterator;

    #[test]
    fn test_registration() {
        let name = "Test Entity";
        let entity = "test_entity";
        let mut descriptor = RegistrationDescriptor::new(name);

        Sensor::iter().for_each(|sensor| descriptor.add_component(sensor));

        assert_eq!(descriptor.device.name, name);
        assert_eq!(descriptor.device.identifiers, entity);

        assert_eq!(
            descriptor.state_topic,
            "mqtt-system-monitor/test_entity/state"
        );

        for component in &descriptor.components {
            assert_eq!(component.1.unique_id, format!("{entity}_{}", component.0));
            if component.1.name.is_some() {
                assert_eq!(component.1.state_class, Some("measurement"));
            }
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

        for sensor in Sensor::iter() {
            let name = sensor.as_string();
            let component = DeviceComponent::new(sensor, entity);

            assert_eq!(component.unique_id, format!("{entity}_{name}"));
        }
    }
}
