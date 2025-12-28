use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use dashmap::DashMap;
use futures::StreamExt;
use serde::Deserialize;
use tracing::info;
pub use windivert::prelude::*;
use wmi::{FilterValue, WMIConnection};

pub const HEARTBEAT_SIZES: [usize; 3] = [12, 18, 63];
pub const DIVERT_FILTER: &str = "udp.DstPort == 6672 and udp.PayloadLength > 0 and ip";

/// Helper to determine if a packet is a heartbeat.
pub fn is_heartbeat(data: &[u8]) -> bool {
	let Ok(ip) = etherparse::Ipv4Slice::from_slice(data) else {
		return false;
	};
	let Ok(udp) = etherparse::UdpSlice::from_slice(ip.payload().payload) else {
		return false;
	};
	HEARTBEAT_SIZES.contains(&udp.payload().len())
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy)]
pub struct FlowKey {
	pub local_addr: IpAddr,
	pub local_port: u16,
	pub remote_addr: IpAddr,
	pub remote_port: u16,
}

pub struct GuardEngine {
	divert: WinDivert<NetworkLayer>,
	flow_divert: Arc<WinDivert<FlowLayer>>,
	shutdown_handle: ShutdownHandle,
	flow_shutdown_handle: ShutdownHandle,
	gta_flows: Arc<DashMap<FlowKey, ()>>,
	gta_pids: Arc<DashMap<u32, ()>>,
}

unsafe impl Send for GuardEngine {}
unsafe impl Sync for GuardEngine {}

#[derive(Deserialize, Debug)]
#[allow(non_snake_case)]
#[allow(non_camel_case_types)]
struct Win32_Process {
	ProcessId: u32,
	Name: String,
}

#[derive(Deserialize, Debug)]
#[allow(non_snake_case)]
struct __InstanceCreationEvent {
	TargetInstance: Win32_Process,
}

#[derive(Deserialize, Debug)]
#[allow(non_snake_case)]
struct __InstanceDeletionEvent {
	TargetInstance: Win32_Process,
}

pub async fn get_gta_pids(con: &WMIConnection) -> wmi::WMIResult<Vec<u32>> {
	let mut filters = HashMap::new();
	filters.insert("Name".to_owned(), FilterValue::Str("GTA5.exe"));
	let results1 = con.async_filtered_query::<Win32_Process>(&filters).await?;

	filters.insert("Name".to_owned(), FilterValue::Str("GTA5_enhanced.exe"));
	let results2 = con.async_filtered_query::<Win32_Process>(&filters).await?;

	let mut pids: Vec<u32> = results1.into_iter().map(|p| p.ProcessId).collect();
	pids.extend(results2.into_iter().map(|p| p.ProcessId));
	Ok(pids)
}

pub async fn monitor_gta_processes(
	con: &WMIConnection, pids: Arc<DashMap<u32, ()>>,
) -> wmi::WMIResult<()> {
	let mut create_filters = HashMap::new();
	create_filters.insert(
		"TargetInstance".to_owned(),
		FilterValue::is_a::<Win32_Process>()?,
	);

	let mut creation_stream = con.async_filtered_notification::<__InstanceCreationEvent>(
		&create_filters,
		Some(Duration::from_secs(1)),
	)?;

	let mut deletion_stream = con.async_filtered_notification::<__InstanceDeletionEvent>(
		&create_filters, // Filter is the same: TargetInstance ISA Win32_Process
		Some(Duration::from_secs(1)),
	)?;

	loop {
		tokio::select! {
			Some(event) = creation_stream.next() => {
				if let Ok(event) = event {
					let name = event.TargetInstance.Name.to_lowercase();
					if name == "gta5.exe" || name == "gta5_enhanced.exe" {
						pids.insert(event.TargetInstance.ProcessId, ());
						info!(
							"GTA process started: PID {}",
							event.TargetInstance.ProcessId
						);
					}
				}
			}
			Some(event) = deletion_stream.next() => {
				if let Ok(event) = event {
					let name = event.TargetInstance.Name.to_lowercase();
					if name == "gta5.exe" || name == "gta5_enhanced.exe" {
						pids.remove(&event.TargetInstance.ProcessId);
						info!(
							"GTA process stopped: PID {}",
							event.TargetInstance.ProcessId
						);
					}
				}
			}
		}
	}
}

fn normalize_ip(ip: IpAddr) -> IpAddr {
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

impl GuardEngine {
	pub fn start() -> Result<Self, WinDivertError> {
		let divert = WinDivert::<NetworkLayer>::network(DIVERT_FILTER, 0, Default::default())?;
		let shutdown_handle = divert.shutdown_handle();

		// Listen for flows to identify GTA connections
		let flow_divert = WinDivert::<FlowLayer>::flow("ip", 0, Default::default())?;
		let flow_shutdown_handle = flow_divert.shutdown_handle();

		Ok(Self {
			divert,
			flow_divert: Arc::new(flow_divert),
			shutdown_handle,
			flow_shutdown_handle,
			gta_flows: Arc::new(DashMap::new()),
			gta_pids: Arc::new(DashMap::new()),
		})
	}

	pub fn run(&self, blocked_count: Arc<AtomicU64>) {
		let gta_flows = self.gta_flows.clone();
		let gta_pids = self.gta_pids.clone();
		let flow_divert = self.flow_divert.clone();

		// Start WMI monitoring in a background thread
		let gta_pids_wmi = self.gta_pids.clone();
		std::thread::spawn(move || {
			let rt = tokio::runtime::Builder::new_current_thread()
				.enable_all()
				.build()
				.unwrap();

			rt.block_on(async {
				let con = WMIConnection::new().unwrap();

				// Initial scan
				if let Ok(pids) = get_gta_pids(&con).await {
					for pid in pids {
						gta_pids_wmi.insert(pid, ());
					}
				}

				// Continuous monitoring
				let _ = monitor_gta_processes(&con, gta_pids_wmi).await;
			});
		});

		// Background thread to monitor flows and identify GTA processes
		std::thread::spawn(move || loop {
			match flow_divert.recv() {
				Ok(packet) => {
					let flow = packet.address;

					let key = FlowKey {
						local_addr: normalize_ip(flow.local_address()),
						local_port: flow.local_port(),
						remote_addr: normalize_ip(flow.remote_address()),
						remote_port: flow.remote_port(),
					};

					match flow.event() {
						WinDivertEvent::FlowEstablished => {
							if gta_pids.contains_key(&flow.process_id()) {
								gta_flows.insert(key, ());
							}
						}
						WinDivertEvent::FlowDeleted => {
							gta_flows.remove(&key);
						}
						_ => {}
					}
				}
				Err(_) => break,
			}
		});

		let gta_flows = self.gta_flows.clone();
		let mut buffer = [0u8; 1500];
		loop {
			match self.divert.recv(&mut buffer) {
				Ok(packet) => {
					let mut is_gta = false;
					let mut is_hb = false;

					if let Ok(ip) = etherparse::Ipv4Slice::from_slice(&packet.data) {
						if let Ok(udp) = etherparse::UdpSlice::from_slice(ip.payload().payload) {
							let udp_header = udp.to_header();
							let key = FlowKey {
								local_addr: IpAddr::V4(ip.header().source_addr().into()),
								local_port: udp_header.source_port,
								remote_addr: IpAddr::V4(ip.header().destination_addr().into()),
								remote_port: udp_header.destination_port,
							};
							let rev_key = FlowKey {
								local_addr: IpAddr::V4(ip.header().destination_addr().into()),
								local_port: udp_header.destination_port,
								remote_addr: IpAddr::V4(ip.header().source_addr().into()),
								remote_port: udp_header.source_port,
							};

							if gta_flows.contains_key(&key) || gta_flows.contains_key(&rev_key) {
								is_gta = true;
							}
							is_hb = HEARTBEAT_SIZES.contains(&udp.payload().len());
						}
					}

					if is_gta || is_hb {
						let _ = self.divert.send(&packet);
					} else {
						blocked_count.fetch_add(1, Ordering::Relaxed);
					}
				}
				Err(_) => break,
			}
		}
	}

	pub fn shutdown(&self) -> Result<(), WinDivertError> {
		let _ = self.flow_shutdown_handle.shutdown();
		self.shutdown_handle.shutdown()
	}
}
