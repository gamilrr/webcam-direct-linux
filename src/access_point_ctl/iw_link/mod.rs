//! This module provides functionality for managing wireless links using a wireless driver.
//!
//! The main components are:
//! - `IwLinkHandler` trait: Defines the interface for adding an IPv4 address.
//! - `IwLink` struct: Represents a wireless link and provides methods to manage it.
//! - `wdev_drv` module: Contains the wireless driver interface and related types.

//Re-export the `WirelessDriver` trait and related types from the `wdev_drv` module.
pub mod wdev_drv;

use crate::error::Result;
use anyhow::anyhow;
use log::{error, info, warn};
use wdev_drv::{InterfaceIndex, WirelessDriver};

#[cfg(test)]
use mockall::automock;

/// Trait defining the interface for handling wireless links.
#[cfg_attr(test, automock)]
pub trait IwLinkHandler {
    /// Adds an IPv4 address to the wireless link.
    ///
    /// # Arguments
    ///
    /// * `addr` - A string slice that holds the IPv4 address to be added.
    ///
    /// # Errors
    ///
    /// Returns an error if the address could not be added.
    fn add_ipv4_addr(&mut self, addr: &str) -> Result<()>;

    /// Returns the name of the interface.
    ///
    /// # Returns
    ///
    /// A string slice that holds the name of the interface.
    fn get_if_name(&self) -> &str;
}

/// Struct representing a wireless link.
pub struct IwLink<T: WirelessDriver> {
    driver: T,
    if_name: String,
    current_addr: Option<String>,
    if_idx: InterfaceIndex,
}

impl<T: WirelessDriver> IwLink<T> {
    /// Creates a new `IwLink` object.
    ///
    /// # Arguments
    ///
    /// * `driver` - The wireless driver to be used.
    ///
    /// # Errors
    ///
    /// Returns an error if the link could not be created.
    ///
    /// # Examples
    ///
    /// ```
    /// use crate::IwLink;
    /// use crate::wdev_drv::MockWirelessDriver;
    ///
    /// let mock_driver = MockWirelessDriver::new();
    /// let iw_link = IwLink::with_driver(mock_driver);
    /// ```
    pub fn new(driver: T, if_name: &str) -> Result<Self> {
        let wiphy_idx = match driver.get_ap_wiphy_indx()? {
            Some(idx) => idx,
            None => {
                error!("Failed to get wiphy index, the wireless driver does not support AP mode");
                return Err(anyhow!("Failed to get wiphy index, the wireless driver does not support AP mode"));
            }
        };

        let if_idx = match driver.create_new_link(if_name, wiphy_idx)? {
            Some(idx) => idx,
            None => {
                error!("Failed to create new link");
                return Err(anyhow!("Failed to create new link"));
            }
        };

        Ok(Self {
            driver,
            if_name: if_name.to_owned(),
            current_addr: None,
            if_idx,
        })
    }
}

impl<T: WirelessDriver> IwLinkHandler for IwLink<T> {
    fn add_ipv4_addr(&mut self, addr: &str) -> Result<()> {
        if self.current_addr.is_some() {
            warn!("Address already exists on interface");
            return Err(anyhow!("Address already exists on interface"));
        }

        info!("Adding IPv4 address: {} to interface: {}", addr, self.if_idx);
        self.driver.add_ipv4_addr(self.if_idx, addr)?;
        self.current_addr = Some(addr.to_string());

        Ok(())
    }

    fn get_if_name(&self) -> &str {
        &self.if_name
    }
}

impl<T: WirelessDriver> Drop for IwLink<T> {
    /// Deletes the wireless link when the `IwLink` object is dropped.
    fn drop(&mut self) {
        info!("Deleting link with index: {}", self.if_idx);
        if let Err(error) = self.driver.delete_link(self.if_idx) {
            error!(
                "Failed to delete link with index: {}, error: {}",
                self.if_idx, error
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use mockall::predicate::eq;
    use wdev_drv::MockWirelessDriver;

    use super::*;
    use crate::error::Result;

    fn init_logger() {
        let _ = env_logger::builder().is_test(true).try_init();
    }

    #[test]
    fn test_create_new_link_error() -> Result<()> {
        init_logger();
        let mut mock_driver = MockWirelessDriver::new();

        mock_driver
            .expect_get_ap_wiphy_indx()
            .returning(|| Err(anyhow!("Error")));

        let iw_link = IwLink::new(mock_driver, "test");

        assert!(iw_link.is_err());
        Ok(())
    }

    #[test]
    fn test_create_new_link_success() -> Result<()> {
        init_logger();
        let mut mock_driver = MockWirelessDriver::new();

        mock_driver
            .expect_get_ap_wiphy_indx()
            .returning(|| Ok(Some(InterfaceIndex(1))));

        mock_driver
            .expect_create_new_link()
            .with(eq("test"), eq(InterfaceIndex(1)))
            .returning(|_, _| Ok(Some(InterfaceIndex(1))));

        mock_driver
            .expect_delete_link()
            .with(eq(InterfaceIndex(1)))
            .returning(|_| Ok(()))
            .times(1);

        let iw_link = IwLink::new(mock_driver, "test");

        assert!(iw_link.is_ok());
        assert_eq!(iw_link.unwrap().if_idx, InterfaceIndex(1));
        Ok(())
    }

    #[test]
    fn test_add_ipv4_addr_success() -> Result<()> {
        init_logger();
        let mut mock_driver = MockWirelessDriver::new();

        mock_driver
            .expect_add_ipv4_addr()
            .with(eq(InterfaceIndex(1)), eq("192.168.1.1"))
            .returning(|_, _| Ok(()));

        mock_driver
            .expect_delete_link()
            .with(eq(InterfaceIndex(1)))
            .returning(|_| Ok(()))
            .times(1);

        let mut iw_link = IwLink {
            driver: mock_driver,
            current_addr: None,
            if_idx: InterfaceIndex(1),
            if_name: "test".to_string(),
        };

        let result = iw_link.add_ipv4_addr("192.168.1.1");

        assert!(result.is_ok());
        assert_eq!(iw_link.current_addr, Some("192.168.1.1".to_string()));
        Ok(())
    }

    #[test]
    fn test_add_ipv4_addr_error() -> Result<()> {
        init_logger();
        let mut mock_driver = MockWirelessDriver::new();

        mock_driver
            .expect_add_ipv4_addr()
            .with(eq(InterfaceIndex(1)), eq("192.168.1.1"))
            .returning(|_, _| Err(anyhow!("Error")));

        mock_driver
            .expect_delete_link()
            .with(eq(InterfaceIndex(1)))
            .returning(|_| Ok(()))
            .times(1);

        let mut iw_link = IwLink {
            driver: mock_driver,
            current_addr: None,
            if_idx: InterfaceIndex(1),
            if_name: "test".to_string(),
        };

        let result = iw_link.add_ipv4_addr("192.168.1.1");

        assert!(result.is_err());
        assert!(iw_link.current_addr.is_none());
        Ok(())
    }

    #[test]
    fn test_drop_link() -> Result<()> {
        init_logger();
        let mut mock_driver = MockWirelessDriver::new();

        mock_driver
            .expect_delete_link()
            .with(eq(InterfaceIndex(1)))
            .returning(|_| Ok(()))
            .times(1);

        let iw_link = IwLink {
            driver: mock_driver,
            if_name: "test".to_string(),
            current_addr: None,
            if_idx: InterfaceIndex(1),
        };

        drop(iw_link); // Explicitly drop to test the Drop implementation

        Ok(())
    }

    #[test]
    fn test_add_ipv4_addr_when_addr_exists() -> Result<()> {
        init_logger();
        let mut mock_driver = MockWirelessDriver::new();

        mock_driver
            .expect_delete_link()
            .with(eq(InterfaceIndex(1)))
            .returning(|_| Ok(()))
            .times(1);

        let mut iw_link = IwLink {
            driver: mock_driver,
            if_name: "test".to_string(),
            current_addr: Some("192.168.1.1".to_string()),
            if_idx: InterfaceIndex(1),
        };

        let result = iw_link.add_ipv4_addr("192.168.1.2");

        assert!(result.is_err());
        assert_eq!(iw_link.current_addr, Some("192.168.1.1".to_string()));

        Ok(())
    }

    #[test]
    fn test_get_if_name() {
        let mut mock_driver = MockWirelessDriver::new();

        mock_driver
            .expect_delete_link()
            .with(eq(InterfaceIndex(1)))
            .returning(|_| Ok(()))
            .times(1);

        let iw_link = IwLink {
            driver: mock_driver,
            if_name: "test".to_string(),
            current_addr: Some("192.168.1.1".to_string()),
            if_idx: InterfaceIndex(1),
        };
        assert_eq!(iw_link.get_if_name(), "test");
    }
}
