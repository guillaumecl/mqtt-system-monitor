//! # mqtt-system-monitor
//!
//! `mqtt-system-monitor` allows sending reports to the MQTT integration of Home Assistant
//!
//!

pub use self::configuration::Configuration;
pub use self::configuration::Mqtt;
pub use self::configuration::Sensors;
pub use self::daemon::Daemon;
pub use self::home_assistant::DeviceComponent;
pub use self::home_assistant::RegistrationDescriptor;
pub use self::home_assistant::Sensor;
pub use self::status::StatusMessage;

/// Contains the configuration stuff
pub mod configuration;
/// Contains the daemon code
pub mod daemon;
/// Contains Home Assistant registration data
pub mod home_assistant;
/// Contains the status that is sent to MQTT
pub mod status;
