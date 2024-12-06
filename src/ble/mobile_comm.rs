use std::collections::HashMap;

use log::{error, info};

use anyhow::anyhow;
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;

use super::{
    ble_cmd_api::{Address, BleBuffer, PubSubPublisher, PubSubSubscriber},
    ble_server::MultiMobileCommService,
};
use crate::vcam::VCamDevice;
use crate::{app_data::MobileSchema, error::Result};

#[cfg(test)]
use mockall::automock;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BufferComm {
    pub remain_len: usize,
    pub payload: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HostProvInfo {
    pub id: String,
    pub name: String,
    pub connection_type: String,
}

/*
 * This represent the json
 * {
 *  "id": "host_id",
 *  "name": "host_name",
 *  "connection_type": "WLAN"
 * }
 * */

/// A trait that defines the operations for interacting with the application's data store.
#[cfg_attr(test, automock)]
pub trait AppDataStore: Send + Sync + 'static {
    /// Retrieves the host provisioning info  from the data store.
    ///
    /// # Errors
    ///
    /// Returns an error if the host information is not found in the data store.
    fn get_host_prov_info(&self) -> Result<HostProvInfo>;

    /// Adds a mobile device to the data store.
    ///
    /// # Errors
    ///
    /// Returns an error if the host information is not found in the data store.
    fn add_mobile(&mut self, mobile: &MobileSchema) -> Result<()>;

    fn get_mobile(&self, id: &str) -> Result<MobileSchema>;
}

//States:
//Provisioning:   ReadHostInfo->WriteMobileInfo->WriteMobileId->ReadyToStream
//Identification: WriteMobileId->ReadyToStream
//
#[derive(Debug)]
enum MobileDataState {
    ReadHostInfo,

    WriteMobileInfo,

    WriteMobileId,

    SaveMobileData { mobile: MobileSchema },

    ReadyToStream { virtual_device: VCamDevice },
}

//State for the communication buffer
enum CommBufferStatus {
    RemainLen(usize),      //used in queries
    CurrentBuffer(String), //used in commands
}

struct ConnectedMobileData {
    pub mobile_state: MobileDataState,
    pub buffer_status: Option<CommBufferStatus>,
}

type MobileMap = HashMap<Address, ConnectedMobileData>;

//caller to send SDP data as a publisher
//to all mobiles subscribed
struct MobileSdpCaller {
    pub max_buffer_len: usize,
    pub publisher: PubSubPublisher,
}

pub struct MobileComm<Db> {
    db: Db,
    mobiles_connected: MobileMap,
    host_info: String,
    sdp_caller: MobileSdpCaller,
}

impl<Db: AppDataStore> MobileComm<Db> {
    pub fn new(db: Db) -> Result<Self> {
        let host = db.get_host_prov_info()?;
        let host_info = serde_json::to_string(&host)?;

        let (sender, _) = broadcast::channel(16);
        let sdp_caller = MobileSdpCaller {
            max_buffer_len: 19, //default mtu size 23 - 4 bytes for header
            publisher: sender,
        };

        Ok(Self {
            db,
            mobiles_connected: HashMap::new(),
            host_info,
            sdp_caller,
        })
    }
}

impl<Db: AppDataStore> MultiMobileCommService for MobileComm<Db> {
    fn device_disconnected(&mut self, addr: Address) -> Result<()> {
        if let Some(_) = self.mobiles_connected.remove(&addr) {
            info!("Removing device with addr {}", addr);
        } else {
            error!("Mobile not found in connected devices");
            return Err(anyhow!("Mobile not found"));
        }
        Ok(())
    }

    fn read_host_info(
        &mut self, addr: Address, max_buffer_len: usize,
    ) -> Result<BleBuffer> {
        info!("Host info requested by: {:?}", addr);

        let total_len = self.host_info.len();

        //if mobile is not connected, add it with the state ReadHostInfo
        //start condition
        if !self.mobiles_connected.contains_key(&addr) {
            self.mobiles_connected.insert(
                addr.clone(),
                ConnectedMobileData {
                    mobile_state: MobileDataState::ReadHostInfo,
                    buffer_status: Some(CommBufferStatus::RemainLen(total_len)),
                },
            );
        }

        if let ConnectedMobileData {
            mobile_state: MobileDataState::ReadHostInfo,
            buffer_status: Some(CommBufferStatus::RemainLen(remain)),
        } = self
            .mobiles_connected
            .get_mut(&addr)
            .ok_or_else(|| anyhow!("Mobile not found in connected devices"))?
        {
            let initial_len = total_len - *remain;

            let ble_comm = if max_buffer_len >= *remain {
                *remain = total_len;
                //move to next state
                self.mobiles_connected.insert(
                    addr.clone(),
                    ConnectedMobileData {
                        mobile_state: MobileDataState::WriteMobileInfo,
                        buffer_status: Some(CommBufferStatus::CurrentBuffer(
                            "".to_string(),
                        )),
                    },
                );
                info!("Mobile: {:?} in state WriteMobileInfo", addr);

                BufferComm {
                    remain_len: 0,
                    payload: self.host_info[initial_len..total_len].to_owned(),
                }
            } else {
                *remain -= max_buffer_len;
                BufferComm {
                    remain_len: *remain,
                    payload: self.host_info
                        [initial_len..initial_len + max_buffer_len]
                        .to_owned(),
                }
            };

            info!("Sending host info: {:?}", ble_comm);

            return Ok(serde_json::to_vec(&ble_comm)?);
        }

        Err(anyhow!("Mobile is not reading host info"))
    }

    fn set_register_mobile(
        &mut self, addr: Address, data: BleBuffer,
    ) -> Result<()> {
        info!("Registering mobile: {:?}", addr);

        if let ConnectedMobileData {
            mobile_state: MobileDataState::WriteMobileInfo,
            buffer_status: Some(CommBufferStatus::CurrentBuffer(current_buffer)),
        } = self
            .mobiles_connected
            .get_mut(&addr)
            .ok_or_else(|| anyhow!("Mobile not found in connected devices"))?
        {
            let buff_comm = serde_json::from_slice::<BufferComm>(&data)?;

            info!("buff_comm {:?}", buff_comm);

            current_buffer.push_str(&buff_comm.payload);

            info!("current_buffer {:?}", buff_comm);

            if buff_comm.remain_len == 0 {
                let mobile = serde_json::from_str(&current_buffer)?;
                self.db.add_mobile(&mobile)?;
                info!("Mobile registered: {:?}", mobile);
                //move to next state
                self.mobiles_connected.insert(
                    addr.clone(),
                    ConnectedMobileData {
                        mobile_state: MobileDataState::WriteMobileId,
                        buffer_status: Some(CommBufferStatus::CurrentBuffer(
                            "".to_string(),
                        )),
                    },
                );
                info!("Mobile: {:#?} in state WriteMobileId", mobile);
            }
        } else {
            return Err(anyhow!("Mobile is not writing mobile info"));
        }

        Ok(())
    }

    fn set_mobile_pnp_id(
        &mut self, addr: Address, data: BleBuffer,
    ) -> Result<()> {
        info!("Mobile Pnp ID: {:?}", addr);

        if !self.mobiles_connected.contains_key(&addr) {
            //new connection, already registered
            self.mobiles_connected.insert(
                addr.clone(),
                ConnectedMobileData {
                    mobile_state: MobileDataState::WriteMobileId,
                    buffer_status: Some(CommBufferStatus::CurrentBuffer(
                        "".to_string(),
                    )),
                },
            );
        }

        if let ConnectedMobileData {
            mobile_state: MobileDataState::WriteMobileId,
            buffer_status: Some(CommBufferStatus::CurrentBuffer(current_buffer)),
        } = self
            .mobiles_connected
            .get_mut(&addr)
            .ok_or_else(|| anyhow!("Mobile not found in connected devices"))?
        {
            let buff_comm = serde_json::from_slice::<BufferComm>(&data)?;

            info!("buff_comm {:?}", buff_comm);

            current_buffer.push_str(&buff_comm.payload);

            info!("current_buffer {:?}", buff_comm);

            if buff_comm.remain_len == 0 {
                let mobile_id = current_buffer.clone();
                if let Ok(mobile) = self.db.get_mobile(&mobile_id) {
                    info!("Mobile: {:#?} found", mobile);
                    //move to next State
                    self.mobiles_connected.insert(
                        addr.clone(),
                        ConnectedMobileData {
                            mobile_state: MobileDataState::SaveMobileData {
                                mobile,
                            },
                            buffer_status: None,
                        },
                    );
                } else {
                    error!("Mobile with id: {current_buffer} not found");
                    return Err(anyhow!("Mobile not found"));
                }
            }
        }

        Ok(())
    }

    fn subscribe_to_sdp_req(
        &mut self, addr: String, max_size: usize,
    ) -> Result<PubSubSubscriber> {
        if let Some(ConnectedMobileData {
            mobile_state: MobileDataState::SaveMobileData { mobile },
            buffer_status: None,
        }) = self.mobiles_connected.remove(&addr)
        {
            info!("Mobile: {:#?} is subscribe to SDP call", mobile);

            //move to next state
            self.mobiles_connected.insert(
                addr.clone(),
                ConnectedMobileData {
                    mobile_state: MobileDataState::ReadyToStream {
                        virtual_device: VCamDevice::new(mobile),
                    },
                    buffer_status: Some(CommBufferStatus::CurrentBuffer(
                        "".to_string(),
                    )),
                },
            );

            //update the max buffer len
            self.sdp_caller.max_buffer_len = max_size;
        } else {
            return Err(anyhow!(
                "Mobile not ready is not ready to start streaming"
            ));
        }

        Ok(self.sdp_caller.publisher.subscribe())
    }

    fn set_mobile_sdp_resp(
        &mut self, addr: String, data: BleBuffer,
    ) -> Result<()> {
        if let ConnectedMobileData {
            mobile_state: MobileDataState::ReadyToStream { .. },
            buffer_status: Some(CommBufferStatus::CurrentBuffer(current_buffer)),
        } = self
            .mobiles_connected
            .get_mut(&addr)
            .ok_or_else(|| anyhow!("Mobile not found in connected devices"))?
        {
            let buff_comm = serde_json::from_slice::<BufferComm>(&data)?;

            info!("buff_comm {:?}", buff_comm);

            current_buffer.push_str(&buff_comm.payload);

            info!("current_buffer {:?}", buff_comm);

            if buff_comm.remain_len == 0 {
                info!("SDP data: {:?}", current_buffer);

                //parse the sdp data and use it some how
                //ex: virtual_device.send_sdp_data(&data)?;
            }
        } else {
            return Err(anyhow!("Mobile is not ready to stream"));
        }

        Ok(())
    }
}

impl<Db: AppDataStore> MobileComm<Db> {}
