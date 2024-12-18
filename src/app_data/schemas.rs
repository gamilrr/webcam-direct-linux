//! This module defines the schemas for mobile and host data structures used in the application.
//! It includes the necessary types and implementations for serialization and deserialization,
//! as well as the required traits for database schema handling.

use serde::{Deserialize, Serialize};

use super::kv_db::SchemaType;

/// Type alias for Mobile ID, represented as a String.
pub type MobileId = String;

/// Represents the properties of a video, including resolution and frames per second.
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct VideoProp {
    resolution: (u32, u32),
    fps: u32,
}

/// Represents information about a camera, including its name and supported video formats.
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct CameraInfo {
    pub name: String,
    pub format: Vec<VideoProp>,
}

/// Represents the schema for mobile devices, including ID, name, and associated cameras.
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct MobileSchema {
    pub id: MobileId,
    pub name: String,
    pub cameras: Vec<CameraInfo>,
}

impl SchemaType for MobileSchema {
    const KEYSPACE_NAME: &'static str = "registered_mobiles";
}

/// Type alias for Host ID, represented as a String.
pub type HostId = String;

/// Enum representing the type of connection for a host.
#[derive(Debug, Serialize, Deserialize, Clone, Default, PartialEq, Eq)]
pub enum ConnectionType {
    #[default]
    WLAN,
    AP,
}

/// Represents the schema for host devices, including ID, name, connection type, and registered mobiles.
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct HostSchema {
    pub id: HostId,
    pub name: String,
    pub connection_type: ConnectionType,
    pub registered_mobiles: Vec<MobileId>,
}

impl SchemaType for HostSchema {
    const KEYSPACE_NAME: &'static str = "host_information";
}
