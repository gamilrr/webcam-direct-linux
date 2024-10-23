//! This module defines the `AppDataStore` trait and the `AppData` struct which provides
//! methods to interact with the application's data store. It includes functionality to
//! get host information and add mobile devices to the store.

mod kv_db;
mod schemas;

use anyhow::anyhow;
use kv_db::KvDbOps;
use log::error;
use log::info;
use schemas::ConnectionType;
use schemas::HostSchema;
pub use schemas::{MobileSchema, MobileId};
use uuid::Uuid;

use crate::error::Result;

#[cfg(test)]
use mockall::automock;

/// A trait that defines the operations for interacting with the application's data store.
#[cfg_attr(test, automock)]
pub trait AppDataStore {
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

/// A struct that holds the application's data store.
pub struct AppData<Db> {
    data_db: Db,
}

/// A struct that holds information about the host.
pub struct HostInfo {
    pub name: String,
    pub connection_type: ConnectionType,
}

impl<Db> AppData<Db>
where
    Db: KvDbOps,
{
    /// Creates a new `AppData` instance.
    ///
    /// If the host information is not present in the data store, it adds it.
    ///
    /// # Errors
    ///
    /// Returns an error if there is an issue reading from or writing to the data store.
    pub fn new(data_db: Db, host_info: HostInfo) -> Result<Self> {
        // If host_info is not present in the db, add it
        if let None = data_db.read::<HostSchema>("host_info")? {
            info!("Host info not found in the database. Adding new host info.");
            let host_info = HostSchema {
                id: Uuid::new_v4().to_string(),
                name: host_info.name,
                connection_type: host_info.connection_type,
                registered_mobiles: Vec::new(),
            };
            data_db.add("host_info", &host_info)?;
        } else {
            info!("Host info already exists in the database.");
        }

        Ok(AppData { data_db })
    }
}

impl<Db> AppDataStore for AppData<Db>
where
    Db: KvDbOps,
{
    fn get_host_name(&self) -> Result<String> {
        if let Some(host) = self.data_db.read::<HostSchema>("host_info")? {
            info!("Host name retrieved successfully.");
            return Ok(host.name);
        }
        error!("Failed to retrieve host name: Host info not found.");
        Err(anyhow!("Host info not found"))
    }

    fn get_host_id(&self) -> Result<String> {
        if let Some(host) = self.data_db.read::<HostSchema>("host_info")? {
            info!("Host ID retrieved successfully.");
            return Ok(host.id);
        }
        error!("Failed to retrieve host ID: Host info not found.");
        Err(anyhow!("Host info not found"))
    }

    fn add_mobile(&mut self, mobile: &MobileSchema) -> Result<()> {
        if let Some(mut host) = self.data_db.read::<HostSchema>("host_info")? {
            // Update the host info with the new mobile id
            host.registered_mobiles.push(mobile.id.clone());
            self.data_db.update("host_info", &host)?;
            // Store the mobile info
            self.data_db.add(&mobile.id, mobile)?;
            info!("Mobile device added successfully.");
            return Ok(());
        }

        error!("Failed to add mobile device: Host info not found.");
        Err(anyhow!("Host info not found"))
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use kv_db::MockKvDbOps;
    use mockall::predicate::eq;

    fn init_logger() {
        let _ = env_logger::builder().is_test(true).try_init();
    }

    #[test]
    fn test_new_app_data() {
        init_logger();
        let mut mock_db = MockKvDbOps::new();

        let host_info = HostInfo {
            name: "TestHost".to_string(),
            connection_type: ConnectionType::WLAN,
        };

        mock_db
            .expect_read::<HostSchema>()
            .with(eq("host_info"))
            .returning(|_| Ok(None));

        mock_db
            .expect_add::<HostSchema>()
            .withf(|key, host| key == "host_info" && host.name == "TestHost")
            .returning(|_, _| Ok(()));

        let app_data = AppData::new(mock_db, host_info);
        assert!(app_data.is_ok());
    }

    #[test]
    fn test_get_host_name() {
        init_logger();
        let mut mock_db = MockKvDbOps::new();
        let host_schema = HostSchema {
            id: "123".to_string(),
            name: "TestHost".to_string(),
            connection_type: ConnectionType::WLAN,
            registered_mobiles: Vec::new(),
        };

        mock_db
            .expect_read::<HostSchema>()
            .with(eq("host_info"))
            .returning(move |_| Ok(Some(host_schema.clone())));

        let app_data = AppData { data_db: mock_db };
        let host_name = app_data.get_host_name();
        assert_eq!(host_name.unwrap(), "TestHost");
    }

    #[test]
    fn test_get_host_id() {
        init_logger();
        let mut mock_db = MockKvDbOps::new();
        let host_schema = HostSchema {
            id: "123".to_string(),
            name: "TestHost".to_string(),
            connection_type: ConnectionType::WLAN,
            registered_mobiles: Vec::new(),
        };

        mock_db
            .expect_read::<HostSchema>()
            .with(eq("host_info"))
            .returning(move |_| Ok(Some(host_schema.clone())));

        let app_data = AppData { data_db: mock_db };
        let host_id = app_data.get_host_id();
        assert_eq!(host_id.unwrap(), "123");
    }

    #[test]
    fn test_add_mobile() {
        init_logger();
        let mut mock_db = MockKvDbOps::new();
        let host_schema = HostSchema {
            id: "123".to_string(),
            name: "TestHost".to_string(),
            connection_type: ConnectionType::WLAN,
            registered_mobiles: Vec::new(),
        };

        let mobile_schema = MobileSchema {
            id: "mobile_1".to_string(),
            name: "Mobile1".to_string(),
            ..Default::default()
        };

        mock_db
            .expect_read::<HostSchema>()
            .with(eq("host_info"))
            .returning(move |_| Ok(Some(host_schema.clone())));

        mock_db
            .expect_update::<HostSchema>()
            .withf(|key, host| {
                key == "host_info"
                    && host.registered_mobiles.contains(&"mobile_1".to_string())
            })
            .returning(|_, _| Ok(()));

        mock_db
            .expect_add::<MobileSchema>()
            .withf(|key, mobile| key == "mobile_1" && mobile.name == "Mobile1")
            .returning(|_, _| Ok(()));

        let mut app_data = AppData { data_db: mock_db };
        let result = app_data.add_mobile(&mobile_schema);
        assert!(result.is_ok());
    }
}
