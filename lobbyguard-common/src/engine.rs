//! Packet filtering engine.

use std::net::IpAddr;
use std::sync::Arc;

use dashmap::DashMap;
pub use windivert::prelude::*;

use crate::config::{FilterMode, SharedConfig, MATCHMAKING_SIZES};
use crate::events::GuardEvent;
use crate::flow::FlowKey;

/// Heartbeat payload sizes to allow through.
pub const HEARTBEAT_SIZES: [usize; 3] = [12, 18, 63];

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

/// The guard engine that filters traffic based on flows detected by the monitor.
pub struct PacketGuard {
	config: SharedConfig,
	divert: Option<WinDivert<NetworkLayer>>,
	shutdown_handle: Option<ShutdownHandle>,
	gta_flows: Arc<DashMap<FlowKey, ()>>,
	// Pids are not strictly needed for filtering if we rely on flows,
	// but might be useful for future logic or verification.
	_gta_pids: Arc<DashMap<u32, ()>>,
	event_tx: Option<kanal::Sender<GuardEvent>>,
}

unsafe impl Send for PacketGuard {}
unsafe impl Sync for PacketGuard {}

impl PacketGuard {
	/// Start the packet guard with the given configuration and monitor data.
	pub fn new(
		config: SharedConfig, gta_flows: Arc<DashMap<FlowKey, ()>>, gta_pids: Arc<DashMap<u32, ()>>,
		event_tx: Option<kanal::Sender<GuardEvent>>,
	) -> Result<Self, WinDivertError> {
		let filter = config.read().filter_mode.to_divert_filter();
		// Priority 0, flags default
		let divert = WinDivert::<NetworkLayer>::network(filter, 0, Default::default())?;
		let shutdown_handle = divert.shutdown_handle();

		Ok(Self {
			config,
			divert: Some(divert),
			shutdown_handle: Some(shutdown_handle),
			gta_flows,
			_gta_pids: gta_pids,
			event_tx,
		})
	}

	/// Run the packet guard. This blocks until shutdown.
	/// Should be run in a blocking task or separate thread.
	pub fn run(&self) {
		let Some(ref divert) = self.divert else {
			return;
		};

		let gta_flows = self.gta_flows.clone();
		let config = self.config.clone();
		let event_tx = self.event_tx.clone();

		let mut buffer = [0u8; 1500];
		while let Ok(packet) = divert.recv(&mut buffer) {
			let filter_mode = config.read().filter_mode;
			let mut payload_len = 0;
			if let Ok(ip) = etherparse::Ipv4Slice::from_slice(&packet.data) {
				// Extract ports for both UDP and TCP to identify target flows
				let (src_port, dst_port, is_udp) =
					if let Ok(udp) = etherparse::UdpSlice::from_slice(ip.payload().payload) {
						(udp.source_port(), udp.destination_port(), true)
					} else if let Ok(tcp) = etherparse::TcpSlice::from_slice(ip.payload().payload) {
						(tcp.source_port(), tcp.destination_port(), false)
					} else {
						(0, 0, false)
					};

				if src_port != 0 {
					let key = FlowKey {
						local_addr: IpAddr::V4(ip.header().source_addr()),
						local_port: src_port,
						remote_addr: IpAddr::V4(ip.header().destination_addr()),
						remote_port: dst_port,
					};
					let rev_key = FlowKey {
						local_addr: IpAddr::V4(ip.header().destination_addr()),
						local_port: dst_port,
						remote_addr: IpAddr::V4(ip.header().source_addr()),
						remote_port: src_port,
					};

					// If NOT in our GTA flow map, and NOT in reverse map, it's irrelevant traffic (usually).
					// HOWEVER, WinDivert filter should have already narrowed it down to port 6672 or similar if in Solo/Locked.
					// But in Disconnect mode (filter="ip"), we catch everything.
					// We only want to block GTA traffic.

					// Wait, if filter is "ip", we get everything. We only want to block if it matches a GTA flow.
					// The original logic was:
					// if !gta_flows.contains_key(&key) && !gta_flows.contains_key(&rev_key) {
					//     let _ = divert.send(&packet);
					//     continue;
					// }
					// This logic assumes we ONLY care about flows we tracked.
					// If we haven't tracked it (e.g. system traffic), we pass it.
					if !gta_flows.is_empty()
						&& !gta_flows.contains_key(&key)
						&& !gta_flows.contains_key(&rev_key)
					{
						let _ = divert.send(&packet);
						continue;
					} else if filter_mode == FilterMode::Disconnect {
						// It IS a GTA flow, and we are in Disconnect mode. Block it.
						PacketGuard::notify_blocked(&event_tx);
						continue;
					}

					if is_udp {
						// Heartbeat and Matchmaking checks are only for UDP
						if let Ok(udp) = etherparse::UdpSlice::from_slice(ip.payload().payload) {
							payload_len = udp.payload().len();
						}
					}
				}
			}

			// Decision logic based on filter mode
			let should_pass = match filter_mode {
				FilterMode::Solo => {
					// Solo: Only allow heartbeat packets (sizes 12, 18, 63)
					HEARTBEAT_SIZES.contains(&payload_len)
				}
				FilterMode::Locked => {
					// Locked: Block matchmaking packets, allow everything else for GTA
					!MATCHMAKING_SIZES.contains(&payload_len)
				}
				FilterMode::Disconnect => {
					// Disconnect: Block all traffic associated with GTA processes
					false
				}
			};

			if should_pass {
				let _ = divert.send(&packet);
			} else {
				PacketGuard::notify_blocked(&event_tx);
			}
		}
	}

	fn notify_blocked(tx: &Option<kanal::Sender<GuardEvent>>) {
		if let Some(tx) = tx {
			// Non-blocking send, if channel full drop it
			let _ = tx.try_send(GuardEvent::PacketBlocked);
		}
	}

	/// Shutdown the guard engine.
	pub fn shutdown(&self) -> Result<(), WinDivertError> {
		if let Some(ref handle) = self.shutdown_handle {
			handle.shutdown()
		} else {
			Ok(())
		}
	}
}
