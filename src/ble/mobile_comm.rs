use std::collections::HashMap;

use log::{error, info};

use anyhow::anyhow;
use serde::{Deserialize, Serialize};

use super::{
    ble_cmd_api::{Address, BleBuffer, SubSender},
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
//Provisioning:  ReadHostInfo->WriteMobileInfo->WriteMobileId->ReadyToStream
//Identification:WriteMobileId->ReadyToStream
//
#[derive(Debug)]
enum MobileDataState {
    ReadHostInfo {
        remain_len: usize,
    },

    WriteMobileInfo {
        current_buffer: String,
    },

    WriteMobileId {
        current_buffer: String,
    },

    InitVirtualDevice {
        virtual_device: VCamDevice,
    },

    ReadyToStream {
        virtual_device: VCamDevice,
        publisher: SubSender<BleBuffer>,
    },
}

type MobileMap = HashMap<Address, MobileDataState>;

pub struct MobileComm<Db> {
    db: Db,
    connected: MobileMap,
    host_info: String,
}

impl<Db: AppDataStore> MobileComm<Db> {
    pub fn new(db: Db) -> Result<Self> {
        let host = db.get_host_prov_info()?;
        let host_info = serde_json::to_string(&host)?;

        Ok(Self { db, connected: HashMap::new(), host_info })
    }
}

impl<Db: AppDataStore> MultiMobileCommService for MobileComm<Db> {
    fn device_disconnected(&mut self, addr: Address) -> Result<()> {
        if let Some(_) = self.connected.remove(&addr) {
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
        if !self.connected.contains_key(&addr) {
            self.connected.insert(
                addr.clone(),
                MobileDataState::ReadHostInfo { remain_len: total_len },
            );
        }

        if let MobileDataState::ReadHostInfo { remain_len } = self
            .connected
            .get_mut(&addr)
            .ok_or_else(|| anyhow!("Mobile not found in connected devices"))?
        {
            let initial_len = total_len - *remain_len;

            let ble_comm = if max_buffer_len >= *remain_len {
                *remain_len = total_len;
                //move to next state
                self.connected.insert(
                    addr.clone(),
                    MobileDataState::WriteMobileInfo {
                        current_buffer: "".to_string(),
                    },
                );
                info!("Mobile: {:#?} in state WriteMobileInfo", addr);

                BufferComm {
                    remain_len: 0,
                    payload: self.host_info[initial_len..total_len].to_owned(),
                }
            } else {
                *remain_len -= max_buffer_len;
                BufferComm {
                    remain_len: *remain_len,
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

        if let MobileDataState::WriteMobileInfo { current_buffer } = self
            .connected
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
                self.connected.insert(
                    addr.clone(),
                    MobileDataState::WriteMobileId {
                        current_buffer: String::new(),
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

        if !self.connected.contains_key(&addr) {
            //new connection, already registered
            self.connected.insert(
                addr.clone(),
                MobileDataState::WriteMobileId {
                    current_buffer: String::new(),
                },
            );
        }

        if let MobileDataState::WriteMobileId { current_buffer } = self
            .connected
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
                    self.connected.insert(
                        addr.clone(),
                        MobileDataState::InitVirtualDevice {
                            //TODO: create a virtual device
                            virtual_device: VCamDevice::new("vcam".to_string()),
                        },
                    );

                    info!("Mobile: {:#?} in state ReadyToStream", mobile);
                } else {
                    error!("Mobile with id: {current_buffer} not found");
                    return Err(anyhow!("Mobile not found"));
                }
            }
        }

        Ok(())
    }

    fn sdp_call_sub(
        &mut self, addr: String, sender: SubSender<BleBuffer>,
    ) -> Result<()> {

        if let MobileDataState::InitVirtualDevice { virtual_device } = self
            .connected
            .get_mut(&addr)
            .ok_or_else(|| anyhow!("Mobile not found in connected devices"))?
        {
            //move to next state

            info!("Mobile: {:#?} in state ReadyToStream", addr);
        }else{
            return Err(anyhow!("Mobile not in InitVirtualDevice state"));
        }

        Ok(())
    }
}
