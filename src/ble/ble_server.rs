use log::info;
use tokio::sync::mpsc::Sender;
use tokio::sync::{mpsc, oneshot};

use crate::app_data::{HostSchema, MobileSchema};
use crate::error::Result;

#[cfg(test)]
use mockall::automock;

use super::ble_cmd_api::BleCmdApi;

//trait
#[cfg_attr(test, automock)]
pub trait DevicesStatusService: Send + Sync + 'static {
    fn add_mobile(&mut self, mobile: MobileSchema) -> Result<()>;
    fn remove_mobile(&mut self, addr: String) -> Result<()>;
    fn get_host_info(&self) -> Result<HostSchema>;
}

pub struct BleServer {
    ble_tx: Sender<BleCmdApi>,
    _drop_tx: oneshot::Sender<()>,
}

impl BleServer {
    pub fn new(
        mut dev_service: impl DevicesStatusService, req_buffer_size: usize,
    ) -> Self {
        let (ble_tx, mut ble_rx) = mpsc::channel(req_buffer_size);

        let (drop_tx, mut drop_rx) = oneshot::channel();

        tokio::spawn(async move {
            loop {
                tokio::select! {
                    Some(req) = ble_rx.recv() => {
                        Self::handle_request(&mut dev_service, req);
                    }
                    _ = &mut drop_rx => {
                        info!("MobileManager task is stopping");
                        break;
                    }
                }
            }
        });

        Self { ble_tx, _drop_tx: drop_tx }
    }

    fn handle_request(
        mobile_mgr: &mut impl DevicesStatusService, req: BleCmdApi,
    ) {
        match req {
            BleCmdApi::MobileConnected { addr, resp } => {
                info!("Mobile connected: {:?}", addr);
                if let Some(tx) = resp {
                    let _ = tx.send(addr);
                }
            }
            BleCmdApi::MobileDisconnected { addr, resp } => {
                info!("Mobile disconnected: {:?}", addr);
                if let Some(tx) = resp {
                    let _ = tx.send(addr);
                }
            }
            _ => {
                info!("Unhandled event: {:?}", req);
            }
        }
    }

    pub fn get_ble_tx(&self) -> Sender<BleCmdApi> {
        self.ble_tx.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app_data::MockAppDataStore;

    fn init_logger() {
        let _ = env_logger::builder().is_test(true).try_init();
    }

    #[tokio::test]
    async fn test_mobile_manager() {
        init_logger();

        let app_data = MockAppDataStore::new();
    }
}
