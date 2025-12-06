//! # LobbyGuard CLI
//!
//! Command-line interface for packet capture and filtering.
//!
//! ## Usage
//!
//! Run with administrator privileges:
//! ```bash
//! cargo run --release
//! ```
//!
//! Press Ctrl-C to exit gracefully.

use compio::signal::ctrl_c;
use lobbyguard_core::Result;
use lobbyguard_core::capture::PacketCapture;
use snafu::report;

/// Main entry point for CLI application
#[report]
#[compio::main]
async fn main() -> Result<()> {
	println!("LobbyGuard CLI - Packet Capture System");
	println!("Make sure this is run with administrator privileges!");
	println!("Press Ctrl-C to exit.\n");

	// Create packet capture instance
	let mut capture = PacketCapture::new()?;
	let shutdown_handle = capture.shutdown_handle();

	// Setup graceful shutdown on Ctrl-C
	let ctrl_c_task = compio::runtime::spawn(async {
		ctrl_c().await.ok();
		println!("\nCtrl-C received! Shutting down gracefully...");
	});

	// Run packet capture in background
	let capture_task = compio::runtime::spawn_blocking(move || {
		if let Err(e) = capture.run() {
			eprintln!("Capture error: {}", e);
		}
	});
	// Wait for Ctrl-C
	ctrl_c_task
		.await
		.unwrap_or_else(|e| std::panic::resume_unwind(e));

	// Shutdown WinDivert
	shutdown_handle.shutdown().ok();

	// Wait for capture task to finish
	capture_task
		.await
		.unwrap_or_else(|e| std::panic::resume_unwind(e));

	println!("LobbyGuard CLI exited successfully.");
	Ok(())
}
