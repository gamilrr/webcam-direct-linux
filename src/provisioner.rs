use crate::error::Result;
use std::{collections::BTreeSet, future};

use bluer::{
    adv::Advertisement,
    gatt::{
        local::{
            characteristic_control, Application, Characteristic,
            CharacteristicControlEvent, CharacteristicRead,
            CharacteristicReadRequest, CharacteristicWrite,
            CharacteristicWriteMethod, Service,
        },
        CharacteristicReader,
    },
    Adapter, Uuid,
};
use futures::{channel::oneshot, pin_mut, FutureExt, StreamExt};
use log::info;
use serde_json::json;
use tokio::io::AsyncReadExt;

use crate::{
    app_data_store::{mobile_entity::MobileInfo, AppStore},
    gatt_const::{
        PROV_CHAR_HOST_INFO_UUID, PROV_CHAR_MOBILE_INFO_UUID,
        PROV_SERV_HOST_UUID,
    },
};

pub struct Provisioner {
    ble_adapter: Adapter,
    app_store: AppStore,
    _tx_drop: Option<oneshot::Sender<()>>,
}

impl Provisioner {
    pub fn new(ble_adapter: Adapter, app_store: AppStore) -> Self {
        Self { ble_adapter, app_store, _tx_drop: None }
    }

    pub async fn start_provisioning(&mut self) -> Result<()> {
        let mut services = BTreeSet::<Uuid>::new();
        services.insert(PROV_SERV_HOST_UUID);

        let le_advertisement = Advertisement {
            service_uuids: services,
            discoverable: Some(true),
            local_name: Some(self.app_store.get_host_name()),
            ..Default::default()
        };

        let (tx, mut rx) = oneshot::channel();
        self._tx_drop = Some(tx);
        let (mobile_char_control, mobile_char_handle) =
            characteristic_control();

        let host_id = self.app_store.get_host_id();
        let app = Application {
            services: vec![Service {
                uuid: PROV_SERV_HOST_UUID,
                primary: true,
                characteristics: vec![
                    Characteristic {
                        uuid: PROV_CHAR_HOST_INFO_UUID,
                        read: Some(CharacteristicRead {
                            read: true,
                            fun: Box::new(
                                move |req: CharacteristicReadRequest| {
                                    info!(
                                        "Read request with MTU {} from {}",
                                        req.mtu,
                                        req.device_address.to_string()
                                    );
                                    let host_id = host_id.clone();
                                    async move {
                                        info!("Sending host info");
                                        let host_info = json!({
                                            "i": host_id,
                                            "c": "w",
                                        });

                                        Ok(host_info.to_string().into_bytes())
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
                            write_without_response: true,
                            method: CharacteristicWriteMethod::Io,
                            ..Default::default()
                        }),
                        control_handle: mobile_char_handle,
                        ..Default::default()
                    },
                ],
                ..Default::default()
            }],
            ..Default::default()
        };

        let adapter = self.ble_adapter.clone();
        let app_store = self.app_store.clone();
        tokio::spawn(async move {
            let _advertisement_handle =
                Some(adapter.advertise(le_advertisement.clone()).await);
            let _adapter_handle =
                adapter.serve_gatt_application(app).await.unwrap();

            let mut read_buf = Vec::new();
            let mut reader_opt: Option<CharacteristicReader> = None;
            pin_mut!(mobile_char_control);

            loop {
                tokio::select! {
                    _ = &mut rx => {break}

                    evt = mobile_char_control.next() => {
                        match evt {
                            Some(CharacteristicControlEvent::Write(req)) => {
                                info!("Accepting write event with MTU {} from {}", req.mtu(), req.device_address());
                                reader_opt = Some(req.accept().unwrap());
                            },
                            Some(_) => print!("Another event"),
                            None => print!("Another None event")
                        }
                    }
                    read_res = async {
                        match &mut reader_opt {
                            Some(reader) => {
                                info!("mtu in reader: {}, device address: {}", reader.mtu(), reader.device_address());
                                read_buf = vec![0; reader.mtu()];
                                let read = reader.read(&mut read_buf).await;
                                let null_index = read_buf.iter().position(|&x| x == 0).unwrap_or(read_buf.len());
                                let read_buf_json = read_buf[..null_index].to_vec();
                                let mobile_info_json = String::from_utf8(read_buf_json).unwrap();
                                info!("Mobile info: {:?}", mobile_info_json);
                                let mobile_info: MobileInfo = serde_json::from_str(&mobile_info_json).unwrap();
                                app_store.add_mobile(mobile_info).await.unwrap();
                                read
                            },
                            None => future::pending().await,
                        }
                    } => {
                        match read_res {
                            Ok(0) => {
                                info!("Write stream ended");
                                reader_opt = None;
                            }
                            Ok(n) => {
                                info!("Write request with {} bytes", n);
                            }
                            Err(err) => {
                                info!("Write stream error: {}", &err);
                                reader_opt = None;
                            }
                        }
                    }
                }
            }
            info!("End of advertise thread");
        });

        Ok(())
    }

    pub fn stop_provisioning(&mut self) {
        self._tx_drop.take();
    }
}
