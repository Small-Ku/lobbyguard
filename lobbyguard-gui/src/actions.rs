//! Action types for navigation and UI events.

use lobbyguard_common::FilterMode;

/// Pages in the application.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Page {
	#[default]
	Dashboard,
	Connections,
	Whitelist,
	Settings,
}

/// Ways the app can navigate or respond to UI events.
#[derive(Debug, Clone)]
pub enum Action {
	/// Switch to a page.
	Navigate(Page),
	/// Set the filter mode (triggers engine start/stop).
	SetMode(SessionMode),
	/// Toggle between light and dark theme.
	ToggleTheme,
	/// Add a whitelist entry.
	AddWhitelist { name: String, ip: String },
	/// Remove a whitelist entry by ID.
	RemoveWhitelist(u64),
	/// No action (null pattern for optional actions).
	None,
}

/// UI-facing session modes (maps to FilterMode + Standard).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SessionMode {
	/// No filtering - engine stopped.
	#[default]
	Standard,
	/// Solo session - strictest firewall.
	Solo,
	/// Locked session - blocks new connections.
	Locked,
	/// Disconnect - blocks all GTA traffic.
	Disconnect,
}

impl SessionMode {
	/// All modes for iteration.
	pub const ALL: [SessionMode; 4] = [
		SessionMode::Standard,
		SessionMode::Solo,
		SessionMode::Locked,
		SessionMode::Disconnect,
	];

	/// Convert to engine FilterMode. Returns None for Standard (engine off).
	#[must_use]
	pub fn to_filter_mode(self) -> Option<FilterMode> {
		match self {
			SessionMode::Standard => None,
			SessionMode::Solo => Some(FilterMode::Solo),
			SessionMode::Locked => Some(FilterMode::Locked),
			SessionMode::Disconnect => Some(FilterMode::Disconnect),
		}
	}

	/// Display name for the mode.
	#[must_use]
	pub fn name(&self) -> &'static str {
		match self {
			SessionMode::Standard => "Standard",
			SessionMode::Solo => "Solo",
			SessionMode::Locked => "Locked",
			SessionMode::Disconnect => "Disconnect",
		}
	}

	/// Short description of the mode.
	#[must_use]
	pub fn description(&self) -> &'static str {
		match self {
			SessionMode::Standard => "Unfiltered traffic",
			SessionMode::Solo => "Strictest firewall",
			SessionMode::Locked => "Blocks new connections",
			SessionMode::Disconnect => "Blocks all traffic",
		}
	}
}
