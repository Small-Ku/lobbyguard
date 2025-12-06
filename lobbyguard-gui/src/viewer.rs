//! Captured packet viewer component
//!
//! Provides a window for viewing and analyzing captured network packets.
//! Displays packet information in a scrollable list with pass/reject status.

use crate::error::{ClientSizeSnafu, Error, Result, ShowWindowSnafu};
use lobbyguard_core::packet_data::PacketCapture as PacketCaptureSession;
use snafu::ResultExt;
use winio::prelude::*;

/// Viewer window for captured packets
pub struct PacketViewerModel {
	window: Child<Window>,
	/// List of packets to display
	packet_list: Vec<String>,
	/// Index of currently selected packet
	selected_index: usize,
	/// Current packet capture session
	session: PacketCaptureSession,
}

/// Messages for the packet viewer
#[derive(Debug)]
pub enum ViewerMessage {
	/// No operation
	Noop,
	/// Window needs redrawing
	Redraw,
	/// Window close requested
	Hide,
	/// Update with new packet session
	UpdateSession(PacketCaptureSession),
	Close,
}

/// Events produced by the packet viewer
#[derive(Debug)]
pub enum ViewerEvent {
	Hide,
}

impl Component for PacketViewerModel {
	type Error = Error;
	type Event = ViewerEvent;
	type Init<'a> = PacketCaptureSession;
	type Message = ViewerMessage;

	/// Initialize the viewer component
	async fn init(init: Self::Init<'_>, sender: &ComponentSender<Self>) -> Result<Self> {
		init! {
			window: Window = (()) => {
				text: "LobbyGuard - Packet Viewer",
				size: Size::new(800.0, 600.0),
			}
		}

		let packet_list = Self::build_packet_list(&init);
		let selected_index = 0;

		sender.post(ViewerMessage::Redraw);

		Ok(Self {
			window,
			packet_list,
			selected_index,
			session: init,
		})
	}

	/// Start listening for events
	async fn start(&mut self, sender: &ComponentSender<Self>) -> ! {
		start! {
			sender, default: ViewerMessage::Noop,
			self.window => {
				WindowEvent::Close => ViewerMessage::Hide,
				WindowEvent::Resize | WindowEvent::ThemeChanged => ViewerMessage::Redraw,
			}
		}
	}

	/// Process messages and update component state
	async fn update(
		&mut self, message: Self::Message, sender: &ComponentSender<Self>,
	) -> Result<bool> {
		match message {
			ViewerMessage::Noop => Ok(false),
			ViewerMessage::Redraw => Ok(true),
			ViewerMessage::UpdateSession(session) => {
				self.session = session;
				self.packet_list = Self::build_packet_list(&self.session);
				self.selected_index = 0;
				Ok(true)
			}
			ViewerMessage::Hide => {
				self.window.hide().context(ShowWindowSnafu)?;
				sender.output(ViewerEvent::Hide);
				Ok(true)
			}
			ViewerMessage::Close => Ok(false),
		}
	}

	/// Update child widgets
	async fn update_children(&mut self) -> Result<bool> {
		update_children!(self.window)
	}

	/// Render the window and layout
	fn render(&mut self, _sender: &ComponentSender<Self>) -> Result<()> {
		let _csize = self.window.client_size().context(ClientSizeSnafu)?;

		let _grid = layout! {
			Grid::from_str("1*", "1*").unwrap(),
			self.window => {
				column: 0,
				row: 0,
			},
		};

		Ok(())
	}
}

impl PacketViewerModel {
	/// Show packet viewer window
	pub async fn show(&mut self) -> Result<()> {
		self.window.show().context(ShowWindowSnafu)?;
		self.window.set_visible(true).context(ShowWindowSnafu)
	}

	/// Builds a list of formatted packet descriptions from the session
	fn build_packet_list(session: &PacketCaptureSession) -> Vec<String> {
		session
			.packets
			.iter()
			.enumerate()
			.map(|(idx, packet)| {
				format!(
					"[{}] {} [{}B] {}",
					idx + 1,
					packet.description(),
					packet.size,
					if packet.passed { "✓" } else { "✗" }
				)
			})
			.collect()
	}

	/// Gets summary statistics for the current session
	pub fn get_summary(&self) -> String {
		format!(
			"Total: {} | Passed: {} | Rejected: {}",
			self.session.total_count(),
			self.session.passed_count(),
			self.session.rejected_count()
		)
	}

	/// Gets detailed information about a specific packet
	pub fn get_packet_details(&self, index: usize) -> Option<String> {
		self.session.packets.get(index).map(|packet| {
			format!(
				"Packet #{}\n\
Source: {}:{}\n\
Destination: {}:{}\n\
Size: {} bytes\n\
Protocol: {}\n\
Status: {}\n\
Timestamp: {}",
				index + 1,
				packet.src_ip,
				packet.src_port,
				packet.dst_ip,
				packet.dst_port,
				packet.size,
				packet.protocol,
				if packet.passed { "PASSED" } else { "REJECTED" },
				packet.timestamp
			)
		})
	}

	/// Updates the viewer with a new capture session
	pub fn set_session(&mut self, session: PacketCaptureSession) {
		self.session = session;
		self.packet_list = Self::build_packet_list(&self.session);
		self.selected_index = 0;
	}

	/// Gets the packet list for rendering
	pub fn packet_list(&self) -> &[String] {
		&self.packet_list
	}

	/// Sets the selected packet index
	pub fn set_selected(&mut self, index: usize) {
		if index < self.packet_list.len() {
			self.selected_index = index;
		}
	}

	/// Gets the currently selected index
	pub fn selected_index(&self) -> usize {
		self.selected_index
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use lobbyguard_core::packet_data::CapturedPacket;

	#[test]
	fn test_packet_viewer_creation() {
		let session = PacketCaptureSession::new();
		let viewer = PacketViewerModel {
			window: unsafe { std::mem::zeroed() }, // Mock for testing
			packet_list: Vec::new(),
			selected_index: 0,
			session,
		};
		assert_eq!(viewer.selected_index(), 0);
	}

	#[test]
	fn test_build_packet_list() {
		let mut session = PacketCaptureSession::new();
		session.add_packet(CapturedPacket::new(
			vec![1, 2, 3],
			true,
			"192.168.1.1".to_string(),
			"192.168.1.2".to_string(),
			5000,
			5001,
			17,
		));

		let list = PacketViewerModel::build_packet_list(&session);
		assert_eq!(list.len(), 1);
		assert!(list[0].contains("PASSED") || list[0].contains("✓"));
	}

	#[test]
	fn test_summary_stats() {
		let mut session = PacketCaptureSession::new();
		session.add_packet(CapturedPacket::new(
			vec![1, 2, 3],
			true,
			"192.168.1.1".to_string(),
			"192.168.1.2".to_string(),
			5000,
			5001,
			17,
		));
		session.add_packet(CapturedPacket::new(
			vec![1, 2],
			false,
			"192.168.1.3".to_string(),
			"192.168.1.4".to_string(),
			5002,
			5003,
			17,
		));

		let viewer = PacketViewerModel {
			window: unsafe { std::mem::zeroed() }, // Mock for testing
			packet_list: Vec::new(),
			selected_index: 0,
			session,
		};

		let summary = viewer.get_summary();
		assert!(summary.contains("Total: 2"));
		assert!(summary.contains("Passed: 1"));
		assert!(summary.contains("Rejected: 1"));
	}
}
