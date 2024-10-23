use cmd_event::BleCmdEvent;
mod mobiles_manager;

use mobiles_manager::MobilesManager;

use log::info;
use tokio::sync::mpsc::Sender;
use tokio::sync::{mpsc, oneshot};

use crate::app_data::{MobileSchema};
use crate::error::Result;

pub mod ble_events;
mod cmd_event;

pub trait MobileManager: Send + Sync + 'static {
    fn add_mobile(&mut self, mobile: MobileSchema) -> Result<()>;
    fn remove_mobile(&mut self, addr: String) -> Result<()>;
}

pub struct BleCtl {
    ble_tx: Sender<BleCmdEvent>,
    _drop_tx: oneshot::Sender<()>,
}

impl BleCtl {
    pub fn new(mut mobile_mgr: impl MobileManager) -> Self {
        let (ble_tx, mut ble_rx) = mpsc::channel(256);

        let (drop_tx, mut drop_rx) = oneshot::channel();

        tokio::spawn(async move {
            loop {
                tokio::select! {
                    Some(event) = ble_rx.recv() => {
                        Self::process_ble_event(&mut mobile_mgr, event);
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

    fn process_ble_event(mobile_mgr: &mut impl MobileManager, event: BleCmdEvent) {
        match event {
            BleCmdEvent::MobileConnected { addr, resp } => {
                info!("Mobile connected: {:?}", addr);
                if let Some(tx) = resp {
                    let _ = tx.send(addr);
                }
            }
            BleCmdEvent::MobileDisconnected { addr, resp } => {
                info!("Mobile disconnected: {:?}", addr);
                if let Some(tx) = resp {
                    let _ = tx.send(addr);
                }
            }
            _ => {
                info!("Unhandled event: {:?}", event);
            }
        }
    }

    pub fn get_ble_tx(&self) -> Sender<BleCmdEvent> {
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

    #[tokio::test]
    async fn test_mobile_manager_disconnect() {
        init_logger();
        let app_data = MockAppDataStore::new();
    }
}
