//! Configuration types for LobbyGuard.

use std::sync::Arc;

/// Matchmaking packet sizes to block in Locked mode.
pub const MATCHMAKING_SIZES: [usize; 4] = [191, 207, 223, 239];

/// Filter mode: which session type to enforce.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FilterMode {
	/// Solo session: Only allow heartbeat packets (sizes 12, 18, 63).
	/// Most restrictive - blocks all join requests and existing connections.
	Solo,

	/// Locked session: Block new join requests but allow existing connections.
	/// Blocks matchmaking packets (sizes 191, 207, 223, 239).
	#[default]
	Locked,

	/// Disconnect: Block ALL TCP and UDP traffic to/from GTA executables.
	/// Complete network isolation.
	Disconnect,
}

impl FilterMode {
	/// Returns the WinDivert filter string for this mode.
	#[must_use]
	pub fn to_divert_filter(&self) -> &'static str {
		match self {
			// Solo and Locked both use UDP port 6672 filter
			FilterMode::Solo | FilterMode::Locked => {
				"udp.DstPort == 6672 and udp.PayloadLength > 0 and ip"
			}
			// Disconnect blocks all IP traffic
			FilterMode::Disconnect => "ip",
		}
	}
}

/// Runtime configuration for the guard engine.
#[derive(Debug, Clone)]
pub struct GuardConfig {
	/// Executable names to monitor (case-insensitive).
	pub executable_names: Vec<String>,
	/// Filter mode for packet interception.
	pub filter_mode: FilterMode,
}

impl Default for GuardConfig {
	fn default() -> Self {
		Self {
			executable_names: vec!["GTA5.exe".to_owned(), "GTA5_Enhanced.exe".to_owned()],
			filter_mode: FilterMode::default(),
		}
	}
}

impl GuardConfig {
	/// Create a new config with default values.
	#[must_use]
	pub fn new() -> Self { Self::default() }

	/// Set executable names to monitor.
	#[must_use]
	pub fn with_executables(mut self, names: Vec<String>) -> Self {
		self.executable_names = names;
		self
	}

	/// Set the filter mode.
	#[must_use]
	pub fn with_filter_mode(mut self, mode: FilterMode) -> Self {
		self.filter_mode = mode;
		self
	}

	/// Check if a process name matches any configured executable.
	pub fn matches_executable(&self, name: &str) -> bool {
		let name_lower = name.to_lowercase();
		self
			.executable_names
			.iter()
			.any(|exe| exe.to_lowercase() == name_lower)
	}
}

/// Shared configuration handle for hot-switching.
pub type SharedConfig = Arc<parking_lot::RwLock<GuardConfig>>;

/// Create a new shared configuration.
#[must_use]
pub fn shared_config(config: GuardConfig) -> SharedConfig {
	Arc::new(parking_lot::RwLock::new(config))
}
