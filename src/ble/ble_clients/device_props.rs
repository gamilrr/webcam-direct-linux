//! Discover Bluetooth devices and list them.
use crate::{ble::ble_server::ServerConn, error::Result};
use bluer::{
    Adapter, AdapterEvent, Address, DeviceEvent, DiscoveryFilter,
    DiscoveryTransport,
};
use futures::{pin_mut, stream::SelectAll, StreamExt};
use log::info;
use std::{collections::HashSet, env};
use tokio::sync::oneshot;

async fn query_device(adapter: &Adapter, addr: Address) -> bluer::Result<()> {
    let device = adapter.device(addr)?;
    info!("    Address type:       {}", device.address_type().await?);
    info!("    Name:               {:?}", device.name().await?);
    info!("    Icon:               {:?}", device.icon().await?);
    info!("    Class:              {:?}", device.class().await?);
    info!(
        "    UUIDs:              {:?}",
        device.uuids().await?.unwrap_or_default()
    );
    info!("    Paired:             {:?}", device.is_paired().await?);
    info!("    Connected:          {:?}", device.is_connected().await?);
    info!("    Trusted:            {:?}", device.is_trusted().await?);
    info!("    Modalias:           {:?}", device.modalias().await?);
    info!("    RSSI:               {:?}", device.rssi().await?);
    info!("    TX power:           {:?}", device.tx_power().await?);
    info!("    Manufacturer data:  {:?}", device.manufacturer_data().await?);
    info!("    Service data:       {:?}", device.service_data().await?);
    Ok(())
}

async fn query_all_device_properties(
    adapter: &Adapter, addr: Address,
) -> Result<()> {
    let device = adapter.device(addr)?;
    let props = device.all_properties().await?;
    for prop in props {
        info!("    {:?}", &prop);
    }
    Ok(())
}

pub struct MobileBlePropClient {
    _tx_drop: oneshot::Sender<()>,
}

impl MobileBlePropClient {
    pub fn new(ble_adapter: Adapter, server_conn: ServerConn) -> Self {
        let (tx, rx) = oneshot::channel();

        tokio::spawn(async move {
            if let Ok(()) = device_props(ble_adapter, server_conn).await {
                info!("MobileBlePropClient started");

                let _ = rx.await;

                info!("MobileBlePropClient stopped");
            } else {
                info!("MobileBlePropClient failed to start");
            }
        });

        Self { _tx_drop: tx }
    }
}

pub async fn device_props(
    adapter: Adapter, server_conn: ServerConn,
) -> Result<()> {
    //let filter_addr: HashSet<_> = env::args().filter_map(|arg| arg.parse::<Address>().ok()).collect();

    let device_events = adapter.events().await?;
    pin_mut!(device_events);

    let mut all_change_events = SelectAll::new();

    loop {
        tokio::select! {
            Some(device_event) = device_events.next() => {
                match device_event {
                    AdapterEvent::DeviceAdded(addr) => {
                        info!("Device added from query_device: {addr}");
                        let res = query_device(&adapter, addr).await;
                        if let Err(err) = res {
                            info!("    Error: {}", &err);
                        }

                        info!("Device added from query_all_device_properties: {addr}");
                        let res = query_all_device_properties(&adapter, addr).await;
                        if let Err(err) = res {
                            info!("    Error: {}", &err);
                        }

                        let device = adapter.device(addr)?;
                        let change_events = device.events().await?.map(move |evt| (addr, evt));
                        all_change_events.push(change_events);
                    }

                    AdapterEvent::DeviceRemoved(addr) => {
                        info!("Device removed: {addr}");


                    }
                    _ => (),
                }
            }

            Some((addr, DeviceEvent::PropertyChanged(property))) = all_change_events.next() => {
                info!("Property Device changed: {addr}");
                info!("    {property:?}");
            }
            else => break
        }
    }

    Ok(())
}
