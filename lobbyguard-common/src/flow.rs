//! Flow tracking utilities.

use std::net::IpAddr;

/// Key for identifying a network flow.
#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy)]
pub struct FlowKey {
	pub local_addr: IpAddr,
	pub local_port: u16,
	pub remote_addr: IpAddr,
	pub remote_port: u16,
}

/// Normalize an IP address (convert IPv4-mapped IPv6 to IPv4).
pub fn normalize_ip(ip: IpAddr) -> IpAddr {
	match ip {
		IpAddr::V6(v6) => {
			if let Some(v4) = v6.to_ipv4_mapped() {
				IpAddr::V4(v4)
			} else {
				ip
			}
		}
		_ => ip,
	}
}
