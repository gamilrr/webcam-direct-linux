use tokio::sync::oneshot;

use crate::app_data::{HostSchema, MobileSchema};

pub type Address = String;

//Ble Server-Client request
//the response is optionally requested

#[derive(Debug)]
pub enum BleCmdApi {
    //Mobile Connection status
    MobileConnected {
        addr: Address,
        resp: Option<oneshot::Sender<Address>>,
    },

    MobileDisconnected {
        addr: Address,
        resp: Option<oneshot::Sender<Address>>,
    },

    //Register mobile
    RegisterMobile {
        addr: Address,
        payload: MobileSchema,
        resp: Option<oneshot::Sender<Address>>,
    },

    //Read host info
    HostInfo {
        addr: Address,
        resp: Option<oneshot::Sender<HostSchema>>,
    },
}
