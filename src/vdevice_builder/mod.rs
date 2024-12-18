use crate::app_data::MobileSchema;
use crate::ble::{VDeviceBuilderOps, VDeviceMap};
use crate::error::Result;
use async_trait::async_trait;
use log::{error, info};
use std::path::{Path, PathBuf};
use system_utils::{load_kmodule, unload_kmodule};
use tokio::fs::File;
use tokio::io::{AsyncBufReadExt, BufReader};
mod system_utils;
mod vdevice;

pub use vdevice::VDevice;

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
            load_kmodule("videodev", None).await?;
        }

        //check for v4l2loopback module
        if !is_kmodule_loaded("/proc/modules", "v4l2loopback").await? {
            is_videodev_loaded = true;
            load_kmodule("v4l2loopback", Some(&["exclusive_caps=1"])).await?;
        }

        Ok(Self { is_v4l2loopback_loaded, is_videodev_loaded })
    }
}

#[async_trait]
impl VDeviceBuilderOps for VDeviceBuilder {
    async fn create_from(&self, mobile: MobileSchema) -> Result<VDeviceMap> {
        let mut device_map = VDeviceMap::new();

        for camera in &mobile.cameras {
            if let Ok(vdevice) =
                VDevice::new(format!("{}-{}", &mobile.name, &camera.name)).await
            {
                let path =
                    PathBuf::from(format!("/dev/video{}", vdevice.device_num));

                info!(
                    "Created virtual device with name {} in path {:?}",
                    &vdevice.name, &path
                );

                device_map.insert(path, vdevice);
            }
        }

        Ok(device_map)
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
