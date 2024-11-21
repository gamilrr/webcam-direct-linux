use crate::app_data::MobileSchema;

#[derive(Debug, Clone)]
pub struct VCamDevice {
    mobile: MobileSchema,
}

impl VCamDevice {
    pub fn new(mobile: MobileSchema) -> Self {
        Self { mobile }
    }
}
