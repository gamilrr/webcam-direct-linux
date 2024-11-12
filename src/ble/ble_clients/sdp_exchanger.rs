use crate::ble::ble_server::ServerConn;
use crate::gatt_const::{
    SDP_NOTIFY_CHAR_UUID, SDP_WRITE_CHAR_UUID, WEBCAM_PNP_WRITE_CHAR_UUID,
};
use bluer::adv::Advertisement;
use bluer::gatt::local::{
    Application, Characteristic, CharacteristicNotify,
    CharacteristicNotifyMethod, CharacteristicRead, CharacteristicWrite,
    CharacteristicWriteMethod, Service,
};
use bluer::Uuid;
use bluer::{
    adv::AdvertisementHandle, gatt::local::ApplicationHandle, Adapter,
};
use futures::FutureExt;
use log::{error, info};
use tokio::sync::oneshot;

use crate::error::Result;

pub struct SdpExchangerClient {
    _tx_drop: oneshot::Sender<()>,
}

impl SdpExchangerClient {
    pub fn new(
        ble_adapter: Adapter, server_conn: ServerConn, host_name: String,
        host_id: String,
    ) -> Self {
        info!("Starting SdpExchangerClient");

        let (tx, rx) = oneshot::channel();
        tokio::spawn(async move {
            if let Ok((_adv_handler, _app_handler)) =
                sdp_exchanger(ble_adapter, server_conn, host_name, host_id)
                    .await
            {
                info!("SdpExchanger started");
                let _ = rx.await;
                info!("SdpExchanger stopped");
            } else {
                error!("SdpExchangerClient failed");
            }
        });

        Self { _tx_drop: tx }
    }
}

async fn sdp_exchanger(
    ble_adapter: Adapter, server_conn: ServerConn, host_name: String,
    host_id: String,
) -> Result<(AdvertisementHandle, ApplicationHandle)> {
    info!(
        "Advertising Sdp Exchanger on Bluetooth adapter {} with address {}",
        ble_adapter.name(),
        ble_adapter.address().await?
    );
    let host_id = Uuid::parse_str(&host_id)?;
    let le_advertisement = Advertisement {
        service_uuids: vec![host_id].into_iter().collect(),
        discoverable: Some(true),
        local_name: Some(host_name),
        ..Default::default()
    };

    let adv_handle = ble_adapter.advertise(le_advertisement).await?;

    info!("Serving GATT service on Bluetooth adapter {}", ble_adapter.name());

    let reader_server_conn = server_conn.clone();
    let writer_server_conn = server_conn.clone();
    let app = Application {
        services: vec![Service {
            uuid: host_id,
            primary: true,
            characteristics: vec![
                Characteristic {
                    uuid: SDP_WRITE_CHAR_UUID,
                    read: Some(CharacteristicRead {
                        read: true,
                        fun: Box::new(move |req| {
                            async move {
                                info!("SDP Write Characteristic read request");
                                Ok(vec![])
                            }
                            .boxed()
                        }),
                        ..Default::default()
                    }),
                    ..Default::default()
                },
                Characteristic {
                    uuid: SDP_NOTIFY_CHAR_UUID,
                    notify: Some(CharacteristicNotify {
                        notify: true,
                        method: CharacteristicNotifyMethod::Fun(Box::new(
                            move |mut notifier| {
                                async move {
                                    tokio::spawn(async move{
                                        info!(
                                            "Notification session start with confirming={:?}",
                                            notifier.confirming()
                                            );
                                    });
                                }.boxed()
                            },
                        )),
                        ..Default::default()
                    }),
                    ..Default::default()
                },
                Characteristic {
                    uuid: WEBCAM_PNP_WRITE_CHAR_UUID,
                    write: Some(CharacteristicWrite {
                        write: true,
                        write_without_response: false,
                        method: CharacteristicWriteMethod::Fun(Box::new(
                            move |new_value, req| {
                                async move {
                                    info!(
                                        "SDP Read Received new value: {:?}",
                                        new_value
                                    );
                                    Ok(())
                                }
                                .boxed()
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

    let app_handle = ble_adapter.serve_gatt_application(app).await?;

    Ok((adv_handle, app_handle))
}
