use serde::Serialize;

#[derive(Serialize, Debug)]
pub struct StatusMessage {
    pub cpu_usage: f32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cpu_temp: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disk_usage: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub net_tx: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub net_rx: Option<f64>,
}

impl StatusMessage {
    pub fn to_string(&self) -> String {
        serde_json::to_string(&self).unwrap_or_default()
    }
}
