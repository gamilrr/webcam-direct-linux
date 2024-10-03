//! This module contains the implementation of the netlink operations for the nl80211 driver.
//!
//! The nl80211 driver is responsible for managing wireless interfaces on Linux systems using
//! the nl80211 protocol. This module provides functionalities to interact with the nl80211
//! driver through netlink sockets, allowing for operations such as:
//!
//! - Retrieving the wiphy index for the access point.
//! - Creating new wireless interfaces.
//! - Deleting existing wireless interfaces.
//! - Adding IPv4 addresses to interfaces.
//!
//! The module leverages the `neli` crate to handle netlink communication and provides a
//! high-level API for managing wireless interfaces. It includes the following key components:
//!
//! - `Nl80211Driver`: A struct representing the nl80211 driver, implementing the `WirelessDriver` trait.
//! - Various helper functions and constants to facilitate netlink communication and nl80211 operations.
//!
//! This module is essential for applications that need to manage wireless interfaces programmatically
//! on Linux systems, providing a robust and flexible interface for interacting with the nl80211 driver.

mod nl80211_const;
mod nl80211_parser;

use std::net::Ipv4Addr;
use std::str::FromStr;

use super::InterfaceIndex;
use super::WirelessDriver;

use log::error;
use log::info;
use neli::consts::rtnl::Ifa;
use neli::consts::rtnl::IfaFFlags;
use neli::consts::rtnl::RtAddrFamily;
use neli::consts::rtnl::Rtm;
use neli::rtnl::Ifaddrmsg;
use neli::rtnl::Rtattr;
use neli::types::RtBuffer;
use neli::{
    consts::{
        nl::{GenlId, NlmF, NlmFFlags},
        socket::NlFamily,
    },
    genl::{Genlmsghdr, Nlattr},
    nl::{NlPayload, Nlmsghdr},
    socket::NlSocketHandle,
    types::GenlBuffer,
};
use nl80211_const::Nl80211Attribute;
use nl80211_const::Nl80211Command;
use nl80211_const::Nl80211Iftype;
use nl80211_const::NL80211_GENL_NAME;
use nl80211_parser::parse_nl80211_payload;
use nl80211_parser::WiPhyProps;

use crate::error::Result;

/// Struct representing the nl80211 driver.
pub struct Nl80211Driver;

impl WirelessDriver for Nl80211Driver {
    /// Retrieves the wiphy index for the access point.
    ///
    /// # Returns
    /// - `Ok(Some(InterfaceIndex))` if the wiphy index is found.
    /// - `Ok(None)` if the wiphy index is not found.
    /// - `Err` if there is an error during the operation.
    fn get_ap_wiphy_indx(&self) -> Result<Option<InterfaceIndex>> {
        // Connect to the netlink socket
        let mut sock = NlSocketHandle::connect(
            NlFamily::Generic, /* family */
            Some(0),           /* pid */
            &[],               /* groups */
        )?;

        // Create the netlink request
        let nl_req = {
            let len = None;
            let nl_type = sock.resolve_genl_family(NL80211_GENL_NAME)?;
            let flags = NlmFFlags::new(&[NlmF::Request, NlmF::Dump, NlmF::Ack]);
            let seq = Some(1);
            let pid = Some(0);
            let payload = NlPayload::Payload(Genlmsghdr::<
                Nl80211Command,
                Nl80211Attribute,
            >::new(
                Nl80211Command::GetWiPhy,
                1,
                GenlBuffer::new(),
            ));

            Nlmsghdr::new(len, nl_type, flags, seq, pid, payload)
        };

        sock.send(nl_req)?;

        let mut phy_indx_opt = None;

        for msg in sock.iter(false) {
            let msg: Nlmsghdr<
                GenlId,
                Genlmsghdr<Nl80211Command, Nl80211Attribute>,
            > = msg?;
            if let NlPayload::Err(e) = msg.nl_payload {
                if e.error == -2 {
                    error!("nl80211 driver does not exist; skipping");
                } else {
                    error!("Error: {:?}", e);
                }
            } else if let Some(payload) = msg.nl_payload.get_payload() {
                let props = parse_nl80211_payload(payload)?;
                phy_indx_opt = props.ap_supported.and_then(|_| props.phy_idx);

                if phy_indx_opt.is_some() {
                    break;
                }
            }
        }

        Ok(phy_indx_opt)
    }

    /// Creates a new link with the given name and wiphy index.
    ///
    /// # Parameters
    /// - `name`: The name of the new interface.
    /// - `wiphy_idx`: The wiphy index to create the new interface from.
    ///
    /// # Returns
    /// - `Ok(Some(InterfaceIndex))` if the new interface is created successfully.
    /// - `Ok(None)` if the new interface is not created.
    /// - `Err` if there is an error during the operation.
    fn create_new_link(
        &self, name: &str, wiphy_idx: InterfaceIndex,
    ) -> Result<Option<InterfaceIndex>> {
        info!("Creating new interface from wiphy index: {}", wiphy_idx);

        let mut sock = NlSocketHandle::connect(
            NlFamily::Generic, /* family */
            Some(0),           /* pid */
            &[],               /* groups */
        )?;

        let nl_req = {
            let len = None;
            let nl_type = sock.resolve_genl_family(NL80211_GENL_NAME)?;
            let flags = NlmFFlags::new(&[NlmF::Request, NlmF::Ack]);
            let seq = Some(1);
            let pid = Some(0);

            let mut gen_buff = GenlBuffer::new();

            // Add the wiphy index to attribute buffer
            let wiphy_idx: u16 = wiphy_idx.into();
            gen_buff.push(Nlattr::new(
                false,
                false,
                Nl80211Attribute::Wiphy,
                wiphy_idx as u32,
            )?);

            gen_buff.push(Nlattr::new(
                false,
                false,
                Nl80211Attribute::Ifname,
                name,
            )?);

            let station_type: Vec<u16> =
                vec![Nl80211Iftype::IftypeStation.into(), 0];

            gen_buff.push(Nlattr::new(
                false,
                false,
                Nl80211Attribute::Iftype,
                station_type,
            )?);

            let payload =
                NlPayload::Payload(Genlmsghdr::<
                    Nl80211Command,
                    Nl80211Attribute,
                >::new(
                    Nl80211Command::NewInterface, 0, gen_buff
                ));

            Nlmsghdr::new(len, nl_type, flags, seq, pid, payload)
        };

        // Send the request
        sock.send(nl_req)?;

        let mut props = WiPhyProps::default();

        for msg in sock.iter(false) {
            let msg: Nlmsghdr<
                GenlId,
                Genlmsghdr<Nl80211Command, Nl80211Attribute>,
            > = msg?;
            if let NlPayload::Err(e) = msg.nl_payload {
                if e.error == -2 {
                    error!("nl80211 driver does not exist; skipping");
                } else {
                    error!("Error: {:?}", e);
                }
            } else if let Some(payload) = msg.nl_payload.get_payload() {
                props = parse_nl80211_payload(payload)?;
            }
        }

        info!("Props: {:?}", props);

        Ok(props.if_idx)
    }

    /// Deletes the link with the given interface index.
    ///
    /// # Parameters
    /// - `ifindex`: The interface index of the link to delete.
    ///
    /// # Returns
    /// - `Ok(())` if the link is deleted successfully.
    /// - `Err` if there is an error during the operation.
    fn delete_link(&self, ifindex: InterfaceIndex) -> Result<()> {
        info!("Deleting interface");

        let mut sock = NlSocketHandle::connect(
            NlFamily::Generic, /* family */
            Some(0),           /* pid */
            &[],               /* groups */
        )?;

        let nl_req = {
            let len = None;
            let nl_type = sock.resolve_genl_family(NL80211_GENL_NAME)?;
            let flags = NlmFFlags::new(&[NlmF::Request, NlmF::Ack]);
            let seq = Some(1);
            let pid = Some(0);

            let mut gen_buff = GenlBuffer::new();

            let ifindex: u16 = ifindex.into();
            gen_buff.push(Nlattr::new(
                false,
                false,
                Nl80211Attribute::Ifindex,
                ifindex as u32,
            )?);

            let payload =
                NlPayload::Payload(Genlmsghdr::<
                    Nl80211Command,
                    Nl80211Attribute,
                >::new(
                    Nl80211Command::DelInterface, 1, gen_buff
                ));

            Nlmsghdr::new(len, nl_type, flags, seq, pid, payload)
        };

        // Send the request
        sock.send(nl_req)?;

        for msg in sock.iter(false) {
            let msg: Nlmsghdr<
                GenlId,
                Genlmsghdr<Nl80211Command, Nl80211Attribute>,
            > = msg?;
            if let NlPayload::Err(e) = msg.nl_payload {
                if e.error == -2 {
                    error!("nl80211 driver does not exist; skipping");
                } else {
                    error!("Error: {:?}", e);
                }
            } else if let Some(payload) = msg.nl_payload.get_payload() {
                let props = parse_nl80211_payload(payload)?;
                info!("Props: {:?}", props);
            }
        }

        Ok(())
    }

    /// Adds an IPv4 address to the interface with the given index.
    ///
    /// # Parameters
    /// - `ifindex`: The interface index to add the IP address to.
    /// - `addr`: The IPv4 address to add.
    ///
    /// # Returns
    /// - `Ok(())` if the IP address is added successfully.
    /// - `Err` if there is an error during the operation.
    fn add_ipv4_addr(&self, ifindex: InterfaceIndex, addr: &str) -> Result<()> {
        info!("Adding IP to interface {}", addr);

        // Get the IPv4 address
        let ipv4_addr = Ipv4Addr::from_str(addr)?;

        let mut sock = NlSocketHandle::connect(
            NlFamily::Route, /* family */
            None,            /* pid */
            &[],             /* groups */
        )?;

        let mut rtattrs = RtBuffer::new();

        rtattrs.push(Rtattr::new(
            None,
            Ifa::Local,
            ipv4_addr.octets().to_vec(),
        )?);

        let ifindex: u16 = ifindex.into();
        let ifaddrmsg = Ifaddrmsg {
            ifa_family: RtAddrFamily::Inet,
            ifa_prefixlen: 24,
            ifa_flags: IfaFFlags::empty(),
            ifa_scope: 0,
            ifa_index: ifindex as i32,
            rtattrs,
        };

        let payload = NlPayload::Payload(ifaddrmsg);

        let nlmsg = Nlmsghdr::new(
            None,
            Rtm::Newaddr,
            NlmFFlags::new(&[
                NlmF::Excl,
                NlmF::Create,
                NlmF::Request,
                NlmF::Ack,
            ]),
            Some(1),
            Some(0),
            payload,
        );

        sock.send(nlmsg)?;

        for msg in sock.iter(false) {
            let msg: Nlmsghdr<Rtm, Ifaddrmsg> = msg?;
            info!("Received message {:#?}", msg);
        }

        Ok(())
    }
}
