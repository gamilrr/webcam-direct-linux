use std::collections::HashMap;

use log::info;

use anyhow::anyhow;
use serde::{Deserialize, Serialize};

use super::{
    ble_cmd_api::{Address, BleBuffer},
    ble_server::MultiMobileCommService,
};
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
}

//This is a enum bc we only do a single operation at a time,
//if a new operation is requested, it will overwrite the previous one
//turn into a hash map to keep track of parallel operations, not currently used
#[derive(Default, PartialEq, Debug)]
enum MobileDataState {
    ReadingHostInfo {
        remain_len: usize,
    },
    WritingMobileInfo {
        current_buffer: String,
    },

    #[default]
    Idle,
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
        info!("Mobile disconnected: {:?}", addr);
        self.connected.remove(&addr);
        Ok(())
    }

    fn set_register_mobile(
        &mut self, addr: Address, data: BleBuffer,
    ) -> Result<()> {
        info!("Registering mobile: {:?}", addr);

        //check if the mobile is connected or ready for the next op
        match self.connected.get(&addr) {
            Some(MobileDataState::WritingMobileInfo { .. }) => {}
            _ => {
                self.connected.insert(
                    addr.clone(),
                    MobileDataState::WritingMobileInfo {
                        current_buffer: String::new(),
                    },
                );
            }
        }

        if let MobileDataState::WritingMobileInfo { current_buffer } = self
            .connected
            .get_mut(&addr)
            .ok_or_else(|| anyhow!("Mobile not found in connected devices"))?
        {
            let buff_comm = serde_json::from_slice::<BufferComm>(&data)?;

            current_buffer.push_str(&buff_comm.payload);

            //current_buffer.extend_from_slice(&buff_comm.payload);

            if buff_comm.remain_len == 0 {
                let mobile =
                    serde_json::from_str(&current_buffer)?;
                self.db.add_mobile(&mobile)?;
                info!("Mobile registered: {:?}", mobile);
            }
        }

        Ok(())
    }

    fn read_host_info(
        &mut self, addr: Address, max_buffer_len: usize,
    ) -> Result<BleBuffer> {
        info!("Host info requested by: {:?}", addr);

        let total_len = self.host_info.len();

        //check if the mobile is connected or ready for the next op
        match self.connected.get(&addr) {
            Some(MobileDataState::ReadingHostInfo { .. }) => {}
            _ => {
                self.connected.insert(
                    addr.clone(),
                    MobileDataState::ReadingHostInfo { remain_len: total_len },
                );
            }
        }

        if let MobileDataState::ReadingHostInfo { remain_len } = self
            .connected
            .get_mut(&addr)
            .ok_or_else(|| anyhow!("Mobile not found in connected devices"))?
        {
            let initial_len = total_len - *remain_len;

            let end_len = if max_buffer_len >= *remain_len {
                *remain_len = 0;
                total_len
            } else {
                *remain_len -= max_buffer_len;
                initial_len + max_buffer_len
            };

            let payload = self.host_info[initial_len..end_len].to_string();

            info!("Sending host info: {:?}", payload);

            let ble_buffer = BufferComm { remain_len: *remain_len, payload };

            return Ok(serde_json::to_vec(&ble_buffer)?);
        }

        Err(anyhow!("Mobile is not reading host info"))
    }
}
