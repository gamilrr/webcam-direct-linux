//! This module contains the implementation to handle the dnsmasq process as a child process.
use super::process_hdl::ProcessHdlOps;
use crate::error::Result;
use std::process::Command;
mod ip_range;

pub use ip_range::DhcpIpRange;

#[cfg(test)]
use mockall::automock;

/// Trait for DHCP server control.
#[cfg_attr(test, automock)]
pub trait DhcpServerCtl {
    /// Starts the DHCP server with the specified interface name and IP range.
    ///
    /// # Arguments
    ///
    /// * `iw_name` - The name of the network interface.
    /// * `ip_range` - A tuple containing the start and end IP addresses for the DHCP range.
    ///
    /// # Errors
    ///
    /// Returns an error if the process fails to start.
    fn start(&mut self, iw_name: &str, ip_range: DhcpIpRange) -> Result<()>;

    /// Stops the DHCP server.
    ///
    /// # Errors
    ///
    /// Returns an error if the process fails to stop.
    fn stop(&mut self) -> Result<()>;
}

/// Struct to control the dnsmasq process.
pub struct DnsmasqProc<T: ProcessHdlOps> {
    process: T,
}

impl<T: ProcessHdlOps> DnsmasqProc<T> {
    /// Creates a new `DnsmasqCtl` instance with the specified process handler.
    ///
    /// # Arguments
    ///
    /// * `process` - An instance of a type that implements the `ProcessOps` trait.
    ///
    /// # Returns
    ///
    /// A new instance of `DnsmasqCtl`.
    pub fn new(process: T) -> Self {
        Self { process }
    }
}

impl<T: ProcessHdlOps> DhcpServerCtl for DnsmasqProc<T> {
    /// Starts the dnsmasq process with the specified interface name and IP range.
    ///
    /// # Arguments
    ///
    /// * `iw_name` - The name of the network interface.
    /// * `ip_range` - A tuple containing the start and end IP addresses for the DHCP range.
    ///
    /// # Errors
    ///
    /// Returns an error if the process fails to start.
    fn start(&mut self, iw_name: &str, ip_range: DhcpIpRange) -> Result<()> {
        let ip_range =
            format!("{},{}", ip_range.get_start_ip(), ip_range.get_end_ip());
        let mut cmd = Command::new("dnsmasq");
        cmd.arg("-p")
            .arg("0")
            .arg("-i")
            .arg(iw_name)
            .arg("-F")
            .arg(ip_range)
            .arg("-n")
            .arg("-d");

        self.process.spawn(&mut cmd)?;
        Ok(())
    }

    /// Stops the dnsmasq process.
    ///
    /// # Errors
    ///
    /// Returns an error if the process fails to stop.
    fn stop(&mut self) -> Result<()> {
        self.process.kill()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::super::process_hdl::MockProcessHdlOps;
    use super::DhcpIpRange;
    use super::*;

    #[test]
    fn test_start_dnsmasq() {
        let mut mock_process = MockProcessHdlOps::new();
        let iw_name = "test_interface";
        let ip_range =
            DhcpIpRange::new("192.168.1.100", "192.168.1.200").unwrap();

        // Expect the spawn method to be called with the correct command
        mock_process
            .expect_spawn()
            .withf(move |cmd: &Command| {
                cmd.get_program() == "dnsmasq"
                    && cmd.get_args().collect::<Vec<_>>()
                        == vec![
                            "-p",
                            "0",
                            "-i",
                            iw_name,
                            "-F",
                            "192.168.1.100,192.168.1.200",
                            "-n",
                            "-d",
                        ]
            })
            .returning(|_| Ok(()));

        let mut dnsmasq_ctl = DnsmasqProc::new(mock_process);

        // Test starting the dnsmasq process
        let result = dnsmasq_ctl.start(iw_name, ip_range);
        assert!(result.is_ok());
    }

    #[test]
    fn test_start_dnsmasq_spawn_fails() {
        let mut mock_process = MockProcessHdlOps::new();
        let iw_name = "test_interface";
        let ip_range =
            DhcpIpRange::new("192.168.1.100", "192.168.1.200").unwrap();

        // Expect the spawn method to be called and return an error
        mock_process
            .expect_spawn()
            .returning(|_| Err(anyhow::anyhow!("Failed to spawn process")));

        let mut dnsmasq_ctl = DnsmasqProc::new(mock_process);

        // Test starting the dnsmasq process
        let result = dnsmasq_ctl.start(iw_name, ip_range);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().to_string(), "Failed to spawn process");
    }

    #[test]
    fn test_stop_dnsmasq() {
        let mut mock_process = MockProcessHdlOps::new();

        // Expect the kill method to be called and return Ok
        mock_process.expect_kill().returning(|| Ok(()));

        let mut dnsmasq_ctl = DnsmasqProc::new(mock_process);

        // Test stopping the dnsmasq process
        let result = dnsmasq_ctl.stop();
        assert!(result.is_ok());
    }

    #[test]
    fn test_stop_dnsmasq_kill_fails() {
        let mut mock_process = MockProcessHdlOps::new();

        // Expect the kill method to be called and return an error
        mock_process
            .expect_kill()
            .returning(|| Err(anyhow::anyhow!("Failed to kill process")));

        let mut dnsmasq_ctl = DnsmasqProc::new(mock_process);

        // Test stopping the dnsmasq process
        let result = dnsmasq_ctl.stop();
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().to_string(), "Failed to kill process");
    }
}
