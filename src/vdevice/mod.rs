use crate::app_data::MobileSchema;
use crate::ble::{VDeviceBuilderOps, VDeviceMap};
use crate::error::Result;
use anyhow::anyhow;
use log::{error, info};
use std::io::Write;
use std::path::{Path, PathBuf};
use tokio::fs::{File, OpenOptions};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::Command;
use tokio::task;
use v4l2loopback::{add_device, delete_device, DeviceConfig};

async fn pnp_plug(device: String) -> Result<()> {
    //let uevent_path = Path::new("/sys/class/video4linux/video0/uevent");
    let uevent_path =
        Path::new(&format!("/sys/class/video4linux/{}", device)).join("uevent");

    if uevent_path.exists() {
        let mut file = OpenOptions::new().write(true).open(uevent_path).await?;

        file.write_all(b"add").await?;

        info!("Successfully triggered unplug event.");
    } else {
        error!("uevent file does not exist.");
    }

    Ok(())
}

fn pnp_unplug(device: String) -> Result<()> {
    //let uevent_path = Path::new("/sys/class/video4linux/video0/uevent");
    let uevent_path =
        Path::new(&format!("/sys/class/video4linux/{}", device)).join("uevent");

    if uevent_path.exists() {
        let mut file =
            std::fs::OpenOptions::new().write(true).open(uevent_path)?;

        file.write_all(b"remove")?;

        info!("Successfully triggered unplug event.");
    } else {
        error!("uevent file does not exist.");
    }

    Ok(())
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
            max_buffers: u32::MAX,
            max_openers: u32::MAX,
            label: name.clone(),
            ..Default::default()
        };

        let device_num = task::spawn_blocking(move || {
            add_device(None, config)
                .map_err(|e| anyhow!("Failed to add device: {:?}", e))
        })
        .await??;

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
