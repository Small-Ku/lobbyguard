//! Packet data structures for capture and storage
//!
//! Defines serializable data structures for captured network packets
//! using epserde for efficient serialization.

use epserde::prelude::*;
use std::time::{SystemTime, UNIX_EPOCH};

/// Represents a single captured packet with metadata
#[derive(Epserde, Clone, Debug)]
pub struct CapturedPacket {
	/// Timestamp when the packet was captured (seconds since UNIX_EPOCH)
	pub timestamp: u64,
	/// Raw packet data
	pub data: Vec<u8>,
	/// Packet size in bytes
	pub size: u32,
	/// Whether the packet passed filtering (true) or was rejected (false)
	pub passed: bool,
	/// Source IP address as string for readability
	pub src_ip: String,
	/// Destination IP address as string for readability
	pub dst_ip: String,
	/// Source port
	pub src_port: u16,
	/// Destination port
	pub dst_port: u16,
	/// Protocol identifier (e.g., UDP=17, TCP=6)
	pub protocol: u8,
}

impl CapturedPacket {
	/// Creates a new captured packet with current timestamp
	pub fn new(
		data: Vec<u8>, passed: bool, src_ip: String, dst_ip: String, src_port: u16, dst_port: u16,
		protocol: u8,
	) -> Self {
		let size = data.len() as u32;
		let timestamp = SystemTime::now()
			.duration_since(UNIX_EPOCH)
			.map(|d| d.as_secs())
			.unwrap_or(0);

		Self {
			timestamp,
			data,
			size,
			passed,
			src_ip,
			dst_ip,
			src_port,
			dst_port,
			protocol,
		}
	}

	/// Returns a human-readable description of the packet
	pub fn description(&self) -> String {
		let status = if self.passed { "PASSED" } else { "REJECTED" };
		format!(
			"[{}] {}:{} -> {}:{} (size: {} bytes, proto: {})",
			status, self.src_ip, self.src_port, self.dst_ip, self.dst_port, self.size, self.protocol
		)
	}
}

/// Collection of captured packets for storage
#[derive(Epserde, Clone, Debug)]
pub struct PacketCapture {
	/// Packets captured in this session
	pub packets: Vec<CapturedPacket>,
	/// Session start timestamp
	pub start_time: u64,
	/// Session end timestamp (0 if still capturing)
	pub end_time: u64,
}

impl PacketCapture {
	/// Creates a new packet capture session
	pub fn new() -> Self {
		let start_time = SystemTime::now()
			.duration_since(UNIX_EPOCH)
			.map(|d| d.as_secs())
			.unwrap_or(0);

		Self {
			packets: Vec::new(),
			start_time,
			end_time: 0,
		}
	}

	/// Adds a captured packet to the session
	pub fn add_packet(&mut self, packet: CapturedPacket) {
		self.packets.push(packet);
	}

	/// Marks the capture session as ended
	pub fn end_session(&mut self) {
		if self.end_time == 0 {
			self.end_time = SystemTime::now()
				.duration_since(UNIX_EPOCH)
				.map(|d| d.as_secs())
				.unwrap_or(0);
		}
	}

	/// Returns count of passed packets
	pub fn passed_count(&self) -> usize {
		self.packets.iter().filter(|p| p.passed).count()
	}

	/// Returns count of rejected packets
	pub fn rejected_count(&self) -> usize {
		self.packets.iter().filter(|p| !p.passed).count()
	}

	/// Returns total packet count
	pub fn total_count(&self) -> usize {
		self.packets.len()
	}
}

impl Default for PacketCapture {
	fn default() -> Self {
		Self::new()
	}
}
