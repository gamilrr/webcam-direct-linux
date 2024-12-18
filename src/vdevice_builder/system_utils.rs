use crate::error::Result;
use anyhow::anyhow;
use log::{error, info};
use std::io::Write;
use std::path::Path;
use tokio::fs::OpenOptions;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;

//utility function to trigger a plug event
pub async fn pnp_plug(device: String) -> Result<()> {
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

pub fn pnp_unplug(device: String) -> Result<()> {
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

//utility function to load a kernel module
pub async fn load_kmodule(
    module_name: &str, args: Option<&[&str]>,
) -> Result<()> {
    let mut cmd = Command::new("modprobe"); //.arg(module_name).status().await?
                                            //add argument if any
    let cmd = match args {
        Some(args) => cmd.arg(module_name).args(args),
        None => cmd.arg(module_name),
    };

    let cmd = cmd.status().await?;

    if cmd.success() {
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
pub fn unload_kmodule(module_name: &str) -> Result<()> {
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
