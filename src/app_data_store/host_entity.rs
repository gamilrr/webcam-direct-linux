use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum ConnectionType {
    WIFI(String),
    WLAN(String),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct HostInfo {
    pub id: String,
    pub name: String,
    pub connection_type: ConnectionType,
}
