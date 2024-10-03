mod webcam_rtc;

use crate::error::Result;
use log::info;
use webcam_rtc::start_webrtc;

use std::{
    collections::{BTreeSet, HashMap},
    future,
    str::FromStr,
    sync::{Arc, Mutex},
    time::Duration,
};

use bluer::{
    adv::Advertisement,
    gatt::{
        local::{
            characteristic_control, Application, Characteristic,
            CharacteristicControlEvent, CharacteristicNotify,
            CharacteristicNotifyMethod, CharacteristicWrite,
            CharacteristicWriteMethod, Service,
        },
        CharacteristicReader, CharacteristicWriter,
    },
    Adapter, AdapterEvent, Address, Uuid,
};
use futures::{pin_mut, FutureExt, StreamExt};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    sync::{mpsc, oneshot},
    time::interval,
};
use v4l2loopback::{add_device, delete_device, query_device, DeviceConfig};

use crate::{
    app_data_store::AppStore,
    gatt_const::{
        SDP_NOTIFY_CHAR_UUID, SDP_WRITE_CHAR_UUID, WEBCAM_PNP_WRITE_CHAR_UUID,
    },
};

pub struct SdpExchanger {
    ble_adapter: Adapter,
    app_store: AppStore,
    main_thread: Option<tokio::task::JoinHandle<()>>,
    _tx_drop: Option<oneshot::Sender<()>>,
}

impl SdpExchanger {
    pub fn new(ble_adapter: Adapter, app_store: AppStore) -> Self {
        Self { ble_adapter, app_store, main_thread: None, _tx_drop: None }
    }

    pub async fn start(&mut self) -> Result<()> {
        let host_id = self.app_store.get_host_id();
        let host_id_uuid = Uuid::from_str(&host_id)?;

        let mut services = BTreeSet::<Uuid>::new();
        services.insert(host_id_uuid);

        let le_advertisement = Advertisement {
            service_uuids: services,
            discoverable: Some(true),
            local_name: Some(self.app_store.get_host_name()),
            ..Default::default()
        };

        let (tx, mut rx) = oneshot::channel();
        self._tx_drop = Some(tx);
        let (webcam_char_control, webcam_char_handle) =
            characteristic_control();

        let adapter = self.ble_adapter.clone();
        let app_store = self.app_store.clone();

        self.main_thread = Some(tokio::spawn(async move {
            let (answer_tx, answer_rx) = mpsc::channel(16384);

            let answer_recv = Arc::new(tokio::sync::Mutex::new(answer_rx));
            let offer_sdp = Arc::new(Mutex::new("".to_string()));
            let app = Application {
                services: vec![Service {
                    uuid: host_id_uuid,
                    primary: true,
                    characteristics: vec![
                        Characteristic {
                            uuid: SDP_WRITE_CHAR_UUID,
                            write: Some(CharacteristicWrite {
                                write_without_response: true,
                                write: true,
                                reliable_write: true,
                                method: CharacteristicWriteMethod::Fun(
                                    Box::new(move |new_value, req| {
                                        let offer_sdp = offer_sdp.clone();
                                        let tx = answer_tx.clone();
                                        info!("req: {:?}", req);
                                        async move {
                                            let offer =
                                                String::from_utf8(new_value)
                                                    .unwrap();
                                            info!("Offer: {}", offer);

                                            //identify end of base64 string
                                            if offer.ends_with("}") {
                                                //concat answer with next base64 string
                                                let offer_sdp_clone = {
                                                    let mut offer_sdp =
                                                        offer_sdp
                                                            .lock()
                                                            .unwrap();
                                                    *offer_sdp = format!(
                                                        "{}{}",
                                                        *offer_sdp, offer
                                                    );
                                                    offer_sdp.clone()
                                                };

                                                info!(
                                                    "Offer so far: {}",
                                                    offer_sdp_clone
                                                );

                                                let answer_sdp = start_webrtc(
                                                    &offer_sdp_clone,
                                                    0,
                                                )
                                                .await
                                                .unwrap();
                                                info!(
                                                    "Answer to send: {}",
                                                    &answer_sdp
                                                );
                                                tx.send(answer_sdp)
                                                    .await
                                                    .unwrap();
                                            } else {
                                                //concat answer with next base64 string
                                                let mut offer_sdp =
                                                    offer_sdp.lock().unwrap();
                                                *offer_sdp = format!(
                                                    "{}{}",
                                                    *offer_sdp, offer
                                                );
                                                //     info!("Offer so far: {}", offer_sdp);
                                            }
                                            return Ok(());
                                        }
                                        .boxed()
                                    }),
                                ),
                                ..Default::default()
                            }),
                            ..Default::default()
                        },
                        Characteristic {
                            uuid: SDP_NOTIFY_CHAR_UUID,
                            notify: Some(CharacteristicNotify {
                                notify: true,
                                method: CharacteristicNotifyMethod::Fun(
                                    Box::new(move |mut notifier| {
                                        let answer_rx = answer_recv.clone();
                                        async move {
                                            let answer_rx = answer_rx.clone();
                                                tokio::spawn(async move{
                                                    let answer_rx = answer_rx.clone();
                                                    info!(
                                                        "Notification session start with confirming={:?}",
                                                        notifier.confirming()
                                                    );

                                                    loop {
                                                        //send answer in chunks of 200 bytes
                                                        info!("Waiting for answer");
                                                        let mut answer_rx = answer_rx.lock().await;
                                                        let answer = answer_rx.recv().await.unwrap();
                                                        info!("Answer received {}", &answer);
                                                        let mut i = 0;
                                                        while i < answer.len() {
                                                            let end = std::cmp::min(i + 200, answer.len());
                                                            if let Err(err) = notifier
                                                                .notify(answer[i..end].to_string().into_bytes())
                                                                    .await
                                                            {
                                                                info!("Notification error: {}", &err);
                                                                break;
                                                            }
                                                            i = end;
                                                        }
                                                        break;
                                                    }

                                                    info!("Notification session stop");
                                                });
                                            }
                                            .boxed()
                                    }),
                                ),
                                ..Default::default()
                            }),
                            ..Default::default()
                        },
                        Characteristic {
                            uuid: WEBCAM_PNP_WRITE_CHAR_UUID,
                            write: Some(CharacteristicWrite {
                                write_without_response: true,
                                method: CharacteristicWriteMethod::Io,
                                ..Default::default()
                            }),
                            control_handle: webcam_char_handle,
                            ..Default::default()
                        },
                    ],
                    ..Default::default()
                }],
                ..Default::default()
            };

            let _advertisement_handle =
                Some(adapter.advertise(le_advertisement.clone()).await);
            let _adapter_handle =
                adapter.serve_gatt_application(app).await.unwrap();

            let mobile_cam =
                Arc::new(Mutex::new(HashMap::<String, u32>::new()));

            let mut reader_webcam_opt: Option<CharacteristicReader> = None;

            let mut read_webcam_buf = Vec::new();
            let mut advs_duration = interval(Duration::from_secs(10));
            pin_mut!(webcam_char_control);

            let mut adapter_events = adapter.events().await.unwrap();

            let mut offer_sdp = String::new();

            loop {
                let mobile_cam = mobile_cam.clone();
                // let writer_notify_opt = writer_notify_opt.clone();
                tokio::select! {

                    adapter_event = adapter_events.next() => {
                        match adapter_event {
                            Some(AdapterEvent::DeviceAdded(address)) => {
                                info!("Adapter event Device Added: {:?}", address);
                                let device = adapter.device(address).expect("Error when getting device");
                            }
                            Some(AdapterEvent::DeviceRemoved(address)) => {
                                let mobile_address = address.to_string();
                                if let Some(device_num) = mobile_cam.lock().unwrap().remove(&mobile_address) {
                                    info!("Unmounting device with number: {}", device_num);
                                    delete_device(device_num).expect("Error when removing device");
                                }
                                info!("Adapter event Device Removed: {}", mobile_address);
                            }
                            _ => {
                                info!("Adapter event: {:?}", adapter_event);
                            }
                        }
                    }

                    _ = &mut rx => {

                        //clean all devices in mobile_address
                        let mobile_cam = {mobile_cam.lock().unwrap().clone()};
                        for (mobile_address, device_num) in mobile_cam.iter() {
                            info!("Unmounting device with number: {}", device_num);
                            info!("Removing device with mobile address: {}", mobile_address);
                            let device_add = Address::from_str(&mobile_address).unwrap();
                            let device = adapter.device(device_add).expect("Error when getting device");
                            device.disconnect().await.expect("Error when disconnecting device");
                            adapter.remove_device(Address::from_str(&mobile_address).unwrap()).await.expect("Error when removing device");
                            delete_device(*device_num).expect("Error when removing device");
                        }
                        break
                    }

                    evt = webcam_char_control.next() => {
                        match evt {
                            Some(CharacteristicControlEvent::Write(req)) => {
                                reader_webcam_opt = Some(req.accept().unwrap());
                            },
                            Some(_) => print!("Another event"),
                            None => print!("Another None event")
                        }
                    }


                    /*
                       _ = advs_duration.tick() => {
                       if advertisement_handle.is_none() {
                       info!("Advertising sdp again");
                       advertisement_handle = Some(adapter.advertise(le_advertisement.clone()).await);
                       } else {
                       info!("Stop Advertising sdp again");
                       let _ = adapter.advertise(le_advertisement.clone()).await;
                       }
                       }
                       */

                    read_webcam_res = async {
                        match &mut reader_webcam_opt {
                            Some(reader) => {
                                read_webcam_buf = vec![0; reader.mtu()];
                                info!("mtu in reader: {}, device address: {}", reader.mtu(), reader.device_address());
                                let read = reader.read(&mut read_webcam_buf).await;
                                let null_index = read_webcam_buf.iter().position(|&x| x == 0).unwrap_or(read_webcam_buf.len());
                                let read_buf = read_webcam_buf[..null_index].to_vec();
                                let mobile_uuid = String::from_utf8(read_buf).unwrap();
                                let registered_mobiles = app_store.get_registered_mobiles().await;
                                info!("Sent Mobile ID: {}", mobile_uuid);
                                let mut mobile_cam = mobile_cam.lock().unwrap();
                                if registered_mobiles.contains_key(&mobile_uuid) && !mobile_cam.contains_key(&reader.device_address().to_string())  {
                                    info!("Registered Mobile ID: {}", mobile_uuid);
                                    let device_config = DeviceConfig {
                                        label: "Test Device".to_string(),
                                        min_width: 100,
                                        max_width: 4000,
                                        min_height: 100,
                                        max_height: 4000,
                                        max_buffers: 9,
                                        max_openers: 3,
                                    };
                                    // Create a device
                                    let device_num =
                                        add_device(None, device_config.clone()).expect("Error when creating the device");

                                    let mobile_address = reader.device_address().to_string();
                                    info!("Adding mobile address: {}", mobile_address);
                                    mobile_cam.insert(mobile_address, device_num);

                                    // Querying informations about a device
                                    // This returns the matchin device's configuration
                                    let cfg =
                                        query_device(device_num).expect("Error when querying the device");

                                    info!("device config: {:?}", cfg);

                                } else {
                                    info!("Mobile ID not registered or already have a device assigned: {}", mobile_uuid);
                                }
                                read
                            },
                            None => future::pending().await,
                        }
                    } => {
                        match read_webcam_res {
                            Ok(0) => {
                                info!("Write stream ended");
                                reader_webcam_opt = None;
                            }
                            Ok(n) => {
                                info!("Write request with {} bytes", n);
                                info!("Write request: {:?}", &read_webcam_buf[..n]);
                            }
                            Err(err) => {
                                info!("Write stream error: {}", &err);
                                reader_webcam_opt = None;
                            }
                        }
                    }

                }
            }
            info!("End of advertise thread");
        }));

        Ok(())
    }

    pub async fn stop(&mut self) -> Result<()> {
        self._tx_drop.take();
        if let Some(thread) = self.main_thread.take() {
            thread.await?;
        }
        Ok(())
    }
}
