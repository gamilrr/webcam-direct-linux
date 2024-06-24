mod app_data_store;
mod gatt_const;
mod provisioner;
mod sdp_exchanger;

use std::time::Duration;

use tokio::{io::AsyncBufReadExt, time::sleep};

use crate::app_data_store::AppStore;
use provisioner::Provisioner;
use sdp_exchanger::SdpExchanger;

#[tokio::main]
async fn main() -> Result<(), String> {
    let session = bluer::Session::new().await.map_err(|e| e.to_string())?;

    let adapter = session.default_adapter().await.map_err(|e| e.to_string())?;

    adapter.set_powered(true).await.map_err(|e| e.to_string())?;

    let app_store = AppStore::new("webcam-direct-config.json").await;

    println!("Webcam direct started");
    let mut sdp_exchanger = SdpExchanger::new(adapter.clone(), app_store.clone());
    let mut provisioner = Provisioner::new(adapter.clone(), app_store.clone());

    provisioner
        .start_provisioning()
        .await
        .map_err(|e| e.to_string())?;

    sdp_exchanger.start().await.map_err(|e| e.to_string())?;

    println!("Service ready. Press enter to quit.");
    let stdin = tokio::io::BufReader::new(tokio::io::stdin());
    let mut lines = stdin.lines();
    let _ = lines.next_line().await;

    provisioner.stop_provisioning();
    sdp_exchanger.stop().await.map_err(|e| e.to_string())?;

    println!("webcam direct stopped stopped");

    Ok(())
}
