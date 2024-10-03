pub mod host_entity;
pub mod mobile_entity;

use std::{
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use host_entity::{ConnectionType, HostInfo};
use log::info;
use mobile_entity::MobileInfo;

use bluer::Uuid;
use directories::ProjectDirs;
use serde_json;
use std::collections::HashMap;
use tokio;

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone)]
pub struct AppData {
    pub host_info: HostInfo,
    registered_mobiles: HashMap<String, MobileInfo>,
}

async fn create_app_data(config_file_path: &Path) -> Result<AppData, String> {
    let app_data_store = AppData {
        host_info: HostInfo {
            id: Uuid::new_v4().to_string(),
            name: match hostname::get() {
                Ok(name) => name.to_string_lossy().to_string(),
                Err(_) => "Unknown".to_owned(),
            },
            connection_type: ConnectionType::WIFI("WLAN".to_string()),
        },
        registered_mobiles: HashMap::new(),
    };

    let app_data_store_str =
        serde_json::to_string(&app_data_store).map_err(|e| e.to_string())?;

    tokio::fs::write(config_file_path, app_data_store_str)
        .await
        .map_err(|e| e.to_string())?;

    Ok(app_data_store)
}

type SharedAppData = Arc<Mutex<AppData>>;

#[derive(Clone)]
pub struct AppStore {
    app_data: SharedAppData,
    config_file_path: PathBuf,
}

impl AppStore {
    pub async fn new(config_file: &str) -> Self {
        let proj_dirs =
            ProjectDirs::from("com", "grr", "webcam-direct").unwrap();
        let config_dir = proj_dirs.config_dir();
        let config_file_path = config_dir.join(config_file);

        let app_data_store = if config_file_path.exists() {
            let config_file =
                tokio::fs::read_to_string(&config_file_path).await.unwrap();
            serde_json::from_str(&config_file).unwrap()
        } else {
            tokio::fs::create_dir_all(config_dir).await.unwrap();
            create_app_data(&config_file_path).await.unwrap()
        };

        AppStore {
            app_data: Arc::new(Mutex::new(app_data_store)),
            config_file_path,
        }
    }

    pub fn get_host_name(&self) -> String {
        let app_data = self.app_data.lock().unwrap();
        return app_data.host_info.name.clone();
    }

    pub fn get_host_id(&self) -> String {
        let app_data = self.app_data.lock().unwrap();
        return app_data.host_info.id.clone();
    }

    pub async fn get_registered_mobiles(&self) -> HashMap<String, MobileInfo> {
        let app_data = { self.app_data.lock().unwrap() };
        return app_data.registered_mobiles.clone();
    }

    pub async fn add_mobile(&self, mobile: MobileInfo) -> Result<(), String> {
        {
            let mut app_data =
                self.app_data.lock().map_err(|e| e.to_string())?;
            app_data.registered_mobiles.insert(mobile.id.clone(), mobile);
        }

        self.update_app_data().await.map_err(|e| e.to_string())?;

        info!(
            "Mobile added: {:?}",
            self.app_data.lock().unwrap().registered_mobiles
        );

        Ok(())
    }

    async fn update_app_data(&self) -> Result<(), String> {
        //avoid MutexGuard issue across await
        let app_data = { self.app_data.lock().unwrap().clone() };

        let app_data_str =
            serde_json::to_string(&app_data).map_err(|e| e.to_string())?;

        tokio::fs::write(&(self.config_file_path), app_data_str)
            .await
            .map_err(|e| e.to_string())?;

        Ok(())
    }
}
