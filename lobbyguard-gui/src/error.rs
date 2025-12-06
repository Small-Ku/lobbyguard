//! GUI-specific error handling
//!
//! Extends core error types with UI-specific error variants using snafu.

use snafu::prelude::*;

/// Result type alias for GUI operations
pub type Result<T, E = Error> = std::result::Result<T, E>;

/// GUI error type combining winio UI errors with core logic errors
#[derive(Debug, Snafu)]
#[snafu(visibility(pub))]
pub enum Error {
	/// Failed to create the application window
	#[snafu(display("Failed to create application: {source}"))]
	NewApp { source: winio::Error },

	/// Failed to show the window
	#[snafu(display("Failed to show window: {source}"))]
	ShowWindow { source: winio::Error },

	/// Generic winio UI error
	#[snafu(context(false), display("UI error: {source}"))]
	Ui { source: winio::Error },

	/// Generic taffy layout error
	#[snafu(context(false), display("UI error: {source}"))]
	Layout {
		source: winio::prelude::LayoutError<winio::Error>,
	},

	/// Failed to set button text
	#[snafu(display("Failed to set button text: {source}"))]
	SetButtonText { source: winio::Error },

	/// Failed to set button enabled state
	#[snafu(display("Failed to set button enabled state: {source}"))]
	SetButtonEnabled { source: winio::Error },

	/// Failed to get window client size
	#[snafu(display("Failed to get window client size: {source}"))]
	ClientSize { source: winio::Error },

	/// Core lobbyguard error
	#[snafu(display("Core error: {source}"))]
	Core { source: lobbyguard_core::Error },
}
