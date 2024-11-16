use log::{error, info};
use tokio::sync::{mpsc, oneshot};

use crate::error::Result;

#[cfg(test)]
use mockall::automock;

use super::ble_cmd_api::{BleApi, BleBuffer};

//trait
#[cfg_attr(test, automock)]
pub trait MultiMobileCommService: Send + Sync + 'static {
    fn set_register_mobile(
        &mut self, addr: String, data: BleBuffer,
    ) -> Result<()>;
    fn read_host_info(
        &mut self, addr: String, max_size: usize,
    ) -> Result<BleBuffer>;
    fn device_disconnected(&mut self, addr: String) -> Result<()>;

    fn set_mobile_pnp_id(
        &mut self, addr: String, data: BleBuffer,
    ) -> Result<()>;
}

pub type ServerConn = mpsc::Sender<BleApi>;

pub struct BleServer {
    ble_tx: ServerConn,
    _drop_tx: oneshot::Sender<()>,
}

impl BleServer {
    pub fn new(
        mut comm_handler: impl MultiMobileCommService, req_buffer_size: usize,
    ) -> Self {
        let (ble_tx, mut ble_rx) = mpsc::channel(req_buffer_size);

        let (drop_tx, mut drop_rx) = oneshot::channel();

        tokio::spawn(async move {
            loop {
                tokio::select! {
                    Some(req) = ble_rx.recv() => {
                       handle_request(&mut comm_handler, req);
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

    pub fn connection(&self) -> ServerConn {
        self.ble_tx.clone()
    }
}

//This function does not return a Result since every request is successful
//if internally any operation fails, it should handle it accordingly
fn handle_request(comm_handler: &mut impl MultiMobileCommService, req: BleApi) {
    match req {
        BleApi::MobileDisconnected(cmd) => {
            info!("Mobile disconnected: {:?}", cmd.addr);
            if let Err(e) =
                cmd.resp.send(comm_handler.device_disconnected(cmd.addr))
            {
                error!("Error disconnecting mobile: {:?}", e);
            }
        }

        BleApi::RegisterMobile(cmd) => {
            if let Err(_) = cmd
                .resp
                .send(comm_handler.set_register_mobile(cmd.addr, cmd.payload))
            {
                error!("Error sending mobile registration response");
            }
        }

        BleApi::HostInfo(query) => {
            if let Err(e) = query.resp.send(
                comm_handler.read_host_info(query.addr, query.max_buffer_len),
            ) {
                error!("Error sending host info: {:?}", e);
            }
        }

        BleApi::MobilePnpId(cmd) => {
            if let Err(_) = cmd
                .resp
                .send(comm_handler.set_mobile_pnp_id(cmd.addr, cmd.payload))
            {
                error!("Error setting mobile PnP ID");
            }
        }

        _ => {
            info!("Unhandled event: {:?}", req);
        }
    };
}

#[cfg(test)]
mod tests {
    use mockall::predicate::eq;

    use crate::ble::ble_cmd_api::{BleCmd, BleQuery};

    use super::*;

    fn init_logger() {
        let _ = env_logger::builder().is_test(true).try_init();
    }

    #[tokio::test]
    async fn test_ble_server_host_info() {
        init_logger();
    }

    #[tokio::test]
    async fn test_ble_server_register_mobile() {
        init_logger();
    }
}
