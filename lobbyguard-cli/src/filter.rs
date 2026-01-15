/// Build the WinDivert filter string for network packet capture.
///
/// # Arguments
/// * `capture_tcp` - Whether to include TCP traffic on ports 80 and 443
///
/// # Returns
/// A WinDivert filter string
pub fn build_network_filter(capture_tcp: bool) -> String {
	let tcp_filter = if capture_tcp {
		"or (tcp ? ((tcp.DstPort == 80 or tcp.DstPort == 443 or tcp.SrcPort == 80 or tcp.SrcPort == 443) and tcp.PayloadLength > 0) : false)"
	} else {
		""
	};

	format!(
		"(udp ? ((udp.SrcPort == 6672 or udp.DstPort == 6672 or \
		(udp.SrcPort >= 61455 and udp.SrcPort <= 61458) or \
		(udp.DstPort >= 61455 and udp.DstPort <= 61458)) and udp.PayloadLength > 0) : false) {} \
		and (ip or ipv6)",
		tcp_filter
	)
}
