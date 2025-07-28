use serde::Deserialize;
use serde_inline_default::serde_inline_default;
use std::error::Error;

#[serde_inline_default]
#[derive(Deserialize)]
pub struct Mqtt {
    #[serde_inline_default(String::from("localhost"))]
    pub host: String,
    #[serde_inline_default(1883)]
    pub port: u16,

    #[serde(default)]
    pub user: String,

    #[serde(default)]
    pub password: String,

    #[serde_inline_default(String::from("homeassistant"))]
    #[serde(rename = "registration-prefix")]
    pub registration_prefix: String,

    #[serde_inline_default(10)]
    pub update_period: u64,

    #[serde(default = "hostname")]
    pub entity: String,
}
#[derive(Deserialize)]
pub struct Sensors {
    pub temperature: Option<String>,
    pub network: Option<String>,
}

#[serde_inline_default]
#[derive(Deserialize)]
pub struct Configuration {
    pub mqtt: Mqtt,

    pub sensors: Sensors,

    #[serde_inline_default(2)]
    #[serde(rename = "log-verbosity")]
    pub log_verbosity: usize,
}

fn hostname() -> String {
    sysinfo::System::host_name().expect("Cannot read hostname")
}

impl Configuration {
    pub fn load(path: &str) -> Result<Configuration, Box<dyn Error>> {
        toml::from_str(std::fs::read_to_string(&path)?.as_str()).map_err(|err| err.into())
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
        assert_eq!(conf.sensors.network, None);

        Ok(())
    }
}
