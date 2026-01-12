#![feature(ip)]

use std::collections::HashMap;
use std::fs::File;
use std::net::{IpAddr, SocketAddr};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use argh::FromArgs;
use dashmap::{DashMap, DashSet};
use etherparse::{NetSlice, SlicedPacket, TransportSlice};
use fastrace::collector::{Config, ConsoleReporter};
use futures::StreamExt;
use log::{debug, error, info, trace, warn};
use logforth::append;
use logforth::filter::env_filter::EnvFilterBuilder;
use pcap_file::pcap::{PcapHeader, PcapPacket, PcapWriter};
use pcap_file::{DataLink, Endianness, TsResolution};
use serde::Deserialize;
use windivert::prelude::*;

#[derive(FromArgs)]
/// Block the GTA connections you don't want.
struct Lobbyguard {
	/// optional path to output captured traffic
	#[argh(option, short = 'f')]
	file: Option<PathBuf>,

	/// whether to capture TCP traffic (ports 80 and 443)
	#[argh(switch)]
	capture_tcp: bool,
}

#[derive(Deserialize, Debug)]
#[serde(rename = "__InstanceCreationEvent")]
struct ProcessOpenEvent {
	#[serde(rename = "TargetInstance")]
	target_instance: Process,
}

#[derive(Deserialize, Debug)]
#[serde(rename = "__InstanceDeletionEvent")]
struct ProcessCloseEvent {
	#[serde(rename = "TargetInstance")]
	target_instance: Process,
}

#[derive(Deserialize, Debug)]
#[serde(rename = "Win32_Process")]
struct Process {
	#[serde(rename = "Name")]
	name: String,
	#[serde(rename = "ProcessID")]
	process_id: u32,
}

#[derive(Deserialize, Debug)]
#[serde(rename = "MSFT_NetTCPConnection")]
struct NetTCPConnection {
	// #[serde(rename = "LocalAddress")]
	// local_address: String,
	#[serde(rename = "LocalPort")]
	local_port: u16,
	#[serde(rename = "RemoteAddress")]
	remote_address: String,
	#[serde(rename = "RemotePort")]
	remote_port: u16,
	#[serde(rename = "OwningProcess")]
	owning_process: u32,
}

#[derive(Deserialize, Debug)]
#[serde(rename = "MSFT_NetUDPEndpoint")]
struct NetUDPEndpoint {
	// #[serde(rename = "LocalAddress")]
	// local_address: String,
	#[serde(rename = "LocalPort")]
	local_port: u16,
	#[serde(rename = "OwningProcess")]
	owning_process: u32,
}

#[derive(Deserialize, Debug)]
#[serde(rename = "__InstanceCreationEvent")]
struct UDPConnectionOpenEvent {
	#[serde(rename = "TargetInstance")]
	target_instance: NetUDPEndpoint,
}

#[derive(Deserialize, Debug)]
#[serde(rename = "__InstanceDeletionEvent")]
struct UDPConnectionCloseEvent {
	#[serde(rename = "TargetInstance")]
	target_instance: NetUDPEndpoint,
}

#[derive(Deserialize, Debug)]
#[serde(rename = "__InstanceCreationEvent")]
struct TCPConnectionOpenEvent {
	#[serde(rename = "TargetInstance")]
	target_instance: NetTCPConnection,
}

#[derive(Deserialize, Debug)]
#[serde(rename = "__InstanceDeletionEvent")]
struct TCPConnectionCloseEvent {
	#[serde(rename = "TargetInstance")]
	target_instance: NetTCPConnection,
}

#[tokio::main]
async fn main() {
	fastrace::set_reporter(ConsoleReporter, Config::default());
	logforth::starter_log::builder()
		.dispatch(|d| {
			d.filter(EnvFilterBuilder::from_default_env_or("info").build())
				.append(append::Stdout::default())
		})
		.dispatch(|d| d.append(append::FastraceEvent::default()))
		.apply();

	let args: Lobbyguard = argh::from_env();

	let main_pid_set: Arc<DashSet<u32>> = Arc::new(DashSet::new());
	// Remote addresses and local ports used by TCP connections
	let main_tcp_map: Arc<DashMap<u32, Vec<(SocketAddr, u16)>>> = Arc::new(DashMap::new());
	// Local ports used by UDP connections
	let main_udp_map: Arc<DashMap<u32, Vec<u16>>> = Arc::new(DashMap::new());

	let Ok(default_con) = wmi::WMIConnection::new() else {
		panic!("Failed to create WMI connection to ROOT\\CIMV2.");
	};

	let Ok(standard_con) = wmi::WMIConnection::with_namespace_path("ROOT\\StandardCIMV2") else {
		panic!("Failed to create WMI connection to ROOT\\StandardCIMV2.");
	};

	let mut filters = HashMap::new();
	filters.insert(
		"Name".to_owned(),
		wmi::FilterValue::Str("GTA5_Enhanced.exe"),
	);
	if let Ok(processes) = default_con.filtered_query::<Process>(&filters) {
		for process in processes {
			info!("Found process: {} ({})", process.name, process.process_id);
			main_pid_set.insert(process.process_id);
		}
	} else {
		panic!("Failed to query existing GTA processes from WMI");
	};

	if let Ok(tcps) = standard_con.query::<NetTCPConnection>() {
		let count = tcps.len();
		debug!("Queried {} TCP connections from WMI", count);
		for tcp in tcps {
			if main_pid_set.contains(&tcp.owning_process) {
				let Ok(remote_addr) = tcp.remote_address.parse() else {
					continue;
				};
				let mut entry = main_tcp_map.entry(tcp.owning_process).or_default();
				entry.push((
					SocketAddr::new(remote_addr, tcp.remote_port),
					tcp.local_port,
				));
			}
		}
	} else {
		panic!("Failed to query initial TCP connection info from WMI");
	};

	if let Ok(udps) = standard_con.query::<NetUDPEndpoint>() {
		let count = udps.len();
		debug!("Queried {} UDP endpoints from WMI", count);
		for udp in udps {
			if main_pid_set.contains(&udp.owning_process) {
				let mut entry = main_udp_map.entry(udp.owning_process).or_default();
				entry.push(udp.local_port);
			}
		}
	} else {
		panic!("Failed to query initial UDP endpoint info from WMI");
	};

	let mut filters = HashMap::new();
	filters.insert(
		"TargetInstance".to_owned(),
		wmi::FilterValue::is_a::<Process>().expect("Process should be a valid WMI class"),
	);
	let Ok(mut process_create_events) = default_con
		.async_filtered_notification::<ProcessOpenEvent>(&filters, Some(Duration::from_secs(1)))
	else {
		panic!("Failed to create WMI process creation notification stream");
	};
	let Ok(mut process_delete_events) = default_con
		.async_filtered_notification::<ProcessCloseEvent>(&filters, Some(Duration::from_secs(1)))
	else {
		panic!("Failed to create WMI process deletion notification stream");
	};

	let mut filters = HashMap::new();
	filters.insert(
		"TargetInstance".to_owned(),
		wmi::FilterValue::is_a::<NetUDPEndpoint>().expect("NetUDPEndpoint should be a valid WMI class"),
	);
	let Ok(mut udp_create_events) = standard_con
		.async_filtered_notification::<UDPConnectionOpenEvent>(&filters, Some(Duration::from_secs(1)))
	else {
		panic!("Failed to create WMI UDP connection creation notification stream");
	};
	let Ok(mut udp_delete_events) = standard_con
		.async_filtered_notification::<UDPConnectionCloseEvent>(&filters, Some(Duration::from_secs(1)))
	else {
		panic!("Failed to create WMI UDP connection deletion notification stream");
	};

	let mut filters = HashMap::new();
	filters.insert(
		"TargetInstance".to_owned(),
		wmi::FilterValue::is_a::<NetTCPConnection>()
			.expect("NetTCPConnection should be a valid WMI class"),
	);
	let Ok(mut tcp_create_events) = standard_con
		.async_filtered_notification::<TCPConnectionOpenEvent>(&filters, Some(Duration::from_secs(1)))
	else {
		panic!("Failed to create WMI TCP connection creation notification stream");
	};
	let Ok(mut tcp_delete_events) = standard_con
		.async_filtered_notification::<TCPConnectionCloseEvent>(&filters, Some(Duration::from_secs(1)))
	else {
		panic!("Failed to create WMI TCP connection deletion notification stream");
	};

	const HEARTBEAT_SIZES: [usize; 3] = [12, 18, 63];
	const MATCHMAKING_SIZES: [usize; 4] = [191, 207, 223, 239];

	// All possible ports specified from Rockstar support page
	let tcp_filter = if args.capture_tcp {
		"or (tcp ? ((tcp.DstPort == 80 or tcp.DstPort == 443 or tcp.SrcPort == 80 or tcp.SrcPort == 443) and tcp.PayloadLength > 0) : false)"
	} else {
		""
	};
	let net_filter = format!(
		"(udp ? ((udp.SrcPort == 6672 or udp.DstPort == 6672 or \
		(udp.SrcPort >= 61455 and udp.SrcPort <= 61458) or \
		(udp.DstPort >= 61455 and udp.DstPort <= 61458)) and udp.PayloadLength > 0) : false) {} \
		and (ip or ipv6)",
		tcp_filter
	);
	debug!("Creating network divert with filter: {}", net_filter);
	let Ok(network_divert) = WinDivert::<NetworkLayer>::network(net_filter, 0, Default::default())
	else {
		panic!("Failed to create network layer WinDivert handle.");
	};

	let net_shutdown_handle = network_divert.shutdown_handle();

	let pid_set = Arc::clone(&main_pid_set);
	let tcp_map = Arc::clone(&main_tcp_map);
	let udp_map = Arc::clone(&main_udp_map);
	let net_handle = tokio::spawn(async move {
		let mut pcap_writer = None;
		if let Some(file) = args.file {
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

		info!("Start receiving network packet");
		loop {
			let result = network_divert.recv_wait(&mut buffer, 0);
			let packet = match result {
				Ok(Some(packet)) => packet,
				Ok(None) => {
					continue;
				}
				Err(WinDivertError::Recv(WinDivertRecvError::NoData)) => {
					warn!("Network packet handle shutdown");
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
					// TODO: handle if user using global address without NAT
					let local_port = if !src_addr.is_global() {
						udp.source_port()
					} else if !dst_addr.is_global() {
						udp.destination_port()
					} else {
						0
					};
					let is_process = pid_set.iter().any(|pid| {
						udp_map
							.view(pid.key(), |_, ports| ports.contains(&local_port))
							.is_some()
					});
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
					let is_process = pid_set.iter().any(|pid| {
						tcp_map
							.view(pid.key(), |_, addrs| {
								addrs.iter().any(|(addr, port)| {
									let src = SocketAddr::new(src_addr, tcp.source_port());
									let dst = SocketAddr::new(dst_addr, tcp.destination_port());
									(*addr == src && *port == tcp.destination_port())
										|| (*addr == dst && *port == tcp.source_port())
								})
							})
							.is_some()
					});
					if is_process {
						let src = SocketAddr::new(src_addr, tcp.source_port());
						let dst = SocketAddr::new(dst_addr, tcp.destination_port());
						let payload = tcp.payload();
						let size = payload.len();
						trace!("PROCESS TCP PACKET PASSED {} -> {} [L{}]", src, dst, size);
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

			if capture && let Some(pcap_writer) = pcap_writer.as_mut() {
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

			if pass && let Err(e) = network_divert.send(&packet) {
				error!("Failed to send packet back to network layer: {}", e);
			}
		}
	});

	info!("Press Ctrl-C to exit.");

	loop {
		tokio::select! {
			Some(Ok(event)) = process_create_events.next() => {
				let process = event.target_instance;
				let process_id = process.process_id;
				let process_name = process.name;
				if process_name == "GTA5_Enhanced.exe" {
					info!("Process {} ({}) created", process_name, process_id);
					main_pid_set.insert(process_id);
				}

			}
			Some(Ok(event)) = process_delete_events.next() => {
				let process = event.target_instance;
				let process_id = process.process_id;
				let process_name = process.name;
				main_pid_set.remove(&process_id);
				if process_name == "GTA5_Enhanced.exe" {
					info!("Process {} ({}) deleted", process_name, process_id);
				}
				main_udp_map.remove(&process_id);
				main_tcp_map.remove(&process_id);
			}
			Some(Ok(event)) = udp_create_events.next() => {
				let udp = event.target_instance;
				if main_pid_set.contains(&udp.owning_process) {
					trace!("UDP connection created for PID {:#?}", udp);
					let mut entry = main_udp_map.entry(udp.owning_process).or_default();
					entry.push(udp.local_port);
				}
			}
			Some(Ok(event)) = udp_delete_events.next() => {
				let udp = event.target_instance;
				if let Some(mut entry) = main_udp_map.get_mut(&udp.owning_process) {
					trace!("UDP connection deleted for PID {:#?}", udp);
					entry.retain(|port| port != &udp.local_port);
				}
			}
			Some(Ok(event)) = tcp_create_events.next() => {
				let tcp = event.target_instance;
				if main_pid_set.contains(&tcp.owning_process) {
					trace!("TCP connection created for PID {:#?}", tcp);
					let Ok(remote_addr) =
						tcp.remote_address.parse()
					else {
						continue;
					};
					let mut entry = main_tcp_map.entry(tcp.owning_process).or_default();
					entry.push((SocketAddr::new(remote_addr, tcp.remote_port), tcp.local_port));
				}
			}
			Some(Ok(event)) = tcp_delete_events.next() => {
				let tcp = event.target_instance;
				if let Some(mut entry) = main_tcp_map.get_mut(&tcp.owning_process) {
					trace!("TCP connection deleted for PID {:#?}", tcp);
					let Ok(remote_addr) =
						tcp.remote_address.parse()
					else {
						continue;
					};
					entry.retain(|(addr, port)| {
						*addr != SocketAddr::new(remote_addr, tcp.remote_port) || *port != tcp.local_port
					});
				}
			}
			_ = tokio::signal::ctrl_c() => {
				info!("Ctrl-C received! Exiting gracefully.");
				break;
			}
		}
	}
	if let Err(e) = net_shutdown_handle.shutdown() {
		error!("Failed to shutdown network WinDivert: {}", e);
	}
	net_handle.abort();
	fastrace::flush();
}
