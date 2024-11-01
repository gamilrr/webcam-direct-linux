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

    /// Retrieves the host name from the data store.
    ///
    /// # Errors
    ///
    /// Returns an error if the host information is not found in the data store.
    fn get_host_name(&self) -> Result<String>;

    /// Retrieves the host ID from the data store.
    ///
    /// # Errors
    ///
    /// Returns an error if the host information is not found in the data store.
    fn get_host_id(&self) -> Result<String>;

    /// Adds a mobile device to the data store.
    ///
    /// # Errors
    ///
    /// Returns an error if the host information is not found in the data store.
    fn add_mobile(&mut self, mobile: &MobileSchema) -> Result<()>;
}

enum MobileDataState {
    Provisioning { remain_len: usize },
    Streaming,
}

type MobileMap = HashMap<Address, MobileDataState>;

pub struct MobileComm<Db> {
    db: Db,
    connected: MobileMap,
    host_info: Vec<u8>,
}

impl<Db: AppDataStore> MobileComm<Db> {
    pub fn new(db: Db) -> Result<Self> {
        let host_info = serde_json::to_vec(&db.get_host_prov_info()?)?;

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
        &mut self, addr: Address, payload: BleBuffer,
    ) -> Result<()> {
        info!("Mobile registered: {:?}", addr);
        Ok(())
    }

    fn get_host_info(
        &mut self, addr: Address, max_buffer_len: usize,
    ) -> Result<BleBuffer> {
        info!("Host info requested by: {:?}", addr);

        let total_len = self.host_info.len();

        //check if the mobile is connected if not add it to the connected devices
        if !self.connected.contains_key(&addr) {
            self.connected.insert(
                addr.clone(),
                MobileDataState::Provisioning { remain_len: total_len },
            );
        }

        if let MobileDataState::Provisioning { remain_len } = self
            .connected
            .get_mut(&addr)
            .ok_or_else(|| anyhow!("Mobile not found in connected devices"))?
        {
            if *remain_len <= max_buffer_len {
                self.connected.insert(addr, MobileDataState::Streaming);

                return Ok(BleBuffer {
                    remain_len: 0,
                    payload: self.host_info.clone(),
                });
            }

            let initial_len = total_len - *remain_len;
            let payload = self.host_info.clone()
                [initial_len..initial_len + max_buffer_len]
                .to_vec();

            *remain_len -= max_buffer_len;

            return Ok(BleBuffer { remain_len: *remain_len, payload });
        }

        Err(anyhow!("Mobile not ready for provisioning"))
    }
}
