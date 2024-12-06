use crate::app_data::MobileSchema;
use crate::ble::VDeviceBuilderOps;
use crate::error::Result;
use anyhow::anyhow;
use log::error;
use std::path::Path;
use tokio::fs::File;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use v4l2loopback::DeviceConfig;

//utility function to check if a kernel module is loaded
async fn is_kmodule_loaded<P>(
    reg_module_file: P, module_name: &str,
) -> Result<bool>
where
    P: AsRef<Path>,
{
    let file = File::open(&reg_module_file).await?;
    let reader = BufReader::new(file);
    let mut lines = reader.lines();

    while let Ok(Some(line)) = lines.next_line().await {
        if line.starts_with(module_name) {
            return Ok(true);
        }
    }

    Ok(false)
}

//utility function to load a kernel module
async fn load_kmodule(module_name: &str) -> Result<()> {
    let status = Command::new("modprobe").arg(module_name).status().await?;

    if status.success() {
        Ok(())
    } else {
        error!(
            "Failed to load module: {}, please install the module",
            module_name
        );
        Err(anyhow!("Failed to load module"))
    }
}

//utility function to unload a kernel module
//turn into aync when async_drop is available
fn unload_kmodule(module_name: &str) -> Result<()> {
    //use std::process::Command to unload the module synchronously
    let status = std::process::Command::new("modprobe")
        .arg("-r")
        .arg(module_name)
        .status()?;

    if status.success() {
        Ok(())
    } else {
        error!("Failed to unload module: {}", module_name);
        Err(anyhow!("Failed to unload module"))
    }
}

pub struct VDeviceBuilder {
    //flags to set up the system at beginning and tear down at the end
    is_v4l2loopback_loaded: bool,
    is_videodev_loaded: bool,
}

impl VDeviceBuilder {
    pub async fn new() -> Result<Self> {
        let mut is_v4l2loopback_loaded = false;
        let mut is_videodev_loaded = false;
        //check for videodev module
        if !is_kmodule_loaded("/proc/modules", "videodev").await? {
            is_v4l2loopback_loaded = true;
            load_kmodule("videodev").await?;
        }

        //check for v4l2loopback module
        if !is_kmodule_loaded("/proc/modules", "v4l2loopback").await? {
            is_videodev_loaded = true;
            load_kmodule("v4l2loopback").await?;
        }

        Ok(Self { is_v4l2loopback_loaded, is_videodev_loaded })
    }
}

impl VDeviceBuilderOps for VDeviceBuilder {
    fn create_from(&self, mobile: MobileSchema) -> Result<Vec<VDevice>> {
        Ok(vec![VDevice::new(mobile)])
    }
}

impl Drop for VDeviceBuilder {
    fn drop(&mut self) {
        //unload the modules
        if self.is_v4l2loopback_loaded
            && unload_kmodule("v4l2loopback").is_err()
        {
            error!("Failed to unload v4l2loopback module");
        }

        if self.is_videodev_loaded && unload_kmodule("videodev").is_err() {
            error!("Failed to unload videodev module");
        }
    }
}

#[derive(Debug, Clone)]
pub struct VDevice {
    mobile: MobileSchema,
    device_num: u32,
}

impl VDevice {
    pub fn new(mobile: MobileSchema) -> Self {
        Self { mobile, device_num: 0 }
    }
}
