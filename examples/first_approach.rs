//! Serves a Bluetooth GATT application using the callback programming model.

use std::io::Write;
use std::sync::Arc;
use std::{fs::File, rc::Rc};

use anyhow::Result;
use bluer::{
    adv::Advertisement,
    gatt::local::{
        Application, Characteristic, CharacteristicNotify, CharacteristicNotifyMethod,
        CharacteristicRead, CharacteristicWrite, CharacteristicWriteMethod, Service,
    },
    Uuid,
};
use futures::FutureExt;
use image::{ImageBuffer, Pixel, Rgb};
use openh264::decoder::Decoder;
use openh264::formats::YUVSource;
use openh264::nal_units;
use std::{str::FromStr, time::Duration};
use tokio::sync::{Mutex, Notify};
use tokio::{
    io::{AsyncBufReadExt, BufReader},
    time::sleep,
};
use v4l::io::traits::OutputStream;
use v4l::video::Output;
use v4l::{self, format};
use v4l::{Device, Format};
use webrtc::api::interceptor_registry::register_default_interceptors;
use webrtc::api::media_engine::{MediaEngine, MIME_TYPE_H264};
use webrtc::api::APIBuilder;
use webrtc::ice_transport::ice_connection_state::RTCIceConnectionState;
use webrtc::ice_transport::ice_server::RTCIceServer;
use webrtc::interceptor::registry::Registry;
use webrtc::media::io::h264_writer::H264Writer;
use webrtc::peer_connection::configuration::RTCConfiguration;
use webrtc::peer_connection::sdp::session_description::RTCSessionDescription;
use webrtc::rtcp::payload_feedbacks::picture_loss_indication::PictureLossIndication;
use webrtc::rtp_transceiver::rtp_codec::{
    RTCRtpCodecCapability, RTCRtpCodecParameters, RTPCodecType,
};
use webrtc::track::track_remote::TrackRemote;
use webrtc::util::marshal::Marshal;

fn save_to_webcam(payload: &[u8], decoder: &mut Decoder) {
    // Create a new H264 decoder

    // Decode the H264 data to raw video frames

    // Open the v4l2loopback device
    let mut device = Device::new(2).unwrap();
    let format = Format::new(640, 480, v4l::format::FourCC::new(b"H264"));
    device.set_format(&format).unwrap();

    println!("frame: {}", payload.len());

    // Write the YUYV frame to the v4l2loopback device
    device.write(payload).unwrap();
}

async fn save_to_disk(
    writer: Arc<Mutex<dyn webrtc::media::io::Writer + Send + Sync>>,
    track: Arc<TrackRemote>,
    notify: Arc<Notify>,
) -> Result<()> {
    let mut decoder = Decoder::new().unwrap();
    loop {
        tokio::select! {
            result = track.read_rtp() => {
                if let Ok((rtp_packet, _)) = result {
                    let mut w = writer.lock().await;

                    //let marshaled = rtp_packet.marshal().unwrap().to_vec();

                    //save_to_webcam(&rtp_packet.payload, &mut decoder);

                    w.write_rtp(&rtp_packet)?;
                }else{
                    println!("file closing begin after read_rtp error");
                    let mut w = writer.lock().await;
                    if let Err(err) = w.close() {
                        println!("file close err: {err}");
                    }
                    println!("file closing end after read_rtp error");
                    return Ok(());
                }
            }
            _ = notify.notified() => {
                println!("file closing begin after notified");
                let mut w = writer.lock().await;
                if let Err(err) = w.close() {
                    println!("file close err: {err}");
                }
                println!("file closing end after notified");
                return Ok(());
            }
        }
    }
}

async fn start_webrtc(offer_sdp: &str) -> Result<String, Box<dyn std::error::Error>> {
    let video_file = "/dev/video3";

    let h264_writer: Arc<Mutex<dyn webrtc::media::io::Writer + Send + Sync>> =
        Arc::new(Mutex::new(H264Writer::new(File::create(video_file)?)));

    let mut device = Device::new(2).unwrap();
    let format = Format::new(640, 480, v4l::format::FourCC::new(b"H264"));
    device.set_format(&format).unwrap();

    // Everything below is the WebRTC-rs API! Thanks for using it ❤️.

    // Create a MediaEngine object to configure the supported codec
    let mut m = MediaEngine::default();

    // Setup the codecs you want to use.
    // We'll use a H264 and Opus but you can also define your own
    m.register_codec(
        RTCRtpCodecParameters {
            capability: RTCRtpCodecCapability {
                mime_type: MIME_TYPE_H264.to_owned(),
                clock_rate: 90000,
                channels: 0,
                sdp_fmtp_line: "".to_owned(),
                rtcp_feedback: vec![],
            },
            payload_type: 102,
            ..Default::default()
        },
        RTPCodecType::Video,
    )?;

    // Create a InterceptorRegistry. This is the user configurable RTP/RTCP Pipeline.
    // This provides NACKs, RTCP Reports and other features. If you use `webrtc.NewPeerConnection`
    // this is enabled by default. If you are manually managing You MUST create a InterceptorRegistry
    // for each PeerConnection.
    let mut registry = Registry::new();

    // Use the default set of Interceptors
    registry = register_default_interceptors(registry, &mut m)?;

    // Create the API object with the MediaEngine
    let api = APIBuilder::new()
        .with_media_engine(m)
        .with_interceptor_registry(registry)
        .build();

    // Prepare the configuration
    let config = RTCConfiguration {
        ice_servers: vec![RTCIceServer {
            urls: vec!["stun:stun.l.google.com:19302".to_owned()],
            ..Default::default()
        }],
        ..Default::default()
    };

    // Create a new RTCPeerConnection
    let peer_connection = Arc::new(api.new_peer_connection(config).await?);

    peer_connection
        .add_transceiver_from_kind(RTPCodecType::Video, None)
        .await?;

    let notify_tx = Arc::new(Notify::new());
    let notify_rx = notify_tx.clone();

    // Set a handler for when a new remote track starts, this handler saves buffers to disk as
    // an ivf file, since we could have multiple video tracks we provide a counter.
    // In your application this is where you would handle/process video
    let pc = Arc::downgrade(&peer_connection);
    peer_connection.on_track(Box::new(move |track, _, _| {
        // Send a PLI on an interval so that the publisher is pushing a keyframe every rtcpPLIInterval
        let media_ssrc = track.ssrc();
        let pc2 = pc.clone();
        tokio::spawn(async move {
            let mut result = Result::<usize>::Ok(0);
            while result.is_ok() {
                let timeout = tokio::time::sleep(Duration::from_secs(3));
                tokio::pin!(timeout);

                tokio::select! {
                    _ = timeout.as_mut() =>{
                        if let Some(pc) = pc2.upgrade(){
                            result = pc.write_rtcp(&[Box::new(PictureLossIndication{
                                sender_ssrc: 0,
                                media_ssrc,
                            })]).await.map_err(Into::into);
                        }else {
                            break;
                        }
                    }
                };
            }
        });

        let notify_rx2 = Arc::clone(&notify_rx);
        let h264_writer2 = Arc::clone(&h264_writer);
        Box::pin(async move {
            let codec = track.codec();
            let mime_type = codec.capability.mime_type.to_lowercase();
            if mime_type == MIME_TYPE_H264.to_lowercase() {
                println!("Got h264 track, saving to disk as output.h264");
                tokio::spawn(async move {
                    let _ = save_to_disk(h264_writer2, track, notify_rx2).await;
                });
            }
        })
    }));

    let (done_tx, mut done_rx) = tokio::sync::mpsc::channel::<()>(1);

    // Set the handler for ICE connection state
    // This will notify you when the peer has connected/disconnected
    peer_connection.on_ice_connection_state_change(Box::new(
        move |connection_state: RTCIceConnectionState| {
            println!("Connection State has changed {connection_state}");

            if connection_state == RTCIceConnectionState::Connected {
                println!("Ctrl+C the remote client to stop the demo");
            } else if connection_state == RTCIceConnectionState::Failed {
                notify_tx.notify_waiters();

                println!("Done writing media files");

                let _ = done_tx.try_send(());
            }
            Box::pin(async {})
        },
    ));

    // Wait for the offer to be pasted
    //let line = signal::must_read_stdin()?;
    //Read from BLE
    println!("Read from BLE {}", offer_sdp);
    let offer = serde_json::from_str::<RTCSessionDescription>(&offer_sdp)?;

    // Set the remote SessionDescription
    peer_connection.set_remote_description(offer).await?;

    // Create an answer
    let answer = peer_connection.create_answer(None).await?;

    // Create channel that is blocked until ICE Gathering is complete
    let mut gather_complete = peer_connection.gathering_complete_promise().await;

    // Sets the LocalDescription, and starts our UDP listeners
    peer_connection.set_local_description(answer).await?;

    // Block until ICE Gathering is complete, disabling trickle ICE
    // we do this because we only can exchange one signaling message
    // in a production application you should exchange ICE Candidates via OnICECandidate
    let _ = gather_complete.recv().await;

    // Output the answer in base64 so we can paste it in browser
    if let Some(local_desc) = peer_connection.local_description().await {
        let json_str = serde_json::to_string(&local_desc)?;
        println!("Answer: {}", json_str);
        return Ok(json_str);
    } else {
        println!("generate local_description failed!");
    }

    return Ok("".to_string());
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> bluer::Result<()> {
    let session = bluer::Session::new().await?;
    let adapter = session.default_adapter().await?;
    adapter.set_powered(true).await?;

    let service_uuid: Uuid = Uuid::from_str("124ddac5-b107-46a0-ade0-4ae8b2b700f5").unwrap();
    let characteristic_host_uuid: Uuid =
        Uuid::from_str("124ddac6-b107-46a0-ade0-4ae8b2b700f5").unwrap();
    let characteristic_device_uuid: Uuid =
        Uuid::from_str("124ddac7-b107-46a0-ade0-4ae8b2b700f5").unwrap();

    println!(
        "Advertising on Bluetooth adapter {} with address {}",
        adapter.name(),
        adapter.address().await?
    );
    let le_advertisement = Advertisement {
        service_uuids: vec![service_uuid].into_iter().collect(),
        discoverable: Some(true),
        local_name: Some("gatt_server".to_string()),
        ..Default::default()
    };
    let adv_handle = adapter.advertise(le_advertisement).await?;

    println!(
        "Serving GATT service on Bluetooth adapter {}",
        adapter.name()
    );

    let answer = Arc::new(Mutex::new("".to_string()));
    let answer_write = answer.clone();
    let answer_notify = answer.clone();
    let offer_sdp = Arc::new(Mutex::new("".to_string()));

    let app = Application {
        services: vec![Service {
            uuid: service_uuid,
            primary: true,
            characteristics: vec![
                Characteristic {
                    uuid: characteristic_device_uuid,
                    write: Some(CharacteristicWrite {
                        write: true,
                        write_without_response: true,
                        method: CharacteristicWriteMethod::Fun(Box::new(move |new_value, req| {
                            let answer = answer_write.clone();
                            let offer_sdp = offer_sdp.clone();
                            async move {
                                let offer = String::from_utf8(new_value).unwrap();
                                //identify end of base64 string
                                if offer.ends_with("}") {
                                    //concat answer with next base64 string
                                    let mut offer_sdp = offer_sdp.lock().await;
                                    *offer_sdp = format!("{}{}", *offer_sdp, offer);
                                    println!("Offer: {}", &*offer_sdp);
                                    let answer_sdp = start_webrtc(&offer_sdp).await.unwrap();
                                    println!("Answer: {}", &answer_sdp);
                                    let mut answer = answer.lock().await;
                                    *answer = answer_sdp;
                                } else {
                                    //concat answer with next base64 string
                                    let mut offer_sdp = offer_sdp.lock().await;
                                    *offer_sdp = format!("{}{}", *offer_sdp, offer);
                                }

                                Ok(())
                            }
                            .boxed()
                        })),
                        ..Default::default()
                    }),
                    ..Default::default()
                },
                Characteristic {
                    uuid: characteristic_host_uuid,
                    notify: Some(CharacteristicNotify {
                        notify: true,
                        method: CharacteristicNotifyMethod::Fun(Box::new(move |mut notifier| {
                            let answer = answer_notify.clone();
                            async move {
                                tokio::spawn(async move {
                                    println!(
                                        "Notification session start with confirming={:?}",
                                        notifier.confirming()
                                    );
                                    loop {
                                        {
                                            sleep(Duration::from_secs(1)).await;
                                            let answer = answer.lock().await;
                                            if answer.len() == 0 || !answer.ends_with("}") {
                                                println!(
                                                    "No answer to notify, asnwer: {}",
                                                    &*answer
                                                );
                                                continue;
                                            }
                                            println!("Notifying with value {:x?}", &*answer);
                                            //send answer in chunks of 200 bytes
                                            let mut i = 0;
                                            while i < answer.len() {
                                                let end = std::cmp::min(i + 200, answer.len());
                                                if let Err(err) = notifier
                                                    .notify(answer[i..end].to_string().into_bytes())
                                                    .await
                                                {
                                                    println!("Notification error: {}", &err);
                                                    break;
                                                }
                                                i = end;
                                            }
                                            break;
                                        }
                                    }

                                    println!("Notification session stop");
                                });
                            }
                            .boxed()
                        })),
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

    println!("Service ready. Press enter to quit.");
    let stdin = BufReader::new(tokio::io::stdin());
    let mut lines = stdin.lines();
    let _ = lines.next_line().await;

    println!("Removing service and advertisement");
    drop(app_handle);
    drop(adv_handle);
    sleep(Duration::from_secs(1)).await;

    Ok(())
}
