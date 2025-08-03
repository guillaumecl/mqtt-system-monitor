use serde::Serialize;

#[derive(Serialize, Debug)]
pub struct StatusMessage {
    pub cpu_usage: f32,
    pub cpu_temp: Option<f32>,
    pub disk_usage: Option<f32>,
    pub net_tx: Option<f64>,
    pub net_rx: Option<f64>,
}

impl StatusMessage {
    pub fn to_string(&self) -> String {
        serde_json::to_string(&self).unwrap_or_default()
    }
}
