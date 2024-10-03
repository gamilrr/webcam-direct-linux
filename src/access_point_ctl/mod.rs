//! This module provides control over the access point functionalities, including
//! configuration, starting/stopping WiFi, and managing DHCP server.

pub mod dhcp_server;
pub mod iw_link;
pub mod process_hdl;
pub mod wifi_manager;

use anyhow::anyhow;
use dhcp_server::DhcpIpRange;
use dhcp_server::DhcpServerCtl;
use iw_link::IwLinkHandler;
use log::{error, info, warn};
use wifi_manager::WifiCredentials;
use wifi_manager::WifiManagerCtl;

use crate::error::Result;

/// Trait defining the control operations for an access point.
pub trait AccessPointCtl {
    /// Configures the access point with the given WiFi credentials and route IP.
    ///
    /// # Arguments
    ///
    /// * `creds` - WiFi credentials to configure the access point.
    /// * `route_ip` - IP address to route the access point.
    ///
    /// # Returns
    ///
    /// * `Result<()>` - Result indicating success or failure.
    fn configure(
        &mut self, creds: WifiCredentials, route_ip: &str,
    ) -> Result<()>;

    /// Starts the WiFi broadcast.
    ///
    /// # Returns
    ///
    /// * `Result<()>` - Result indicating success or failure.
    fn start_wifi(&mut self) -> Result<()>;

    /// Stops the WiFi broadcast.
    ///
    /// # Returns
    ///
    /// * `Result<()>` - Result indicating success or failure.
    fn stop_wifi(&mut self) -> Result<()>;

    /// Starts the DHCP server with the given IP range.
    ///
    /// # Arguments
    ///
    /// * `ip_range` - IP range for the DHCP server.
    ///
    /// # Returns
    ///
    /// * `Result<()>` - Result indicating success or failure.
    fn start_dhcp_server(&mut self, ip_range: DhcpIpRange) -> Result<()>;

    /// Sets the WiFi credentials.
    ///
    /// # Arguments
    ///
    /// * `creds` - New WiFi credentials.
    ///
    /// # Returns
    ///
    /// * `Result<()>` - Result indicating success or failure.
    fn set_creds(&mut self, creds: WifiCredentials) -> Result<()>;

    /// Gets the current WiFi credentials.
    ///
    /// # Returns
    ///
    /// * `Option<WifiCredentials>` - Current WiFi credentials if set.
    fn get_creds(&mut self) -> Option<WifiCredentials>;
}

/// Struct representing the access point controller.
pub struct ApController<I, D, W>
where
    I: IwLinkHandler,
    D: DhcpServerCtl,
    W: WifiManagerCtl,
{
    iw_link: I,
    dhcp_server: D,
    wifi_manager: W,
    if_name: Option<String>,
    creds: Option<WifiCredentials>,
}

impl<I: IwLinkHandler, D: DhcpServerCtl, W: WifiManagerCtl>
    ApController<I, D, W>
{
    /// Creates a new instance of `ApController`.
    ///
    /// # Arguments
    ///
    /// * `iw_link` - Handler for the wireless link.
    /// * `dhcp_server` - Controller for the DHCP server.
    /// * `wifi_manager` - Controller for the WiFi manager.
    ///
    /// # Returns
    ///
    /// * `Self` - New instance of `ApController`.
    pub fn new(iw_link: I, dhcp_server: D, wifi_manager: W) -> Self {
        Self { iw_link, wifi_manager, dhcp_server, if_name: None, creds: None }
    }
}

impl<I: IwLinkHandler, D: DhcpServerCtl, W: WifiManagerCtl> AccessPointCtl
    for ApController<I, D, W>
{
    fn configure(
        &mut self, creds: WifiCredentials, route_ip: &str,
    ) -> Result<()> {
        let if_name = self.wifi_manager.get_iw_name().to_string();

        info!("Configuring access point with name {}", if_name);

        if let Err(error) = self.iw_link.create_with_name(&if_name) {
            error!(
                "Failed to create wireless interface with name {}, error {}",
                if_name, error
            );
            return Err(error);
        };

        // In ubuntu the network manager will try to take control of the interface causing a
        // hostapd to fail to start. We need to disable the network manager for the interface
        let _output = std::process::Command::new("nmcli")
            .arg("device")
            .arg("set")
            .arg(&if_name)
            .arg("managed")
            .arg("no")
            .output()?;

        if let Err(error) = self.iw_link.add_ipv4_addr(route_ip) {
            error!(
                "Failed to add IP address {} to interface {}, error {}",
                route_ip, if_name, error
            );
            return Err(error);
        }

        if let Err(error) = self.wifi_manager.configure(creds.clone()) {
            error!(
                "Failed to configure wifi manager with credentials {:?}, error {}",
                creds, error
            );
            return Err(error);
        }

        info!("Access Point configured successfully, pausing the wifi broadcast for now");
        if let Err(error) = self.wifi_manager.pause() {
            warn!("Failed to pause the wifi broadcast, error {}", error);
        }

        self.creds = Some(creds);
        self.if_name = Some(if_name.to_string());

        Ok(())
    }

    fn start_wifi(&mut self) -> Result<()> {
        info!("Resuming the wifi broadcast");
        if let Err(error) = self.wifi_manager.resume() {
            error!("Failed to resume the wifi broadcast, error {}", error);
            return Err(error);
        }
        Ok(())
    }

    fn stop_wifi(&mut self) -> Result<()> {
        info!("Disabling the wifi broadcast");
        if let Err(error) = self.wifi_manager.pause() {
            error!("Failed to disable the wifi broadcast, error {}", error);
            return Err(error);
        }

        Ok(())
    }

    fn set_creds(&mut self, creds: WifiCredentials) -> Result<()> {
        info!("Changing wifi credentials to {:?}", creds);
        if let Err(error) = self.wifi_manager.change_creds(creds.clone()) {
            error!("Failed to change wifi credentials, error {}", error);
            return Err(error);
        }

        self.creds = Some(creds);
        Ok(())
    }

    fn get_creds(&mut self) -> Option<WifiCredentials> {
        self.creds.clone()
    }

    fn start_dhcp_server(&mut self, ip_range: DhcpIpRange) -> Result<()> {
        info!("Starting DHCP server with IP range {:?}", ip_range);

        if let Some(if_name) = &self.if_name {
            if let Err(error) = self.dhcp_server.start(if_name, ip_range) {
                error!("Failed to start DHCP server, error {}", error);
                return Err(error);
            }
        } else {
            error!("The wireless interface not available");
            return Err(anyhow!("The wireless interface name not available"));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use dhcp_server::MockDhcpServerCtl;
    use iw_link::MockIwLinkHandler;
    use mockall::predicate::*;
    use wifi_manager::MockWifiManagerCtl;

    use super::*;

    #[test]
    fn test_configure_success() {
        let mut mock_iw_link = MockIwLinkHandler::new();
        let mock_dhcp_server = MockDhcpServerCtl::new();
        let mut mock_wifi_manager = MockWifiManagerCtl::new();

        mock_wifi_manager
            .expect_get_iw_name()
            .return_const("wlan0".to_string());
        mock_iw_link
            .expect_create_with_name()
            .with(eq("wlan0"))
            .returning(|_| Ok(()));
        mock_iw_link
            .expect_add_ipv4_addr()
            .with(eq("192.168.1.1"))
            .returning(|_| Ok(()));
        mock_wifi_manager
            .expect_configure()
            .withf(|creds| {
                creds.ssid == "test_ssid" && creds.password == "test_password"
            })
            .returning(|_| Ok(()));
        mock_wifi_manager.expect_pause().returning(|| Ok(()));

        let mut controller = ApController::new(
            mock_iw_link,
            mock_dhcp_server,
            mock_wifi_manager,
        );
        let creds = WifiCredentials {
            ssid: "test_ssid".to_string(),
            password: "test_password".to_string(),
        };

        let result = controller.configure(creds, "192.168.1.1");
        assert!(result.is_ok());
    }

    #[test]
    fn test_start_wifi_success() {
        let mock_iw_link = MockIwLinkHandler::new();
        let mock_dhcp_server = MockDhcpServerCtl::new();
        let mut mock_wifi_manager = MockWifiManagerCtl::new();

        mock_wifi_manager.expect_resume().returning(|| Ok(()));

        let mut controller = ApController::new(
            mock_iw_link,
            mock_dhcp_server,
            mock_wifi_manager,
        );

        let result = controller.start_wifi();
        assert!(result.is_ok());
    }

    #[test]
    fn test_stop_wifi_success() {
        let mock_iw_link = MockIwLinkHandler::new();
        let mock_dhcp_server = MockDhcpServerCtl::new();
        let mut mock_wifi_manager = MockWifiManagerCtl::new();

        mock_wifi_manager.expect_pause().returning(|| Ok(()));

        let mut controller = ApController::new(
            mock_iw_link,
            mock_dhcp_server,
            mock_wifi_manager,
        );

        let result = controller.stop_wifi();
        assert!(result.is_ok());
    }

    #[test]
    fn test_set_creds_success() {
        let mock_iw_link = MockIwLinkHandler::new();
        let mock_dhcp_server = MockDhcpServerCtl::new();
        let mut mock_wifi_manager = MockWifiManagerCtl::new();

        mock_wifi_manager
            .expect_change_creds()
            .withf(|creds| {
                creds.ssid == "new_ssid" && creds.password == "new_password"
            })
            .returning(|_| Ok(()));

        let mut controller = ApController::new(
            mock_iw_link,
            mock_dhcp_server,
            mock_wifi_manager,
        );
        let creds = WifiCredentials {
            ssid: "new_ssid".to_string(),
            password: "new_password".to_string(),
        };

        let result = controller.set_creds(creds);
        assert!(result.is_ok());
    }

    #[test]
    fn test_get_creds() {
        let mock_iw_link = MockIwLinkHandler::new();
        let mock_dhcp_server = MockDhcpServerCtl::new();
        let mock_wifi_manager = MockWifiManagerCtl::new();

        let mut controller = ApController::new(
            mock_iw_link,
            mock_dhcp_server,
            mock_wifi_manager,
        );
        let creds = WifiCredentials {
            ssid: "test_ssid".to_string(),
            password: "test_password".to_string(),
        };
        controller.creds = Some(creds.clone());

        let result = controller.get_creds();
        assert_eq!(result, Some(creds));
    }

    #[test]
    fn test_start_dhcp_server_success() {
        let mock_iw_link = MockIwLinkHandler::new();
        let mut mock_dhcp_server = MockDhcpServerCtl::new();
        let mock_wifi_manager = MockWifiManagerCtl::new();

        mock_dhcp_server
            .expect_start()
            .withf(|if_name, ip_range| {
                if_name == "wlan0"
                    && ip_range.get_start_ip() == "192.168.1.100"
                    && ip_range.get_end_ip() == "192.168.1.200"
            })
            .returning(|_, _| Ok(()));

        let mut controller = ApController::new(
            mock_iw_link,
            mock_dhcp_server,
            mock_wifi_manager,
        );
        controller.if_name = Some("wlan0".to_string());

        let ip_range =
            DhcpIpRange::new("192.168.1.100", "192.168.1.200").unwrap();

        let result = controller.start_dhcp_server(ip_range);
        assert!(result.is_ok());
    }
}
