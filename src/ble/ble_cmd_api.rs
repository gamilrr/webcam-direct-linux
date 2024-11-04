use tokio::sync::oneshot;

use crate::app_data::{HostSchema, MobileSchema};

pub type Address = String;
pub type Responder<T> = oneshot::Sender<T>;

use crate::error::Result;

#[derive(Debug)]
pub struct BleBuffer {
    pub remain_len: usize,
    pub payload: Vec<u8>,
}

#[derive(Debug)]
pub struct BleQuery {
    pub addr: Address,
    pub max_buffer_len: usize,
    pub resp: Responder<Result<BleBuffer>>,
}

#[derive(Debug)]
pub struct BleCmd {
    pub addr: Address,
    pub payload: BleBuffer,
    pub resp: Responder<Result<()>>,
}

//Ble Server-Client request
#[derive(Debug)]
pub enum BleApi {
    //Mobile Connection status
    MobileDisconnected(BleCmd),

    //Register mobile
    RegisterMobile(BleCmd),

    //Read host info
    HostInfo(BleQuery),

    //Mobile Identification
    MobileIdentification(BleCmd),
}
