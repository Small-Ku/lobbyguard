use std::fs::File;
use std::net::{IpAddr, SocketAddr};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use etherparse::{NetSlice, SlicedPacket, TransportSlice};
use log::{debug, error, trace};
use pcap_file::pcap::{PcapHeader, PcapPacket, PcapWriter};
use pcap_file::{DataLink, Endianness, TsResolution};
use windivert::prelude::*;

use crate::connection_tracker::ConnectionTracker;

/// Packet size constants for GTA Online traffic classification
pub const HEARTBEAT_SIZES: [usize; 3] = [12, 18, 63];
pub const MATCHMAKING_SIZES: [usize; 4] = [191, 207, 223, 239];

/// Process network packets from WinDivert
pub fn process_packets(
	network_divert: WinDivert<NetworkLayer>,
	tracker: Arc<ConnectionTracker>,
	pcap_file: Option<PathBuf>,
) {
	let mut pcap_writer = None;
	if let Some(file) = pcap_file {
		match File::create(&file) {
			Ok(file_out) => {
				match PcapWriter::with_header(
					file_out,
					PcapHeader {
						version_major: 2,
						version_minor: 4,
						ts_correction: 0,
						ts_accuracy: 0,
						snaplen: 65535,
						datalink: DataLink::RAW,
						ts_resolution: TsResolution::MicroSecond,
						endianness: Endianness::native(),
					},
				) {
					Ok(pcap) => pcap_writer = Some(pcap),
					Err(e) => error!("Error initializing PCAP writer for {:?}: {}", file, e),
				}
			}
			Err(e) => error!("Error creating PCAP file {:?}: {}", file, e),
		}
	}

	let mut buffer = [0u8; 1500];

	debug!("Start receiving network packet");
	loop {
		let result = network_divert.recv_wait(&mut buffer, 0);
		let packet = match result {
			Ok(Some(packet)) => packet,
			Ok(None) => {
				continue;
			}
			Err(WinDivertError::Recv(WinDivertRecvError::NoData)) => {
				debug!("Network packet handle shutdown");
				break;
			}
			Err(e) => {
				error!("Error receiving network packet: {}", e);
				break;
			}
		};

		let Ok(sliced_packet) = SlicedPacket::from_ip(&packet.data) else {
			error!(
				"Failed to parse packet headers despite filter match - data length: {}",
				packet.data.len()
			);
			continue;
		};

		let (src_addr, dst_addr): (IpAddr, IpAddr) = match sliced_packet.net {
			Some(NetSlice::Ipv4(ip4)) => (
				ip4.header().source_addr().into(),
				ip4.header().destination_addr().into(),
			),
			Some(NetSlice::Ipv6(ip6)) => (
				ip6.header().source_addr().into(),
				ip6.header().destination_addr().into(),
			),
			_ => {
				debug!(
					"Skipping non-IPv4/IPv6 packet from network layer: {:?}",
					sliced_packet.net
				);
				continue;
			}
		};

		let (pass, capture) = match sliced_packet.transport {
			Some(TransportSlice::Udp(udp)) => {
				let src = SocketAddr::new(src_addr, udp.source_port());
				let dst = SocketAddr::new(dst_addr, udp.destination_port());
				let local_port = if !src_addr.is_global() {
					udp.source_port()
				} else if !dst_addr.is_global() {
					udp.destination_port()
				} else {
					0
				};
				let is_process = tracker.is_tracked_udp(local_port);
				let payload = udp.payload();
				let size = payload.len();

				let matching_port = local_port == 6672;

				if is_process {
					if matching_port && HEARTBEAT_SIZES.contains(&size) {
						debug!("HEARTBEAT PACKET PASSED {} -> {} [L{}]", src, dst, size);
					} else if matching_port && MATCHMAKING_SIZES.contains(&size) {
						trace!("MATCHMAKING PACKET BLOCKED {} -> {} [L{}]", src, dst, size);
					} else {
						trace!("PROCESS UDP PACKET BLOCKED {} -> {} [L{}]", src, dst, size);
					}
				}
				(
					!is_process || matching_port && HEARTBEAT_SIZES.contains(&size),
					is_process,
				)
			}
			Some(TransportSlice::Tcp(tcp)) => {
				let src_port = tcp.source_port();
				let dst_port = tcp.destination_port();
				let is_process = tracker.is_tracked_tcp(src_port, dst_port);
				if is_process {
					let payload = tcp.payload();
					let size = payload.len();
					trace!("PROCESS TCP PACKET PASSED {} -> {} [L{}]", src_port, dst_port, size);
				}
				(true, is_process)
			}
			_ => {
				debug!(
					"Skipping non-UDP/TCP packet from network layer: {:?}",
					sliced_packet.transport
				);
				continue;
			}
		};

		if capture
			&& let Some(pcap_writer) = pcap_writer.as_mut() {
				let timestamp = std::time::SystemTime::now()
					.duration_since(std::time::SystemTime::UNIX_EPOCH)
					.unwrap_or_else(|e| {
						error!("Time went backwards: {}", e);
						Duration::ZERO
					});
				let pcap_packet = PcapPacket::new(timestamp, packet.data.len() as u32, &packet.data);
				if let Err(e) = pcap_writer.write_packet(&pcap_packet) {
					error!("Error writing packet to PCAP: {}", e);
				}
			}

		if pass
			&& let Err(e) = network_divert.send(&packet) {
				error!("Failed to send packet back to network layer: {}", e);
			}
	}
}
