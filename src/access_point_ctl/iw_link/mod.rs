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
    /// Creates a new link with the given name.
    ///
    /// # Arguments
    ///
    /// * `name` - A string slice that holds the name of the new link.
    ///
    /// # Returns
    ///
    /// * `Result<()>` - Returns an empty `Result` on success, or an error on failure.
    fn create_with_name(&mut self, name: &str) -> Result<()>;

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
}

/// Struct representing a wireless link.
pub struct IwLink<T: WirelessDriver> {
    driver: T,
    current_addr: Option<String>,
    if_idx: Option<InterfaceIndex>,
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
    pub fn with_driver(driver: T) -> Self {
        Self { driver, current_addr: None, if_idx: None }
    }
}

impl<T: WirelessDriver> IwLinkHandler for IwLink<T> {
    fn create_with_name(&mut self, name: &str) -> Result<()> {
        if self.if_idx.is_some() {
            warn!("Interface '{}' already exists", name);
            return Err(anyhow!("Interface already exists"));
        }

        if let Some(wiphy_idx) = self.driver.get_ap_wiphy_indx()? {
            info!(
                "Creating new link with name: {} and wiphy index: {}",
                name, wiphy_idx
            );
            self.if_idx = self.driver.create_new_link(name, wiphy_idx)?;
        } else {
            error!("Failed to get wiphy index, the wireless driver does not support AP mode");
            return Err(anyhow!("Failed to get wiphy index, the wireless driver does not support AP mode"));
        }

        Ok(())
    }

    fn add_ipv4_addr(&mut self, addr: &str) -> Result<()> {
        if self.current_addr.is_some() {
            warn!("Address already exists on interface");
            return Err(anyhow!("Address already exists on interface"));
        }

        if let Some(if_idx) = self.if_idx {
            info!("Adding IPv4 address: {} to interface: {}", addr, if_idx);
            self.driver.add_ipv4_addr(if_idx, addr)?;
            self.current_addr = Some(addr.to_string());
        } else {
            error!("Call create_new_link before adding an address");
            return Err(anyhow!(
                "You must create a link before adding an address"
            ));
        }

        Ok(())
    }
}

impl<T: WirelessDriver> Drop for IwLink<T> {
    /// Deletes the wireless link when the `IwLink` object is dropped.
    fn drop(&mut self) {
        if let Some(if_idx) = self.if_idx {
            info!("Deleting link with index: {}", if_idx);
            if let Err(error) = self.driver.delete_link(if_idx) {
                error!(
                    "Failed to delete link with index: {}, error: {}",
                    if_idx, error
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use mockall::predicate::eq;
    use wdev_drv::MockWirelessDriver;

    use super::*;
    use crate::error::Result;

    fn init() {
        let _ = env_logger::builder().is_test(true).try_init();
    }

    #[test]
    fn test_create_new_link_error() -> Result<()> {
        init();
        let mut mock_driver = MockWirelessDriver::new();

        mock_driver
            .expect_get_ap_wiphy_indx()
            .returning(|| Err(anyhow!("Error")));

        let mut iw_link = IwLink::with_driver(mock_driver);
        let result = iw_link.create_with_name("test");

        assert!(result.is_err());
        Ok(())
    }

    #[test]
    fn test_create_new_link_success() -> Result<()> {
        init();
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

        let mut iw_link = IwLink::with_driver(mock_driver);
        let result = iw_link.create_with_name("test");

        assert!(result.is_ok());
        assert_eq!(iw_link.if_idx, Some(InterfaceIndex(1)));
        Ok(())
    }

    #[test]
    fn test_add_ipv4_addr_success() -> Result<()> {
        init();
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
            if_idx: Some(InterfaceIndex(1)),
        };

        let result = iw_link.add_ipv4_addr("192.168.1.1");

        assert!(result.is_ok());
        assert_eq!(iw_link.current_addr, Some("192.168.1.1".to_string()));
        Ok(())
    }

    #[test]
    fn test_add_ipv4_addr_error() -> Result<()> {
        init();
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
            if_idx: Some(InterfaceIndex(1)),
        };

        let result = iw_link.add_ipv4_addr("192.168.1.1");

        assert!(result.is_err());
        assert!(iw_link.current_addr.is_none());
        Ok(())
    }

    #[test]
    fn test_create_new_link_twice() -> Result<()> {
        init();
        let mut mock_driver = MockWirelessDriver::new();

        mock_driver
            .expect_get_ap_wiphy_indx()
            .returning(|| Ok(Some(InterfaceIndex(1))));

        mock_driver
            .expect_create_new_link()
            .with(eq("test"), eq(InterfaceIndex(1)))
            .returning(|_, _| Ok(Some(InterfaceIndex(1))))
            .times(1);

        //set expectation for delete_link which is called when the IwLink object is dropped
        mock_driver
            .expect_delete_link()
            .with(eq(InterfaceIndex(1)))
            .returning(|_| Ok(()))
            .times(1);

        let mut iw_link = IwLink::with_driver(mock_driver);
        let result1 = iw_link.create_with_name("test");
        let result2 = iw_link.create_with_name("test");

        assert!(result1.is_ok());
        assert!(result2.is_err());
        Ok(())
    }
}
