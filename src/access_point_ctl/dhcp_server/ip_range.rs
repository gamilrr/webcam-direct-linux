/// This module defines the `DhcpIpRange` struct and its associated methods for managing DHCP IP ranges.
use crate::error::Result;
use anyhow::anyhow;
use std::net::Ipv4Addr;
use std::str::FromStr;

/// Represents a range of IP addresses for DHCP allocation.
#[derive(Debug, Clone)]
pub struct DhcpIpRange(String, String);

impl DhcpIpRange {
    /// Creates a new `DhcpIpRange` instance.
    ///
    /// # Arguments
    ///
    /// * `start` - A string representing the start IP address.
    /// * `end` - A string representing the end IP address.
    ///
    /// # Returns
    ///
    /// * `Result<DhcpIpRange>` - A result containing the `DhcpIpRange` instance or an error.
    ///
    /// # Errors
    ///
    /// This function will return an error if:
    /// - The start or end IP address is invalid.
    /// - The start or end IP address is a network or broadcast address.
    /// - The start or end IP address is the router's IP address.
    /// - The start and end IP addresses are not in the same /24 subnet.
    /// - The start IP address is greater than the end IP address.
    ///
    /// # Examples
    ///
    /// ```
    /// use crate::DhcpIpRange;
    /// let range = DhcpIpRange::new("192.168.1.10", "192.168.1.20").unwrap();
    /// assert_eq!(range.get_start_ip(), "192.168.1.10");
    /// assert_eq!(range.get_end_ip(), "192.168.1.20");
    /// ```
    pub fn new(start: &str, end: &str) -> Result<DhcpIpRange> {
        let start_ip = Ipv4Addr::from_str(&start)
            .map_err(|_| anyhow!("Invalid start IP address"))?;
        let end_ip = Ipv4Addr::from_str(&end)
            .map_err(|_| anyhow!("Invalid end IP address"))?;

        if start_ip.octets()[3] == 0
            || start_ip.octets()[3] == 255
            || end_ip.octets()[3] == 0
            || end_ip.octets()[3] == 255
        {
            return Err(anyhow!(
                "IP addresses cannot be the network or broadcast address"
                    .to_string()
            ));
        }

        if start_ip.octets()[3] == 1 || end_ip.octets()[3] == 1 {
            return Err(anyhow!(
                "IP addresses cannot be the router's IP address".to_string()
            ));
        }

        if start_ip.octets()[0..3] != end_ip.octets()[0..3] {
            return Err(anyhow!(
                "IP addresses are not in the same /24 subnet".to_string()
            ));
        }

        if start_ip > end_ip {
            return Err(anyhow!(
                "Start IP address must be less than or equal to end IP address"
                    .to_string()
            ));
        }

        Ok(Self(start.to_string(), end.to_string()))
    }

    /// Returns the interface IP address based on the start IP address.
    ///
    /// # Returns
    ///
    /// * `String` - The interface IP address.
    ///
    /// # Examples
    ///
    /// ```
    /// use crate::DhcpIpRange;
    /// let range = DhcpIpRange::new("192.168.1.10", "192.168.1.20").unwrap();
    /// assert_eq!(range.get_router_ip(), "192.168.1.1");
    /// ```
    pub fn get_router_ip(&self) -> String {
        let start_ip = &self.0;
        let octets: Vec<&str> = start_ip.split('.').collect();
        format!("{}.{}.{}.1", octets[0], octets[1], octets[2])
    }

    /// Returns the start IP address.
    ///
    /// # Returns
    ///
    /// * `&str` - The start IP address.
    ///
    /// # Examples
    ///
    /// ```
    /// use crate::DhcpIpRange;
    /// let range = DhcpIpRange::new("192.168.1.10", "192.168.1.20").unwrap();
    /// assert_eq!(range.get_start_ip(), "192.168.1.10");
    /// ```
    pub fn get_start_ip(&self) -> &str {
        &self.0
    }

    /// Returns the end IP address.
    ///
    /// # Returns
    ///
    /// * `&str` - The end IP address.
    ///
    /// # Examples
    ///
    /// ```
    /// use crate::DhcpIpRange;
    /// let range = DhcpIpRange::new("192.168.1.10", "192.168.1.20").unwrap();
    /// assert_eq!(range.get_end_ip(), "192.168.1.20");
    /// ```
    pub fn get_end_ip(&self) -> &str {
        &self.1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_valid_range() {
        let range = DhcpIpRange::new("192.168.1.10", "192.168.1.20");
        assert!(range.is_ok());
    }

    #[test]
    fn test_new_invalid_start_ip() {
        let range = DhcpIpRange::new("192.168.1.0", "192.168.1.20");
        assert!(range.is_err());
    }

    #[test]
    fn test_new_invalid_end_ip() {
        let range = DhcpIpRange::new("192.168.1.10", "192.168.1.255");
        assert!(range.is_err());
    }

    #[test]
    fn test_new_router_ip() {
        let range = DhcpIpRange::new("192.168.1.1", "192.168.1.20");
        assert!(range.is_err());
    }

    #[test]
    fn test_new_different_subnets() {
        let range = DhcpIpRange::new("192.168.1.10", "192.168.2.20");
        assert!(range.is_err());
    }

    #[test]
    fn test_new_start_greater_than_end() {
        let range = DhcpIpRange::new("192.168.1.20", "192.168.1.10");
        assert!(range.is_err());
    }

    #[test]
    fn test_new_invalid_ip_format() {
        let range = DhcpIpRange::new("192.168.1.abc", "192.168.1.20");
        assert!(range.is_err());
    }

    #[test]
    fn test_new_invalid_end_ip_format() {
        let range = DhcpIpRange::new("192.168.1.4", "192.168.1.abc");
        assert!(range.is_err());
    }

    #[test]
    fn test_get_interface_ip() {
        let range = DhcpIpRange::new("192.168.1.10", "192.168.1.20").unwrap();
        assert_eq!(range.get_router_ip(), "192.168.1.1");
    }

    #[test]
    fn test_get_start_ip() {
        let range = DhcpIpRange::new("192.168.1.10", "192.168.1.20").unwrap();
        assert_eq!(range.get_start_ip(), "192.168.1.10");
    }

    #[test]
    fn test_get_end_ip() {
        let range = DhcpIpRange::new("192.168.1.10", "192.168.1.20").unwrap();
        assert_eq!(range.get_end_ip(), "192.168.1.20");
    }
}
