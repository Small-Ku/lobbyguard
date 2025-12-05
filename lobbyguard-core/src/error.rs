//! Error types for LobbyGuard Core
//!
//! All fallible operations use `snafu` for structured error handling.
//! This ensures errors include context without using unwrap/panic.

use snafu::prelude::*;

/// Result type alias for operations using snafu error handling
pub type Result<T, E = Error> = std::result::Result<T, E>;

/// Core error type for LobbyGuard operations
#[derive(Debug, Snafu)]
#[snafu(visibility(pub))]
pub enum Error {
	/// Failed to create WinDivert filter
	#[snafu(display("Failed to create WinDivert filter: {filter}"))]
	DivertCreation {
		source: windivert::error::WinDivertError,
		filter: String,
	},

	/// Failed to receive packet from WinDivert
	#[snafu(display("Failed to receive packet from WinDivert: {source}"))]
	DivertRecv {
		source: Box<dyn std::error::Error + Send + Sync>,
	},

	/// Failed to send packet through WinDivert
	#[snafu(display("Failed to send packet through WinDivert: {source}"))]
	DivertSend {
		source: windivert::error::WinDivertError,
	},

	/// Failed to shutdown WinDivert
	#[snafu(display("Failed to shutdown WinDivert: {source}"))]
	DivertShutdown {
		source: windivert::error::WinDivertError,
	},

	/// Failed to parse IP packet headers
	#[snafu(display("Failed to parse IP packet headers"))]
	IpParseFailed {
		source: etherparse::err::ipv4::SliceError,
	},

	/// Failed to parse UDP packet headers
	#[snafu(display("Failed to parse UDP packet headers"))]
	UdpParseFailed{
		source: etherparse::err::LenError,
	},

	/// Packet capture was interrupted
	#[snafu(display("Packet capture was interrupted: no data"))]
	CaptureInterrupted,
}
