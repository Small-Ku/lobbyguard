//! Packet capture lifecycle management for the LobbyGuard GUI.

use compio::runtime::Task;
use lobbyguard_core::{
	capture::PacketCapture, packet_data::PacketCapture as PacketCaptureData,
	windivert::ShutdownHandle,
};
use snafu::prelude::*;
use std::any::Any;
use std::boxed::Box;

use crate::error::Result;

/// Manages the packet capture lifecycle.
pub struct CaptureModel {
	/// Optional background task handle.
	task: Option<
		Task<std::result::Result<lobbyguard_core::Result<PacketCaptureData>, Box<dyn Any + Send>>>,
	>,
	/// Shutdown handle for graceful termination.
	shutdown_handle: Option<ShutdownHandle>,
	/// Current packet capture session.
	session: PacketCaptureData,
}

impl CaptureModel {
	/// Creates a new `CaptureModel`.
	pub fn new() -> Self {
		Self {
			task: None,
			shutdown_handle: None,
			session: PacketCaptureData::new(),
		}
	}

	/// Starts the packet capture.
	pub async fn start(&mut self) -> Result<()> {
		let mut capture = PacketCapture::new().context(crate::error::CoreSnafu)?;
		self.shutdown_handle = Some(capture.shutdown_handle());

		let capture_task = compio::runtime::spawn_blocking(move || {
			let run_result = capture.run();
			run_result.map(|_| capture.into_session())
		});

		self.task = Some(capture_task);
		Ok(())
	}

	/// Stops the packet capture.
	pub async fn stop(&mut self) -> Result<()> {
		if let Some(handle) = self.shutdown_handle.take() {
			if let Err(e) = handle.shutdown() {
				eprintln!("Failed to send shutdown signal: {:?}", e);
			}
		}

		if let Some(task) = self.task.take() {
			match task.await {
				Ok(Ok(session)) => {
					self.session = session;
					println!(
						"Capture stopped. Session contains {} packets.",
						self.session.total_count()
					);
				}
				Ok(Err(e)) => {
					eprintln!("Capture task completed with an error: {:?}", e);
				}
				Err(e) => {
					eprintln!("Capture task failed (panic?): {:?}", e);
				}
			}
		}

		Ok(())
	}

	/// Returns a reference to the current capture session.
	pub fn session(&self) -> &PacketCaptureData {
		&self.session
	}
}
