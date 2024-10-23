use tokio::sync::oneshot;

use crate::app_data::MobileSchema;

pub type Address = String;

#[derive(Debug)]
pub enum BleCmdEvent {
    //Mobile Connection status
    MobileConnected {
        addr: Address,
        resp: Option<oneshot::Sender<Address>>,
    },

    MobileDisconnected {
        addr: Address,
        resp: Option<oneshot::Sender<Address>>,
    },

    //Host<->Mobile Provisioning
    RegisterMobile {
        addr: Address,
        payload: MobileSchema,
        resp: Option<oneshot::Sender<()>>,
    },
}
