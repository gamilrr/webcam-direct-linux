//! Serves a Bluetooth GATT application using the IO programming model.
use bluer::{
    adv::{Advertisement, AdvertisementHandle},
    gatt::{
        local::{
            characteristic_control, service_control, Application,
            ApplicationHandle, Characteristic, CharacteristicControlEvent,
            CharacteristicNotify, CharacteristicNotifyMethod,
            CharacteristicRead, CharacteristicWrite, CharacteristicWriteMethod,
            Service,
        },
        CharacteristicReader, CharacteristicWriter,
    },
    Adapter,
};
use futures::FutureExt;
use log::info;
use tokio::{
    io::{AsyncBufReadExt, BufReader},
    sync::{mpsc, oneshot},
};

use crate::{
    ble::{
        ble_cmd_api::{self, BleApi},
        ble_server::ServerConn,
    },
    gatt_const::{
        PROV_CHAR_HOST_INFO_UUID, PROV_CHAR_MOBILE_INFO_UUID,
        PROV_SERV_HOST_UUID,
    },
};

pub struct ProvisionerClient {
    _tx_drop: oneshot::Sender<()>,
}

impl ProvisionerClient {
    pub fn new(ble_adapter: Adapter, server_conn: ServerConn) -> Self {
        let (tx, rx) = oneshot::channel();

        tokio::spawn(async move {
            if let Ok((_adv_handle, _app_handle)) =
                provisioner(ble_adapter, server_conn).await
            {
                info!("Provisioner started");

                let _ = rx.await;
            } else {
                info!("Provisioner failed to start");
            }
        });

        Self { _tx_drop: tx }
    }
}

pub async fn provisioner(
    adapter: Adapter, server_conn: ServerConn,
) -> bluer::Result<(AdvertisementHandle, ApplicationHandle)> {
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

    let reader_server_conn = server_conn.clone();
    let writer_server_conn = server_conn.clone();
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
                                &req, req.device_address
                            );

                            //prepare the cmd to send to the server
                            let (tx, rx) = oneshot::channel();

                            let cmd = BleApi::HostInfo(ble_cmd_api::BleQuery {
                                addr: req.device_address.to_string(),
                                max_buffer_len: 20,
                                resp: tx,
                            });

                            let reader_server_conn = reader_server_conn.clone();

                            async move {
                                if let Err(e) =
                                    reader_server_conn.send(cmd).await
                                {
                                    info!(
                                        "Error sending host info request: {:?}",
                                        e
                                    );
                                }

                                let _ = rx.await;

                                Ok(vec![])
                            }
                            .boxed()
                        }),
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
                                info!(
                                    "Write request {:?} from {}",
                                    &new_value, req.device_address
                                );

                                //prepare the cmd to send to the server
                                let (tx, rx) = oneshot::channel();

                                let cmd = BleApi::RegisterMobile(
                                    ble_cmd_api::BleCmd {
                                        addr: req.device_address.to_string(),
                                        payload: ble_cmd_api::BleBuffer {
                                            remain_len: new_value.len(),
                                            payload: new_value,
                                        },
                                        resp: tx,
                                    },
                                );

                                let writer_server_conn =
                                    writer_server_conn.clone();
                                async move {
                                    if let Err(e) = writer_server_conn.send(cmd).await {
                                        info!("Error sending mobile registration request: {:?}", e);
                                    }

                                    let _ = rx.await;

                                    Ok(())
                                }.boxed()
                            },
                        )),
                        ..Default::default()
                    }),
                    ..Default::default()
                },
            ],
            ..Default::default()
        }],
        ..Default::default()
    };
    let app_handle = adapter.serve_gatt_application(app).await?;

    Ok((adv_handle, app_handle))
}
