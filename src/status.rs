use serde::Serialize;
use std::fmt;

/// Message sent to the MQTT broker which later forwards it to Home Assistant
///
/// This contains the payload that Home Assistant uses to read the values.
#[derive(Serialize, Debug, Default)]
pub struct StatusMessage {
    pub available: &'static str,

    /// CPU usage in %
    pub cpu_usage: Option<f32>,

    /// CPU temperature in °C
    pub cpu_temp: Option<f32>,

    /// Memory usage in %
    pub memory_usage: Option<f32>,

    /// Net TX rate in KiB/s
    pub net_tx: Option<f64>,

    /// Net RX rate in KiB/s
    pub net_rx: Option<f64>,
}

impl StatusMessage {
    pub fn off() -> StatusMessage {
        StatusMessage {
            available: "OFF",
            ..Default::default()
        }
    }
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
