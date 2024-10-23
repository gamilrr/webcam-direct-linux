//! This module defines the `WirelessDriver` trait and its associated types.
//! The `WirelessDriver` trait serves as an interface for underlying wireless drivers,
//! which can be implemented using netlink, dbus, or any other wireless driver mechanism.

// Import the `interface_index` module, which likely contains functionality related to network interface indices.
mod interface_index;

// Import the `nl80211_drv` module, which likely contains an implementation of the `WirelessDriver` trait using the nl80211 protocol.
mod nl80211_drv;

// Re-export the `InterfaceIndex` type from the `interface_index` module, making it publicly accessible.
pub use interface_index::InterfaceIndex;

// Re-export the `Nl80211Driver` type from the `nl80211_drv` module, making it publicly accessible.
pub use nl80211_drv::Nl80211Driver;

#[cfg(test)]
use mockall::automock;

use crate::error::Result;

/// This trait serves as an interface for the underlying wireless driver.
/// Implementations of this trait can use netlink, dbus, or any other wireless driver mechanism.
#[cfg_attr(test, automock)]
pub trait WirelessDriver {
    /// Returns the phy index of the physical interface that supports AP mode.
    /// Returns `None` if no such phy index is found.
    fn get_ap_wiphy_indx(&self) -> Result<Option<InterfaceIndex>>;

    /// Creates a new link with the given name and phy index.
    /// Returns the interface index of the newly created link, or `None` if the creation fails.
    fn create_new_link(
        &self, name: &str, phy_idx: InterfaceIndex,
    ) -> Result<Option<InterfaceIndex>>;

    /// Adds an IPv4 address to the given interface with a prefix length of 24.
    fn add_ipv4_addr(&self, ifindex: InterfaceIndex, addr: &str) -> Result<()>;

    /// Deletes the link with the given interface index.
    fn delete_link(&self, ifindex: InterfaceIndex) -> Result<()>;

}
