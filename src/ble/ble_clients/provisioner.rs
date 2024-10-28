//! Serves a Bluetooth GATT application using the IO programming model.

use bluer::{
    adv::Advertisement,
    gatt::{
        local::{
            characteristic_control, service_control, Application, Characteristic, CharacteristicControlEvent, CharacteristicNotify, CharacteristicNotifyMethod, CharacteristicRead, CharacteristicWrite, CharacteristicWriteMethod, Service
        }, CharacteristicReader, CharacteristicWriter
    },
};
use futures::{future, pin_mut, FutureExt, StreamExt};
use log::info;
use std::{collections::BTreeMap, sync::{Arc, Mutex}, time::Duration};
use tokio::{
    io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader},
    time::{interval, sleep},
};

use crate::gatt_const::{PROV_CHAR_HOST_INFO_UUID, PROV_CHAR_MOBILE_INFO_UUID, PROV_SERV_HOST_UUID};

pub async fn provisioner() -> bluer::Result<()> {
    let session = bluer::Session::new().await?;
    let adapter = session.default_adapter().await?;
    adapter.set_powered(true).await?;

    info!(
        "Advertising on Bluetooth adapter {} with address {}",
        adapter.name(),
        adapter.address().await?
    );
    let le_advertisement = Advertisement {
        service_uuids: vec![PROV_SERV_HOST_UUID].into_iter().collect(),
        discoverable: Some(true),
        local_name: Some("gatt_server".to_string()),
        ..Default::default()
    };
    let adv_handle = adapter.advertise(le_advertisement).await?;

    info!("Serving GATT service on Bluetooth adapter {}", adapter.name());
    let counter = Arc::new(Mutex::new(0));
    let app = Application {
        services: vec![Service {
            uuid: PROV_SERV_HOST_UUID,
            primary: true,
            characteristics: vec![
                Characteristic {
                    uuid: PROV_CHAR_HOST_INFO_UUID,
                    read: Some(CharacteristicRead {
                        read: true,
                        fun: Box::new(move |req| {
                                info!(
                                    "Read request {:?} from {}",
                                    &req,
                                    req.device_address
                                );
                                async move {
                                    Ok(vec![0x01, 0x02, 0x03])
                                }
                                .boxed()
                            },
                        ),
                        ..Default::default()
                    }),
                  ..Default::default()    
                },
                Characteristic {
                    uuid: PROV_CHAR_MOBILE_INFO_UUID,
                    write: Some(CharacteristicWrite {
                        write: true,
                        write_without_response: false,
                        method: CharacteristicWriteMethod::Fun(Box::new(
                            move |new_value, req| {

                                let counter_int = { 
                                    let mut counter = counter.lock().unwrap();
                                    *counter = *counter + 1;
                                    *counter
                                };

                                async move {

                                info!("Write request {:?} with value {:x?} size {} from {} {counter_int}", &req, &new_value, new_value.len(), req.device_address);
                                
                                Ok(())
                            }
                            .boxed()
                            },
                        )),
                        ..Default::default()
                    }),
                    ..Default::default()
            }],
            ..Default::default()
        }],
        ..Default::default()
    };
    let app_handle = adapter.serve_gatt_application(app).await?;

    info!("Service ready. Press enter to quit.");
    let stdin = BufReader::new(tokio::io::stdin());
    let mut lines = stdin.lines();
    let _ = lines.next_line().await;

    info!("Removing service and advertisement");
    drop(app_handle);
    drop(adv_handle);
    sleep(Duration::from_secs(1)).await;

    Ok(())
}
