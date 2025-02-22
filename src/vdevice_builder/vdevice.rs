use crate::error::Result;
use anyhow::anyhow;
use log::{error, info};
use tokio::task;
use v4l2loopback::{add_device, delete_device, DeviceConfig};

use super::system_utils::{pnp_plug, pnp_unplug};

#[derive(Debug, Clone)]
pub struct VDevice {
    pub name: String,
    pub device_num: u32,
}

impl VDevice {
    pub async fn new(name: String) -> Result<Self> {
        let config = DeviceConfig {
            min_width: 100,
            max_width: 4000,
            min_height: 100,
            max_height: 4000,
            max_buffers: 9,
            max_openers: 1,
            label: name.clone(),
            ..Default::default()
        };

        info!("Adding virtual device with name {}", name);

        let device_num = match add_device(None, config)
            .map_err(|e| anyhow!("Failed to add device: {:?}", e))
        {
            Ok(num) => num,
            Err(e) => {
                // Handle the error from add_device
                error!("Error adding device: {:?}", e);
                return Err(e);
            }
        };

        pnp_plug(format!("video{}", device_num)).await?;

        Ok(Self { name, device_num })
    }
}

impl Drop for VDevice {
    fn drop(&mut self) {
        if let Err(e) =
            pnp_unplug(format!("video{}", format!("video{}", self.device_num)))
        {
            error!(
                "Failed to trigger unplug event for virtual device {} with error: {:?}",
                self.name, e
            );
        }
        if let Err(e) = delete_device(self.device_num) {
            error!(
                "Failed to remove virtual device {} with error: {:?}",
                self.name, e
            );
        }
    }
}
