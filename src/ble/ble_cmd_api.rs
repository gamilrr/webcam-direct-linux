use tokio::sync::oneshot;

use crate::app_data::{HostSchema, MobileSchema};

pub type Address = String;
pub type Responder<T> = oneshot::Sender<T>;

//Generic Command and Query strcuts
//Query will get the state value
#[derive(Debug)]
pub struct BleQuery<T> {
    pub addr: Address,
    pub resp: Responder<T>,
}

//Command will modify the state
#[derive(Debug)]
pub struct BleCmd<T> {
    pub addr: Address,
    pub payload: T,
}

//Ble Server-Client request
#[derive(Debug)]
pub enum BleApi {
    //Mobile Connection status
    MobileDisconnected(BleCmd<Address>),

    //Register mobile
    RegisterMobile(BleCmd<MobileSchema>),

    //Get mobile info
    MobileInfo(BleQuery<MobileSchema>),

    //Read host info
    HostInfo(BleQuery<HostSchema>),
}
