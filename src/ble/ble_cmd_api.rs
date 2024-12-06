use tokio::sync::{broadcast, oneshot};

pub type Address = String;
pub type BleBuffer = Vec<u8>;
pub type Responder<T> = oneshot::Sender<T>;

use crate::error::Result;

//Query
#[derive(Debug)]
pub struct Query<RespType> {
    pub addr: Address,
    pub max_buffer_len: usize,
    pub resp: Responder<Result<RespType>>,
}

//Cmd
#[derive(Debug)]
pub struct Cmd<RespType> {
    pub addr: Address,
    pub payload: BleBuffer,
    pub resp: Responder<Result<RespType>>,
}

pub type BleQuery = Query<BleBuffer>;
pub type BleCmd = Cmd<()>;

//PubSub
pub type PubSubPublisher = broadcast::Sender<BleBuffer>;
pub type PubSubSubscriber = broadcast::Receiver<BleBuffer>;
pub type BleSub = Query<PubSubSubscriber>;
pub type BlePub = Cmd<()>;

#[derive(Debug, Eq, PartialEq, Hash)]
pub enum PubSubTopic {
    SdpCall, //SDP call pub/sub
}

//Ble API
#[derive(Debug)]
pub enum BleApi {
    //Mobile Connection status
    MobileDisconnected(BleCmd),

    //Register mobile
    RegisterMobile(BleCmd),

    //Read host info
    HostInfo(BleQuery),

    //Mobile Pnp ID
    MobilePnpId(BleCmd),

    //Mobile SDP response
    MobileSdpResponse(BleCmd),

    //Publish/Subscribe API
    Subscribe(PubSubTopic, BleSub),
    Publish(PubSubTopic, BlePub),
}
