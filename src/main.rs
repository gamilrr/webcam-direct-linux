mod access_point_ctl;
mod app_data;
mod app_data_store;
mod error;
mod gatt_const;
mod ble_ctl;
mod provisioner;
mod sdp_exchanger;

use std::io::{self, Read};

use access_point_ctl::{
    dhcp_server::{DhcpIpRange, DnsmasqProc},
    iw_link::{wdev_drv, IwLink, IwLinkHandler},
    process_hdl::ProcessHdl,
    wifi_manager::{
        FileHdl, HostapdProc, WifiCredentials, WifiManager, WpaCtl,
    },
    AccessPointCtl, ApController,
};
use app_data::HostInfo;
use app_data_store::host_entity::ConnectionType;
use error::Result;

use ble_ctl::ble_events::device_props::device_props;
use tokio::io::AsyncBufReadExt;
use webrtc::util::vnet::router;

use crate::app_data_store::AppStore;
use log::info;
use provisioner::Provisioner;
use sdp_exchanger::SdpExchanger;

fn setup_access_point() -> Result<impl AccessPointCtl> {
    let if_name = "wcdirect0";

    //init the wireless interface handler---------
    let link = IwLink::new(wdev_drv::Nl80211Driver, if_name)?;

    //init the dhcp server---------
    let dhcp_server_proc = DnsmasqProc::new(ProcessHdl::handler());

    //wifi manager process
    let hostapd_proc = HostapdProc::new(
        FileHdl::from_path("/tmp/hostapd.conf"),
        ProcessHdl::handler(),
    );

    let wpactrl = WpaCtl::new("/tmp/hostapd", if_name);

    let creds = WifiCredentials {
        ssid: "WebcamDirect".to_string(),
        password: "12345678".to_string(),
    };

    let wifi_manager = WifiManager::new(&creds, hostapd_proc, wpactrl)?;

    //init Access Point manager------
    Ok(ApController::new(link, dhcp_server_proc, wifi_manager))
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    info!("Starting webcam direct");

    // let mut ap_controller = setup_access_point()?;

    //ap_controller
    //    .start_dhcp_server(DhcpIpRange::new("193.168.3.5", "193.168.3.150")?)?;

    //ap_controller.start_wifi()?;

    let session = bluer::Session::new().await?;

    let adapter = session.default_adapter().await?;

    adapter.set_powered(true).await?;

    let app_store = AppStore::new("webcam-direct-config.json").await;

    info!("Webcam direct started");
    // let mut sdp_exchanger =
    //     SdpExchanger::new(adapter.clone(), app_store.clone());

    let mut provisioner = Provisioner::new(adapter.clone(), app_store.clone());

    provisioner.start_provisioning().await?;

    //sdp_exchanger.start().await?;

    device_props(adapter.clone()).await?;

    info!("Service ready. Press enter to quit.");
    let stdin = tokio::io::BufReader::new(tokio::io::stdin());
    let mut lines = stdin.lines();
    let _ = lines.next_line().await;

    provisioner.stop_provisioning();
    //sdp_exchanger.stop().await?;

    info!("webcam direct stopped stopped");

    Ok(())
}
