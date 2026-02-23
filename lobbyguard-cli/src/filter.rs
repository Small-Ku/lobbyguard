use std::collections::HashSet;

/// Build a dynamic WinDivert filter string based on currently tracked ports.
///
/// # Arguments
/// * `udp_ports` - Set of active UDP local ports
/// * `tcp_ports` - Set of active TCP local/remote ports
/// * `capture_tcp` - Whether to include TCP traffic on ports 80 and 443
///
/// # Returns
/// A dynamic WinDivert filter string
pub fn build_dynamic_filter(
	udp_ports: &HashSet<u16>, tcp_ports: &HashSet<u16>, capture_tcp: bool,
) -> String {
	let mut udp_conditions = vec![
		"udp.SrcPort == 6672".to_string(),
		"udp.DstPort == 6672".to_string(),
	];

	for &port in udp_ports {
		if port != 6672 {
			udp_conditions.push(format!("udp.SrcPort == {}", port));
			udp_conditions.push(format!("udp.DstPort == {}", port));
		}
	}

	let mut tcp_filter = String::new();
	if capture_tcp {
		let mut tcp_conditions = Vec::new();
		for &port in tcp_ports {
			tcp_conditions.push(format!("tcp.SrcPort == {}", port));
			tcp_conditions.push(format!("tcp.DstPort == {}", port));
		}
		if !tcp_conditions.is_empty() {
			tcp_filter = format!(
				" or (tcp ? (({}) and tcp.PayloadLength > 0) : false)",
				tcp_conditions.join(" or ")
			);
		}
	}

	format!(
		"(udp ? (({}) and udp.PayloadLength > 0) : false) {} and (ip or ipv6)",
		udp_conditions.join(" or "),
		tcp_filter
	)
}
