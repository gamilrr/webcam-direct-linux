//! Discover Bluetooth devices and list them.
use crate::{
    ble::{
        ble_cmd_api::{BleApi, BleCmd},
        ble_server::ServerConn,
    },
    error::Result,
};
use bluer::{Adapter, AdapterEvent, DeviceEvent, DeviceProperty};
use futures::{pin_mut, stream::SelectAll, StreamExt};
use log::info;

use tokio::sync::oneshot;

pub struct MobilePropClient {
    _tx_drop: oneshot::Sender<()>,
}

impl MobilePropClient {
    pub fn new(ble_adapter: Adapter, server_conn: ServerConn) -> Self {
        info!("Starting MobilePropClient");

        let (tx, rx) = oneshot::channel();
        tokio::spawn(async move {
            if let Err(e) = device_props(ble_adapter, server_conn, rx).await {
                info!("MobilePropClient failed: {:?}", e);
            }
        });

        Self { _tx_drop: tx }
    }
}

async fn send_mobile_disconnected(
    server_conn: ServerConn, addr: String,
) -> Result<()> {
    let (tx, rx) = oneshot::channel();

    if let Ok(_) = server_conn
        .send(BleApi::MobileDisconnected(BleCmd {
            addr,
            payload: vec![],
            resp: tx,
        }))
        .await
    {
        if let Ok(_) = rx.await {
            info!("Mobile disconnected sent");
        } else {
            info!("Mobile disconnected failed");
        }
    }

    Ok(())
}

pub async fn device_props(
    adapter: Adapter, server_conn: ServerConn, mut _rx: oneshot::Receiver<()>,
) -> Result<()> {
    //let filter_addr: HashSet<_> = env::args().filter_map(|arg| arg.parse::<Address>().ok()).collect();

    let device_events = adapter.events().await?;
    pin_mut!(device_events);

    let mut all_change_events = SelectAll::new();

    info!("MobilePropClient started");
    loop {
        tokio::select! {
            Some(device_event) = device_events.next() => {
                match device_event {
                    AdapterEvent::DeviceAdded(addr) => {
                        info!("Device added from query_device: {addr}");

                        let device = adapter.device(addr)?;

                        //get only the events for connected property
                        let change_events = device.events().await?.filter_map(
                             move |evt| {
                                 let addr = addr.clone();
                                 Box::pin(async move {
                                     match evt {
                                         DeviceEvent::PropertyChanged(DeviceProperty::Connected(..)) => {
                                             Some((addr, evt))
                                         }
                                         _ => None,
                                     }
                                 })
                             },
                        );

                        all_change_events.push(change_events);
                    }

                    AdapterEvent::DeviceRemoved(addr) => {
                        info!("Device removed: {addr}");
                        if let Err(e)  = send_mobile_disconnected(server_conn.clone(), addr.to_string()).await{
                            info!("Failed to send mobile disconnected: {:?}", e);
                        }
                    }
                    _ => (),
                }
            }

            Some((addr, DeviceEvent::PropertyChanged(property))) = all_change_events.next() => {
                info!("Property Device changed: {addr}");
                info!("    {property:?}");
                if let DeviceProperty::Connected(false) = property {
                    if let Err(e)  = send_mobile_disconnected(server_conn.clone(), addr.to_string()).await{
                        info!("Failed to send mobile disconnected: {:?}", e);
                    }
                }
            }

            _ = &mut _rx => break,

        }
    }

    Ok(())
}
