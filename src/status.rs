use serde::Serialize;
use std::collections::HashMap;
use std::fmt;

/// Message sent to the MQTT broker which later forwards it to Home Assistant
///
/// This contains the payload that Home Assistant uses to read the values.
#[derive(Serialize, Debug, Default)]
pub struct StatusMessage {
    pub available: &'static str,

    /// CPU usage in %
    pub cpu_usage: Option<f32>,

    /// Memory usage in %
    pub memory_usage: Option<f32>,

    /// Temperatures in °C
    pub temperature: HashMap<String, f32>,

    /// Statistics for the network interfaces
    pub network: HashMap<String, NetworkStatus>,
}

/// Network status
#[derive(Serialize, Debug, Default)]
pub struct NetworkStatus {
    /// Net TX rate in KiB/s
    pub tx: f64,

    /// Net RX rate in KiB/s
    pub rx: f64,
}

impl fmt::Display for StatusMessage {
    /// Formats the message to a JSON string
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let Ok(str) = serde_json::to_string(&self) else {
            return Err(fmt::Error);
        };
        write!(f, "{str}")
    }
}

impl StatusMessage {
    /// Produces the status when we're disconnecting
    pub fn off() -> StatusMessage {
        StatusMessage {
            available: "OFF",
            ..Default::default()
        }
    }
}
