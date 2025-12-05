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

use crate::error::{ClientSizeSnafu, Error, Result, SetButtonTextSnafu, ShowWindowSnafu};
use compio::runtime::Task;
use lobbyguard_core::capture::PacketCapture;
use snafu::ResultExt;
use winio::prelude::*;

/// Main application component
///
/// Manages the primary window and button for controlling packet capture
pub struct MainModel {
	window: Child<Window>,
	button: Child<Button>,
	/// Current capture state
	is_running: bool,
	/// Optional background task handle
	capture_task: Option<Task<std::result::Result<(), Box<dyn std::any::Any + Send>>>>,
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
	/// Window needs redrawing
	Redraw,
	/// Window close requested
	Close,
}

impl Component for MainModel {
	type Error = Error;
	type Event = ();
	type Init<'a> = ();
	type Message = MainMessage;

	/// Initialize the component with window and button
	async fn init(_init: Self::Init<'_>, _sender: &ComponentSender<Self>) -> Result<Self> {
		init! {
				window: Window = (()) => {
						text: "LobbyGuard",
						size: Size::new(300.0, 200.0),
				},
				button: Button = (&window) => {
						text: "Start",
				}
		}

		_sender.post(MainMessage::Redraw);
		window.show().context(ShowWindowSnafu)?;

		Ok(Self {
			window,
			button,
			is_running: false,
			capture_task: None,
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
				self.button => {
						ButtonEvent::Click => MainMessage::ToggleButton,
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
				Ok(false)
			}
			MainMessage::Close => {
				// Cleanup before closing
				self.stop_capture().await?;
				sender.output(());
				Ok(false)
			}
		}
	}

	/// Update child widgets
	async fn update_children(&mut self) -> Result<bool> {
		update_children!(self.window, self.button)
	}

	/// Layout widgets in the window
	fn render(&mut self, _sender: &ComponentSender<Self>) -> Result<()> {
		let csize = self.window.client_size().context(ClientSizeSnafu)?;

		let mut grid = layout! {
				Grid::from_str("1*", "1*").unwrap(),
				self.button => {
						column: 0,
						row: 0,
						halign: HAlign::Center,
						valign: VAlign::Center
				},
		};

		grid.set_rect(Rect::new(Point::zero(), csize))?;

		Ok(())
	}
}

impl MainModel {
	/// Start packet capture if not already running
	async fn start_capture(&mut self) -> Result<()> {
		if self.is_running {
			return Ok(());
		}

		println!("Starting packet capture...");

		// Try to create packet capture
		let _capture = match PacketCapture::new() {
			Ok(c) => c,
			Err(e) => {
				eprintln!("Failed to create packet capture: {}", e);
				self
					.button
					.set_text("Admin required - run as administrator")
					.context(SetButtonTextSnafu)?;
				return Err(Error::Core { source: e });
			}
		};

		// Spawn capture task using compio
		let capture_task = compio::runtime::spawn_blocking(|| {
			// Note: Packet capture is spawned as a background task
			// In a real implementation, you would run the capture.run().await here
			// For now, we just keep the task alive
		});

		self.capture_task = Some(capture_task);
		self.is_running = true;
		self.button.set_text("Stop").context(SetButtonTextSnafu)?;

		Ok(())
	}

	/// Stop packet capture if running
	async fn stop_capture(&mut self) -> Result<()> {
		if !self.is_running {
			return Ok(());
		}

		println!("Stopping packet capture...");

		// Cancel the capture task if present
		if let Some(task) = self.capture_task.take() {
			task.cancel().await;
		}

		self.is_running = false;
		self.button.set_text("Start").context(SetButtonTextSnafu)?;

		Ok(())
	}

	/// Toggle between start and stop capture
	async fn toggle_capture(&mut self) -> Result<()> {
		if self.is_running {
			self.stop_capture().await
		} else {
			self.start_capture().await
		}
	}
}
