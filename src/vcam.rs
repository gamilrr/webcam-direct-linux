#[derive(Debug, Clone)]
pub struct VCamDevice {
    path: String,
}

impl VCamDevice {
    pub fn new(path: String) -> Self {
        Self { path }
    }
}
