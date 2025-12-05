//! Packet capture and filtering module
//!
//! Provides the core packet capture functionality for LobbyGuard.
//! Handles WinDivert initialization, packet reception, parsing, and transmission.
//!
//! ## Usage
//!
//! ```ignore
//! use lobbyguard_core::capture::PacketCapture;
//!
//! let mut capture = PacketCapture::new()?;
//! capture.start().await?;
//! ```

use crate::constants::{HEARTBEAT_SIZES, MAX_PACKET_SIZE};
use crate::{Error, error};
use etherparse::{Ipv4Slice, UdpSlice};
use snafu::ResultExt;
use windivert::prelude::*;

/// Manages packet capture and filtering using WinDivert
pub struct PacketCapture {
	divert: WinDivert<NetworkLayer>,
}

impl PacketCapture {
	/// Creates a new PacketCapture instance with WinDivert filter
	///
	/// # Errors
	///
	/// Returns an error if WinDivert fails to initialize or if admin rights are insufficient
	pub fn new() -> crate::Result<Self> {
		let filter = crate::constants::divert_filter();
		let divert = WinDivert::<NetworkLayer>::network(&filter, 0, Default::default())
			.context(error::DivertCreationSnafu { filter })?;

		Ok(Self { divert })
	}

	/// Returns a shutdown handle for graceful termination
	pub fn shutdown_handle(&self) -> windivert::ShutdownHandle {
		self.divert.shutdown_handle()
	}

	/// Processes a single packet, filtering for heartbeat packets
	///
	/// Returns `true` if the packet is a valid heartbeat and was forwarded
	fn process_packet(&self, data: &[u8]) -> crate::Result<bool> {
		let ip = Ipv4Slice::from_slice(data).context(error::IpParseFailedSnafu)?;

		let udp = UdpSlice::from_slice(&ip.payload().payload).context(error::UdpParseFailedSnafu)?;

		let payload = udp.payload();
		let size = payload.len();

		// Check if this is a heartbeat packet
		if HEARTBEAT_SIZES.iter().any(|&x| x == size) {
			log_heartbeat_packet(size);
			Ok(true)
		} else {
			Ok(false)
		}
	}

	/// Starts the packet capture loop
	///
	/// This method runs indefinitely until interrupted or shutdown is called.
	/// It continuously receives packets, filters them, and forwards heartbeats.
	///
	/// # Errors
	///
	/// Returns an error if packet reception or processing fails
	pub async fn run(&mut self) -> crate::Result<()> {
		println!("Starting packet capture...");

		let mut buffer = [0u8; MAX_PACKET_SIZE];

		loop {
			match self.divert.recv(&mut buffer) {
				Ok(packet) => {
					if let Err(e) = self.process_packet(&packet.data) {
						eprintln!("Error processing packet: {:?}", e);
						// Continue processing instead of stopping
					}

					// Forward heartbeat packets
					if let Ok(true) = self.process_packet(&packet.data) {
						self.divert.send(&packet).context(error::DivertSendSnafu)?;
					}
				}
				Err(WinDivertError::Recv(WinDivertRecvError::NoData)) => {
					return Ok(());
				}
				Err(e) => {
					return Err(Error::DivertRecv {
						source: Box::new(e),
					});
				}
			}
		}
	}

	/// Shuts down the packet capture gracefully
	///
	/// # Errors
	///
	/// Returns an error if the shutdown operation fails
	pub fn shutdown(self) -> crate::Result<()> {
		self
			.divert
			.shutdown_handle()
			.shutdown()
			.context(error::DivertShutdownSnafu)
	}
}

/// Logs a heartbeat packet event
///
/// This is a helper function to centralize logging behavior
#[inline]
fn log_heartbeat_packet(size: usize) {
	println!("HEARTBEAT PACKET PASSED [{}]", size);
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_heartbeat_sizes_constant() {
		assert!(HEARTBEAT_SIZES.contains(&12));
		assert!(HEARTBEAT_SIZES.contains(&18));
		assert!(HEARTBEAT_SIZES.contains(&63));
	}

	#[test]
	fn test_divert_filter_string() {
		let filter = crate::constants::divert_filter();
		assert!(filter.contains("6672"));
		assert!(filter.contains("udp"));
	}
}
