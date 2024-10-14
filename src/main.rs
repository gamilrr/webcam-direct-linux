mod access_point_ctl;
mod app_data;
mod app_data_store;
mod error;
mod gatt_const;
mod provisioner;
mod sdp_exchanger;

use std::io::{self, Read};

use access_point_ctl::{
    dhcp_server::{DhcpIpRange, DnsmasqProc},
    iw_link::{wdev_drv, IwLink},
    process_hdl::ProcessHdl,
    wifi_manager::{
        FileHdl, HostapdProc, WifiCredentials, WifiManager, WpaCtl,
    },
    AccessPointCtl, ApController,
};
use app_data::HostInfo;
use app_data_store::host_entity::ConnectionType;
use error::Result;

use tokio::io::AsyncBufReadExt;

use crate::app_data_store::AppStore;
use log::info;
use provisioner::Provisioner;
use sdp_exchanger::SdpExchanger;

fn create_ap_controller() -> Result<impl AccessPointCtl> {
    //init the wireless interface handler---------
    let link = IwLink::with_driver(wdev_drv::Nl80211Driver);

    //init the dhcp server---------
    let dhcp_server_proc = DnsmasqProc::new(ProcessHdl::handler());

    //init the wifi manager---------

    //wifi manager process
    let hostapd_proc = HostapdProc::new(
        FileHdl::from_path("/tmp/hostapd.conf"),
        ProcessHdl::handler(),
    );
    let if_name = "wcdirect0";
    let wifi_manager =
        WifiManager::new(hostapd_proc, WpaCtl::new("/tmp/hostapd", if_name));

    //init Access Point manager------
    let mut ap_controller =
        ApController::new(link, dhcp_server_proc, wifi_manager);

    //init network interface
    let dhcp_ips = DhcpIpRange::new("193.168.3.5", "193.168.3.150")?;
    let router_ip = dhcp_ips.get_router_ip();

    //init wifi credentials
    let creds = WifiCredentials {
        ssid: String::from("WebcamDirect"),
        password: String::from("12345678"),
    };

    ap_controller.configure(creds, &router_ip)?;

    ap_controller.start_dhcp_server(dhcp_ips)?;

    Ok(ap_controller)
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    info!("Starting webcam direct");

    let mut ap_controller = create_ap_controller()?;

    ap_controller.start_wifi()?;

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

    info!("Service ready. Press enter to quit.");
    let stdin = tokio::io::BufReader::new(tokio::io::stdin());
    let mut lines = stdin.lines();
    let _ = lines.next_line().await;

    provisioner.stop_provisioning();
    //sdp_exchanger.stop().await?;

    info!("webcam direct stopped stopped");

    info!("Service ready. Press enter to quit.");
    let stdin = tokio::io::BufReader::new(tokio::io::stdin());
    let mut lines = stdin.lines();
    let _ = lines.next_line().await;

    Ok(())
}
