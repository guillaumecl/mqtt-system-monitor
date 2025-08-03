use serde::Serialize;
use std::fmt;

#[derive(Serialize, Debug)]
pub struct StatusMessage {
    pub cpu_usage: f32,
    pub cpu_temp: Option<f32>,
    pub disk_usage: Option<f32>,
    pub net_tx: Option<f64>,
    pub net_rx: Option<f64>,
}

impl fmt::Display for StatusMessage {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let Ok(str) = serde_json::to_string(&self) else {
            return Err(fmt::Error);
        };
        write!(f, "{str}")
    }
}
