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
use crate::packet_data::{CapturedPacket, PacketCapture as PacketCaptureSession};
use crate::storage::CaptureStorage;
use crate::{Error, error};
use etherparse::{Ipv4Slice, UdpSlice};
use snafu::ResultExt;
use windivert::prelude::*;

/// Manages packet capture and filtering using WinDivert
pub struct PacketCapture {
	divert: WinDivert<NetworkLayer>,
	/// Current capture session for storing packet data
	session: PacketCaptureSession,
	/// Optional storage for persisting captured packets
	storage: Option<CaptureStorage>,
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

		Ok(Self {
			divert,
			session: PacketCaptureSession::new(),
			storage: None,
		})
	}

	/// Creates a new PacketCapture with a specified storage file
	///
	/// # Errors
	///
	/// Returns an error if WinDivert fails to initialize or if admin rights are insufficient
	pub fn with_storage(storage_path: impl Into<String>) -> crate::Result<Self> {
		let mut capture = Self::new()?;
		capture.storage = Some(CaptureStorage::new(storage_path));
		Ok(capture)
	}

	/// Returns a shutdown handle for graceful termination
	pub fn shutdown_handle(&self) -> windivert::ShutdownHandle {
		self.divert.shutdown_handle()
	}

	/// Processes a single packet, filtering for heartbeat packets
	///
	/// Returns `(passed, src_ip, dst_ip, src_port, dst_port, protocol)`
	fn process_packet(&self, data: &[u8]) -> crate::Result<(bool, String, String, u16, u16, u8)> {
		let ip = Ipv4Slice::from_slice(data).context(error::IpParseFailedSnafu)?;
		let header = ip.header();

		let src_ip = {
			let addr = header.source();
			format!("{}.{}.{}.{}", addr[0], addr[1], addr[2], addr[3])
		};
		let dst_ip = {
			let addr = header.destination();
			format!("{}.{}.{}.{}", addr[0], addr[1], addr[2], addr[3])
		};

		let udp = UdpSlice::from_slice(&ip.payload().payload).context(error::UdpParseFailedSnafu)?;

		let src_port = udp.source_port();
		let dst_port = udp.destination_port();
		let payload = udp.payload();
		let size = payload.len();
		let protocol = header.protocol().0 as u8;

		// Check if this is a heartbeat packet
		let passed = HEARTBEAT_SIZES.iter().any(|&x| x == size);

		if passed {
			log_heartbeat_packet(size);
		}

		Ok((passed, src_ip, dst_ip, src_port, dst_port, protocol))
	}

	/// Starts the packet capture loop
	///
	/// This method runs indefinitely until interrupted or shutdown is called.
	/// It continuously receives packets, filters them, and forwards heartbeats.
	///
	/// # Errors
	///
	/// Returns an error if packet reception or processing fails
	pub fn run(&mut self) -> crate::Result<()> {
		println!("Starting packet capture...");

		let mut buffer = [0u8; MAX_PACKET_SIZE];

		loop {
			match self.divert.recv(&mut buffer) {
				Ok(packet) => {
					if let Err(e) = self.process_packet(&packet.data) {
						eprintln!("Error processing packet: {:?}", e);
						// Continue processing instead of stopping
					}

					// Process and store packet data
					if let Ok((passed, src_ip, dst_ip, src_port, dst_port, protocol)) =
						self.process_packet(&packet.data)
					{
						let captured = CapturedPacket::new(
							packet.data.to_vec(),
							passed,
							src_ip,
							dst_ip,
							src_port,
							dst_port,
							protocol,
						);

						self.session.add_packet(captured);

						// Forward heartbeat packets
						if passed {
							self.divert.send(&packet).context(error::DivertSendSnafu)?;
						}
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

	/// Gets a reference to the current capture session
	pub fn session(&self) -> &PacketCaptureSession {
		&self.session
	}

	/// Gets a mutable reference to the current capture session
	pub fn session_mut(&mut self) -> &mut PacketCaptureSession {
		&mut self.session
	}

	/// Consumes the capture and returns the session
	pub fn into_session(self) -> PacketCaptureSession {
		self.session
	}

	/// Saves the current capture session to storage
	///
	/// # Errors
	///
	/// Returns an error if storage is not configured or write fails
	pub async fn save_session(&mut self) -> crate::Result<()> {
		self.session.end_session();

		if let Some(storage) = &self.storage {
			storage.save_capture(&self.session).await?;
			println!("Capture session saved to: {}", storage.path());
		}

		Ok(())
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
