//! This module provides constants and utilities for interacting with the nl80211
//! subsystem in the Linux kernel. The nl80211 subsystem is used for configuring
//! wireless devices and managing wireless connections.

/// The generic netlink family name for nl80211.
pub const NL80211_GENL_NAME: &str = "nl80211";

/// Enum representing various nl80211 commands.
#[neli::neli_enum(serialized_type = "u8")]
pub enum Nl80211Command {
    /// Unspecified command.
    Unspecified = 0,
    /// Command to get wireless physical device information.
    GetWiPhy = 1,
    /// Command to create a new network interface.
    NewInterface = 7,
    /// Command to delete a network interface.
    DelInterface = 8,
    // Many more commands can be added here.
}

/// Implement the `Cmd` trait for `Nl80211Command` to use it as a generic netlink command.
impl neli::consts::genl::Cmd for Nl80211Command {}

/// Enum representing various nl80211 attributes.
#[neli::neli_enum(serialized_type = "u16")]
pub enum Nl80211Attribute {
    /// Unspecified attribute.
    Unspecified = 0,
    /// Attribute representing the wireless physical device.
    Wiphy = 1,
    /// Attribute representing the name of the wireless physical device.
    WiphyName = 2,
    /// Attribute representing the interface index.
    Ifindex = 3,
    /// Attribute representing the interface name.
    Ifname = 4,
    /// Attribute representing the interface type.
    Iftype = 5,
    /// Attribute representing supported interface types.
    SupportedIftypes = 32,
    /// Attribute representing interface combinations.
    InterfaceCombinations = 120,
    /// Attribute representing software interface types.
    SoftwareIftypes = 121,
}

/// Implement the `NlAttrType` trait for `Nl80211Attribute` to use it as a generic netlink attribute type.
impl neli::consts::genl::NlAttrType for Nl80211Attribute {}

/// Enum representing various nl80211 interface types.
#[neli::neli_enum(serialized_type = "u16")]
pub enum Nl80211Iftype {
    /// Unspecified interface type.
    IftypeUnspecified = 0,
    /// Ad-hoc network interface type.
    IftypeAdhoc = 1,
    /// Station (client) network interface type.
    IftypeStation = 2,
    /// Access point (AP) network interface type.
    IftypeAp = 3,
    /// VLAN interface type for AP.
    IftypeApVlan = 4,
    /// Wireless Distribution System (WDS) interface type.
    IftypeWds = 5,
    /// Monitor interface type.
    IftypeMonitor = 6,
    /// Mesh point interface type.
    IftypeMeshPoint = 7,
    /// P2P client interface type.
    IftypeP2pClient = 8,
    /// P2P group owner interface type.
    IftypeP2pGo = 9,
    /// P2P device interface type.
    IftypeP2pDevice = 10,
    /// Outside Context of a BSS (OCB) interface type.
    IftypeOcb = 11,
    /// NAN (Neighbor Awareness Networking) interface type.
    IftypeNan = 12,
    /// Number of interface types.
    NumIftypes = 13,
    /// Maximum interface type value.
    IftypeMax = 12,
}

/// Implement the `NlAttrType` trait for `Nl80211Iftype` to use it as a generic netlink attribute type.
impl neli::consts::genl::NlAttrType for Nl80211Iftype {}
