use tokio::sync::oneshot;

use crate::app_data::{HostSchema, MobileSchema};

pub type Address = String;
pub type Responder<T> = oneshot::Sender<T>;

//Ble Server-Client request
//the response is optionally requested

#[derive(Debug)]
pub enum BleCmdApi {
    //Mobile Connection status
    MobileConnected {
        addr: Address,
        resp: Option<Responder<Address>>,
    },

    MobileDisconnected {
        addr: Address,
        resp: Option<Responder<Address>>,
    },

    //Register mobile
    RegisterMobile {
        addr: Address,
        payload: MobileSchema,
        resp: Option<Responder<Address>>,
    },

    //Read host info
    HostInfo {
        addr: Address,
        resp: Responder<HostSchema>,
    },
}
