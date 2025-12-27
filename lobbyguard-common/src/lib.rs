use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

pub use windivert::prelude::*;

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

pub struct GuardEngine {
	divert: WinDivert<NetworkLayer>,
	shutdown_handle: ShutdownHandle,
}

unsafe impl Send for GuardEngine {}
unsafe impl Sync for GuardEngine {}

impl GuardEngine {
	pub fn start() -> Result<Self, WinDivertError> {
		let divert = WinDivert::<NetworkLayer>::network(DIVERT_FILTER, 0, Default::default())?;
		let shutdown_handle = divert.shutdown_handle();
		Ok(Self {
			divert,
			shutdown_handle,
		})
	}

	pub fn run(&self, blocked_count: Arc<AtomicU64>) {
		let mut buffer = [0u8; 1500];
		loop {
			match self.divert.recv(&mut buffer) {
				Ok(packet) => {
					if is_heartbeat(&packet.data) {
						let _ = self.divert.send(&packet);
					} else {
						blocked_count.fetch_add(1, Ordering::Relaxed);
					}
				}
				Err(_) => break,
			}
		}
	}

	pub fn shutdown(&self) -> Result<(), WinDivertError> { self.shutdown_handle.shutdown() }
}
