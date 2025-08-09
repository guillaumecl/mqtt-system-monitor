use serde::Deserialize;
use serde_inline_default::serde_inline_default;
use std::error::Error;

/// Contains the configuration for communicating with the MQTT broker
#[serde_inline_default]
#[derive(Deserialize)]
pub struct Mqtt {
    /// Hostname or IP address. Default: localhost
    #[serde_inline_default(String::from("localhost"))]
    pub host: String,

    /// Port of the connection to the broker. Default: 1883
    #[serde_inline_default(1883)]
    pub port: u16,

    /// Username for the connection to the broker. Default: empty
    #[serde(default)]
    pub user: String,

    /// Password for the connection to the broker. Default: empty
    #[serde(default)]
    pub password: String,

    /// Prefix for the registration topic sent to Home Assistant. Default: homeassistant
    ///
    /// This must match the configuration of the MQTT integration in Home Assistant
    ///
    /// See <https://www.home-assistant.io/integrations/mqtt#discovery-options>
    #[serde_inline_default(String::from("homeassistant"))]
    #[serde(rename = "registration-prefix")]
    pub registration_prefix: String,

    /// Delay between each sensor report in seconds. Default: 10 seconds
    #[serde_inline_default(10)]
    pub update_period: u64,

    /// Name of the device entity. It should be unique in Home Assistant. Default: machine hostname
    #[serde(default = "hostname")]
    pub entity: String,
}

/// Contains the configuration for the sensors
#[derive(Deserialize)]
pub struct Sensors {
    /// If set, contains a temperature label to search in `sysinfo`'s component reports.
    pub temperature: Option<String>,

    /// If set, contains a list of network interface to monitor.
    #[serde(default)]
    pub network: Vec<String>,
}

/// Contains all the configuration for `mqtt-system-monitor`
#[serde_inline_default]
#[derive(Deserialize)]
pub struct Configuration {
    /// Contains the configuration for communicating with the MQTT broker
    pub mqtt: Mqtt,

    /// Contains the configuration for the sensors
    pub sensors: Sensors,

    /// Sets the verbosity of the logs.
    ///   * 1 => Error
    ///  * 2 => Warning
    ///  * 3 => Info
    ///  * 4 => Debug
    ///  * 5 => Trace
    #[serde_inline_default(2)]
    #[serde(rename = "log-verbosity")]
    pub log_verbosity: usize,
}

fn hostname() -> String {
    sysinfo::System::host_name().expect("Cannot read hostname")
}

impl Configuration {
    /// Load the configuration from a file
    ///
    /// ## Example
    ///
    /// ```
    /// use mqtt_system_monitor::{configuration, Configuration};
    ///
    /// let config = Configuration::load("conf/mqtt-system-monitor.conf").expect("Cannot load configuration");
    ///
    /// assert_eq!(config.mqtt.host, "localhost");
    /// ```
    pub fn load(path: &str) -> Result<Configuration, Box<dyn Error>> {
        toml::from_str(std::fs::read_to_string(path)?.as_str()).map_err(|err| err.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test that we can properly load the default configuration
    #[test]
    fn test_default_config() -> Result<(), Box<dyn Error>> {
        let conf = Configuration::load("conf/mqtt-system-monitor.conf")?;

        assert_eq!(conf.mqtt.host, String::from("localhost"));
        assert_eq!(conf.mqtt.registration_prefix, String::from("homeassistant"));

        // By default, the entity name will be the hostname of the machine
        assert_eq!(conf.mqtt.entity, hostname());

        // Sensors are off by default
        assert_eq!(conf.sensors.temperature, None);
        assert!(conf.sensors.network.is_empty());

        Ok(())
    }
}
