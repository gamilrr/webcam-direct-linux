use bluer::Uuid;

// Constants for the BLE Provisioning Service
pub const PROV_SERV_HOST_UUID: Uuid = Uuid::from_u128(0x124ddac5b10746a0ade04ae8b2b700f5);
pub const PROV_CHAR_HOST_INFO_UUID: Uuid = Uuid::from_u128(0x124ddac6b10746a0ade04ae8b2b700f5);
pub const PROV_CHAR_MOBILE_INFO_UUID: Uuid = Uuid::from_u128(0x124ddac7b10746a0ade04ae8b2b700f5);

//Webrtc SDP offer and answer
pub const SDP_WRITE_CHAR_UUID: Uuid = Uuid::from_u128(0x124ddac8b10746a0ade04ae8b2b700f5);
pub const SDP_NOTIFY_CHAR_UUID: Uuid = Uuid::from_u128(0x124ddac9b10746a0ade04ae8b2b700f5);
pub const WEBCAM_PNP_WRITE_CHAR_UUID: Uuid = Uuid::from_u128(0x124ddacab10746a0ade04ae8b2b700f5);
