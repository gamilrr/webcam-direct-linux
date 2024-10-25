use log::{error, info};
use tokio::sync::mpsc::Sender;
use tokio::sync::{mpsc, oneshot};

use crate::app_data::{HostSchema, MobileSchema};
use crate::error::Result;

#[cfg(test)]
use mockall::automock;

use super::ble_cmd_api::BleApi;

//trait
#[cfg_attr(test, automock)]
pub trait BleDataService: Send + Sync + 'static {
    fn register_mobile(&mut self, mobile: MobileSchema) -> Result<()>;
    fn get_mobile(&self, addr: String) -> Result<MobileSchema>;
    fn get_host_info(&self) -> Result<HostSchema>;
    fn device_disconnected(&self, addr: String) -> Result<()>;
}

pub struct BleServer {
    ble_tx: Sender<BleApi>,
    _drop_tx: oneshot::Sender<()>,
}

impl BleServer {
    pub fn new(
        mut ble_data: impl BleDataService, req_buffer_size: usize,
    ) -> Self {
        let (ble_tx, mut ble_rx) = mpsc::channel(req_buffer_size);

        let (drop_tx, mut drop_rx) = oneshot::channel();

        tokio::spawn(async move {
            loop {
                tokio::select! {
                    Some(req) = ble_rx.recv() => {
                       Self::handle_request(&mut ble_data, req);
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

    //This function does not return a Result since every request is successful
    //if internally any operation fails, it should handle it accordingly
    fn handle_request(ble_data: &mut impl BleDataService, req: BleApi) {
        match req {
            BleApi::MobileDisconnected(cmd) => {
                info!("Mobile disconnected: {:?}", cmd.addr);
                if let Err(e) = ble_data.device_disconnected(cmd.addr) {
                    error!("Error disconnecting mobile: {:?}", e);
                }
            }

            BleApi::RegisterMobile(cmd) => {
                info!("Mobile registered: {:?}", cmd.addr);
                if let Err(_) = ble_data.register_mobile(cmd.payload) {
                    error!("Error registering mobile");
                }
            }

            BleApi::HostInfo(query) => {
                info!("Host info requested by: {:?}", query.addr);
                if let Ok(host_info) = ble_data.get_host_info() {
                    if let Err(e) = query.resp.send(host_info) {
                        error!("Error sending host info: {:?}", e);
                    }
                } else {
                    error!("Error getting host info from device service");
                }
            }

            BleApi::MobileInfo(query) => {
                info!("Mobile info requested by: {:?}", query.addr);
                if let Ok(mobile_info) = ble_data.get_mobile(query.addr) {
                    if let Err(e) = query.resp.send(mobile_info) {
                        error!("Error sending mobile info: {:?}", e);
                    }
                } else {
                    error!("Error getting mobile info from device service");
                }
            }
            _ => {
                info!("Unhandled event: {:?}", req);
            }
        };
    }

    pub fn get_tx(&self) -> Sender<BleApi> {
        self.ble_tx.clone()
    }
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

        //init the mock device service
        let mut data_service = MockBleDataService::new();

        //create the host schema data
        let host_info =
            HostSchema { id: "host_id".to_string(), ..Default::default() };

        let host_info_clone = host_info.clone();

        //set expectations
        data_service
            .expect_get_host_info()
            .times(1)
            .returning(move || Ok(host_info.clone()));

        //start the ble server
        let listener = BleServer::new(data_service, 10);

        //create the cmd
        let (tx_resp, rx_resp) = oneshot::channel();

        let query = BleApi::HostInfo(BleQuery {
            addr: "mobile_addr".to_string(),
            resp: tx_resp,
        });

        let listener_tx = listener.get_tx();

        listener_tx.send(query).await.unwrap();

        let host = rx_resp.await.unwrap();

        assert_eq!(host.id, host_info_clone.id);
    }

    #[tokio::test]
    async fn test_ble_server_register_mobile() {
        init_logger();

        //init the mock device service
        let mut data_service = MockBleDataService::new();

        //create the host schema data
        let mobile_schema =
            MobileSchema { id: "mobile_id".to_string(), ..Default::default() };

        //set expectations to register mobile
        data_service
            .expect_register_mobile()
            .withf(|mobile: &MobileSchema| mobile.id == "mobile_id".to_string())
            .times(1)
            .returning(|_| Ok(()));

        let mobile_schema_clone = mobile_schema.clone();

        data_service
            .expect_get_mobile()
            .with(eq("mobile_addr".to_string()))
            .times(1)
            .returning(move |_| Ok(mobile_schema.clone()));

        //start the ble server
        let listener = BleServer::new(data_service, 10);

        let cmd = BleApi::RegisterMobile(BleCmd {
            addr: "mobile_addr".to_string(),
            payload: mobile_schema_clone.clone(),
        });

        let listener_tx = listener.get_tx();

        listener_tx.send(cmd).await.unwrap();

        let (tx_resp, rx_resp) = oneshot::channel();

        let query = BleApi::MobileInfo(BleQuery {
            addr: "mobile_addr".to_string(),
            resp: tx_resp,
        });

        listener_tx.send(query).await.unwrap();

        let mobile = rx_resp.await.unwrap();

        assert_eq!(mobile.id, mobile_schema_clone.id);
    }
}
