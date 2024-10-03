//! This module provides functionality to manage WiFi operations using Hostapd and WPA control interfaces.
//!
//! The `WifiManager` struct and the `WifiManagerCtl` trait define methods to configure, pause, resume, change credentials, and turn off the WiFi manager.

mod file_hdl;
mod hostapd_proc;
mod wpa_ctl;

//export the `HostapdProcCtl` trait and `WifiCredentials` struct from the `hostapd_proc` module.
pub use file_hdl::FileHdl;
pub use hostapd_proc::{HostapdProc, HostapdProcCtl, WifiCredentials};
pub use wpa_ctl::WpaCtl;

use crate::error::Result;
use anyhow::anyhow;
use log::info;
use wpa_ctl::WpaCtlClientOps;

#[cfg(test)]
use mockall::automock;

/// Trait defining operations for WiFi manager control.
#[cfg_attr(test, automock)]
pub trait WifiManagerCtl {
    /// Configures the WiFi manager with the given credentials.
    ///
    /// # Arguments
    ///
    /// * `creds` - WiFi credentials.
    ///
    /// # Errors
    ///
    /// Returns an error if the configuration fails.
    fn configure(&mut self, creds: WifiCredentials) -> Result<()>;

    /// Pauses the WiFi broadcast, the SSID will be not accessible after this call.
    ///
    /// # Errors
    ///
    /// Returns an error if pausing fails.
    fn pause(&mut self) -> Result<()>;

    /// Resumes the WiFi broadcast, the SSID will be accessible after this call.
    ///
    /// # Errors
    ///
    /// Returns an error if resuming fails.
    fn resume(&mut self) -> Result<()>;

    /// Changes the WiFi credentials.
    ///
    /// # Arguments
    ///
    /// * `creds` - New WiFi credentials.
    ///
    /// # Errors
    ///
    /// Returns an error if changing credentials fails.
    fn change_creds(&mut self, creds: WifiCredentials) -> Result<()>;

    /// Retrieves the interface name for the Wi-Fi device.
    ///
    /// # Returns
    ///
    /// * `&str` - A string slice that holds the interface name.
    fn get_iw_name(&self) -> &str;

    /// Turns off the WiFi manager.
    ///
    /// # Errors
    ///
    /// Returns an error if turning off fails.
    fn turnoff(&mut self) -> Result<()>;
}

/// Struct representing the WiFi manager.
pub struct WifiManager<P, C>
where
    P: HostapdProcCtl,
    C: WpaCtlClientOps,
{
    hostapd: P,
    wpa_ctl: C,
}

impl<P: HostapdProcCtl, C: WpaCtlClientOps> WifiManager<P, C> {
    /// Creates a new `WifiManager` instance.
    ///
    /// # Arguments
    ///
    /// * `hostapd` - Hostapd process control.
    /// * `wpa_ctl` - WPA control client.
    pub fn new(hostapd: P, wpa_ctl: C) -> Self {
        Self { hostapd, wpa_ctl }
    }
}

impl<P: HostapdProcCtl, C: WpaCtlClientOps> WifiManagerCtl
    for WifiManager<P, C>
{
    fn configure(&mut self, creds: WifiCredentials) -> Result<()> {
        let iw_name = self.wpa_ctl.get_iw_name();
        let control_dir = self.wpa_ctl.get_control_dir();

        let control_dir =
            control_dir.to_str().ok_or(anyhow!("Invalid control directory"))?;

        self.hostapd.start(creds, iw_name, control_dir)?;

        // Try to connect during 5 seconds to the AP process
        // This has to wait until the process is ready to accept connections
        let mut tries = 0;
        while tries < 5 {
            if self.wpa_ctl.connect().is_ok() {
                info!("Connected to WPA control socket");
                break;
            }
            tries += 1;
            std::thread::sleep(std::time::Duration::from_secs(1));
        }

        Ok(())
    }

    fn resume(&mut self) -> Result<()> {
        self.wpa_ctl.enable()?;
        Ok(())
    }

    fn pause(&mut self) -> Result<()> {
        self.wpa_ctl.disable()?;
        Ok(())
    }

    fn change_creds(&mut self, creds: WifiCredentials) -> Result<()> {
        self.wpa_ctl.set_ssid(&creds.ssid)?;
        self.wpa_ctl.set_password(&creds.password)?;
        self.wpa_ctl.reload()?;
        Ok(())
    }

    fn get_iw_name(&self) -> &str {
        self.wpa_ctl.get_iw_name()
    }

    fn turnoff(&mut self) -> Result<()> {
        self.hostapd.stop()?;
        self.wpa_ctl.disconnect()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hostapd_proc::MockHostapdProcCtl;
    use std::path::PathBuf;
    use wpa_ctl::MockWpaCtlClientOps;

    #[test]
    fn test_config() {
        let mut mock_hostapd = MockHostapdProcCtl::new();
        let mut mock_wpa_ctl = MockWpaCtlClientOps::new();

        mock_wpa_ctl.expect_get_iw_name().return_const("wlan0".to_string());
        mock_wpa_ctl
            .expect_get_control_dir()
            .return_const(PathBuf::from("/tmp/wpa_supplicant"));
        mock_hostapd.expect_start().returning(|_, _, _| Ok(()));
        mock_wpa_ctl.expect_connect().returning(|| Ok(()));

        let mut wifi_manager = WifiManager::new(mock_hostapd, mock_wpa_ctl);
        let creds = WifiCredentials {
            ssid: "test_ssid".to_string(),
            password: "test_password".to_string(),
        };

        assert!(wifi_manager.configure(creds).is_ok());
    }

    #[test]
    fn test_resume() {
        let mock_hostapd = MockHostapdProcCtl::new();
        let mut mock_wpa_ctl = MockWpaCtlClientOps::new();

        mock_wpa_ctl.expect_enable().returning(|| Ok(()));

        let mut wifi_manager = WifiManager::new(mock_hostapd, mock_wpa_ctl);

        assert!(wifi_manager.resume().is_ok());
    }

    #[test]
    fn test_pause() {
        let mock_hostapd = MockHostapdProcCtl::new();
        let mut mock_wpa_ctl = MockWpaCtlClientOps::new();

        mock_wpa_ctl.expect_disable().returning(|| Ok(()));

        let mut wifi_manager = WifiManager::new(mock_hostapd, mock_wpa_ctl);

        assert!(wifi_manager.pause().is_ok());
    }

    #[test]
    fn test_change_creds() {
        let mock_hostapd = MockHostapdProcCtl::new();
        let mut mock_wpa_ctl = MockWpaCtlClientOps::new();

        mock_wpa_ctl.expect_set_ssid().returning(|_| Ok(()));
        mock_wpa_ctl.expect_set_password().returning(|_| Ok(()));
        mock_wpa_ctl.expect_reload().returning(|| Ok(()));

        let mut wifi_manager = WifiManager::new(mock_hostapd, mock_wpa_ctl);
        let creds = WifiCredentials {
            ssid: "new_ssid".to_string(),
            password: "new_password".to_string(),
        };

        assert!(wifi_manager.change_creds(creds).is_ok());
    }

    #[test]
    fn test_turnoff() {
        let mut mock_hostapd = MockHostapdProcCtl::new();
        let mut mock_wpa_ctl = MockWpaCtlClientOps::new();

        mock_hostapd.expect_stop().returning(|| Ok(()));
        mock_wpa_ctl.expect_disconnect().returning(|| Ok(()));

        let mut wifi_manager = WifiManager::new(mock_hostapd, mock_wpa_ctl);

        assert!(wifi_manager.turnoff().is_ok());
    }

    #[test]
    fn test_get_iw_name() {
        let mock_hostapd = MockHostapdProcCtl::new();
        let mut mock_wpa_ctl = MockWpaCtlClientOps::new();

        mock_wpa_ctl.expect_get_iw_name().return_const("wlan0".to_string());

        let wifi_manager = WifiManager::new(mock_hostapd, mock_wpa_ctl);

        assert_eq!(wifi_manager.get_iw_name(), "wlan0");
    }
}
