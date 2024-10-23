//! Discover Bluetooth devices and list them.
use crate::error::Result;
use bluer::{
    Adapter, AdapterEvent, Address, DeviceEvent, DiscoveryFilter,
    DiscoveryTransport,
};
use futures::{pin_mut, stream::SelectAll, StreamExt};
use log::info;
use std::{collections::HashSet, env};

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

pub async fn device_props(adapter: Adapter) -> Result<()> {
    //let filter_addr: HashSet<_> = env::args().filter_map(|arg| arg.parse::<Address>().ok()).collect();

    info!(
        "Using discovery filter:\n{:#?}\n\n",
        adapter.discovery_filter().await
    );

    let device_events = adapter.events().await?;
    pin_mut!(device_events);

    let mut all_change_events = SelectAll::new();

    loop {
        tokio::select! {
            Some(device_event) = device_events.next() => {
                match device_event {
                    AdapterEvent::DeviceAdded(addr) => {
                       // if !addr.to_string().contains("40:CA:63:45:B9:4A") {
                       //     continue;
                       // } else {
                       //     info!("Device added: {addr}");
                       // }

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
                info!("this");
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
