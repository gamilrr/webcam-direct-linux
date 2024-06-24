use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct VideoProp {
    resolution: (u32, u32),
    fps: u32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CameraInfo {
    name: String,
    video_format: Vec<VideoProp>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MobileInfo {
    pub id: String,
    pub name: String,
    pub cameras: Vec<CameraInfo>,
}
