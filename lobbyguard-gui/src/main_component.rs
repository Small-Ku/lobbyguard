//! Main window component for LobbyGuard GUI
//!
//! Implements the primary UI component that manages:
//! - Window creation and lifecycle
//! - Button state management
//! - Packet capture lifecycle
//! - Layout and rendering
//!
//! ## Architecture
//!
//! Following winio's ELM-style component pattern:
//! 1. Component struct holds child widgets
//! 2. Message enum handles state changes
//! 3. `Component` trait implements lifecycle methods
//! 4. Layout managed in `render()`

use crate::capture::CaptureModel;
use crate::error::{ClientSizeSnafu, Error, Result, SetButtonTextSnafu, ShowWindowSnafu};
use crate::viewer::{PacketViewerModel, ViewerEvent, ViewerMessage};
use snafu::ResultExt;
use winio::prelude::*;

/// Main application component
///
/// Manages the primary window and buttons for controlling packet capture
pub struct MainModel {
	window: Child<Window>,
	toggle_button: Child<Button>,
	viewer_button: Child<Button>,
	status_label: Child<Label>,
	viewer: Child<PacketViewerModel>,
	/// Current capture state
	is_running: bool,
	/// Is the viewer window currently open?
	is_viewer_open: bool,
	/// Packet capture model
	capture: CaptureModel,
}

/// Messages for updating component state
///
/// These messages are produced by event handlers and processed in `update()`
#[derive(Debug)]
pub enum MainMessage {
	/// No operation (used as default)
	Noop,
	/// User clicked the toggle button
	ToggleButton,
	/// User clicked the viewer button
	ToggleViewer,
	/// Window needs redrawing
	Redraw,
	/// Window close requested
	Close,
	/// Viewer window closed
	ViewerClosed,
}

impl Component for MainModel {
	type Error = Error;
	type Event = ();
	type Init<'a> = ();
	type Message = MainMessage;

	/// Initialize the component with window and buttons
	async fn init(_init: Self::Init<'_>, sender: &ComponentSender<Self>) -> Result<Self> {
		let capture = CaptureModel::new();
		init! {
			window: Window = (()) => {
				text: "LobbyGuard",
				size: Size::new(400.0, 250.0),
			},
			toggle_button: Button = (&window) => {
				text: "Start Capture",
			},
			viewer_button: Button = (&window) => {
				text: "View Packets",
			},
			viewer: PacketViewerModel = (capture.session().clone()) => {},
			status_label: Label = (&window) => {
				text: "Ready",
			}
		}

		// Check admin status and update initial UI
		let is_admin = crate::utils::is_running_as_admin();
		let admin_status = if is_admin {
			"Ready - Running as Administrator ✓"
		} else {
			"Warning: Not running as Administrator ⚠️"
		};

		status_label
			.set_text(admin_status)
			.context(SetButtonTextSnafu)?;

		if !is_admin {
			toggle_button
				.set_enabled(false)
				.context(crate::error::SetButtonEnabledSnafu)?;
		}

		sender.post(MainMessage::Redraw);
		window.show().context(ShowWindowSnafu)?;

		Ok(Self {
			window,
			toggle_button,
			viewer_button,
			status_label,
			viewer,
			is_running: false,
			is_viewer_open: false,
			capture,
		})
	}

	/// Start listening for events
	///
	/// This method runs the event loop, mapping widget events to messages
	async fn start(&mut self, sender: &ComponentSender<Self>) -> ! {
		start! {
			sender, default: MainMessage::Noop,
			self.window => {
				WindowEvent::Close => MainMessage::Close,
				WindowEvent::Resize | WindowEvent::ThemeChanged => MainMessage::Redraw,
			},
			self.toggle_button => {
				ButtonEvent::Click => MainMessage::ToggleButton,
			},
			self.viewer_button => {
				ButtonEvent::Click => MainMessage::ToggleViewer,
			},
			self.viewer => {
				ViewerEvent::Hide => MainMessage::ViewerClosed,
			}
		}
	}

	/// Process messages and update component state
	async fn update(
		&mut self, message: Self::Message, sender: &ComponentSender<Self>,
	) -> Result<bool> {
		match message {
			MainMessage::Noop => Ok(false),
			MainMessage::Redraw => Ok(true),
			MainMessage::ToggleButton => {
				self.toggle_capture().await?;
				Ok(true)
			}
			MainMessage::ToggleViewer => {
				self.toggle_viewer().await?;
				Ok(false)
			}
			MainMessage::ViewerClosed => {
				self.is_viewer_open = false;
				self
					.viewer_button
					.set_text("View Packets")
					.context(SetButtonTextSnafu)?;
				Ok(true)
			}
			MainMessage::Close => {
				// Cleanup before closing
				self.viewer.emit(ViewerMessage::Close).await?;
				if self.is_running {
					self.toggle_capture().await?;
				}
				sender.output(());
				Ok(false)
			}
		}
	}

	/// Update child widgets
	async fn update_children(&mut self) -> Result<bool> {
		update_children!(
			self.window,
			self.toggle_button,
			self.viewer_button,
			self.status_label,
			self.viewer
		)
	}

	/// Layout widgets in the window
	fn render(&mut self, _sender: &ComponentSender<Self>) -> Result<()> {
		// Use a simple fixed layout instead of complex grid parsing
		let grid = match Grid::from_str("1*, 1*", "1*, 1*, 1*") {
			Ok(g) => g,
			Err(_) => {
				// Fallback: create a basic 2x3 grid manually
				println!("Warning: Failed to parse grid, using basic layout");
				return Ok(()); // Skip layout for now
			}
		};

		let mut layout = layout! {
				grid,
				self.toggle_button => {
						column: 0,
						row: 0,
						halign: HAlign::Center,
						valign: VAlign::Center
				},
				self.viewer_button => {
						column: 1,
						row: 0,
						halign: HAlign::Center,
						valign: VAlign::Center
				},
				self.status_label => {
						column: 0,
						row: 1,
						column_span: 2,
						halign: HAlign::Center,
						valign: VAlign::Center
				}
		};

		let client_size = self.window.client_size().context(ClientSizeSnafu)?;
		layout.set_rect(Rect::new(Point::zero(), client_size))?;

		Ok(())
	}
}

impl MainModel {
	/// Toggle between start and stop capture
	async fn toggle_capture(&mut self) -> Result<()> {
		if self.is_running {
			self.capture.stop().await?;
			self.is_running = false;
			self
				.toggle_button
				.set_text("Start Capture")
				.context(SetButtonTextSnafu)?;
			let session = self.capture.session();
			self
				.status_label
				.set_text(&format!(
					"Stopped - Captured: {} (Passed: {}, Rejected: {})",
					session.total_count(),
					session.passed_count(),
					session.rejected_count()
				))
				.context(SetButtonTextSnafu)?;
		} else {
			if !crate::utils::is_running_as_admin() {
				self
					.status_label
					.set_text("Administrator privileges required for packet capture")
					.context(SetButtonTextSnafu)?;
				return Ok(());
			}
			self.capture.start().await?;
			self.is_running = true;
			self
				.toggle_button
				.set_text("Stop Capture")
				.context(SetButtonTextSnafu)?;
			self
				.status_label
				.set_text("Capturing...")
				.context(SetButtonTextSnafu)?;
		}
		Ok(())
	}

	/// Open the packet viewer window
	async fn toggle_viewer(&mut self) -> Result<()> {
		if self.is_viewer_open {
			self.viewer.emit(ViewerMessage::Hide).await?;
			self.is_viewer_open = false;
			self
				.viewer_button
				.set_text("View Packets")
				.context(SetButtonTextSnafu)?;
		} else {
			println!(
				"Opening viewer with {} packets",
				self.capture.session().total_count()
			);
			self
				.status_label
				.set_text("Packet viewer opened")
				.context(SetButtonTextSnafu)?;
			self
				.viewer
				.emit(ViewerMessage::UpdateSession(self.capture.session().clone()))
				.await?;
			self.viewer.show().await?;
			self.is_viewer_open = true;
			self
				.viewer_button
				.set_text("Close Viewer")
				.context(SetButtonTextSnafu)?;
		}

		Ok(())
	}
}
