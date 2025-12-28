//! LobbyGuard common library.
//!
//! Provides the guard engine and configuration types for traffic filtering.

pub mod config;
pub mod engine;
pub mod events;
pub mod flow;
pub mod monitor;
pub mod process;

// Re-exports for convenience
pub use config::{shared_config, FilterMode, GuardConfig, SharedConfig};
pub use engine::{is_heartbeat, PacketGuard, HEARTBEAT_SIZES};
pub use events::{AppEvent, GuardEvent, MonitorEvent};
pub use flow::FlowKey;
pub use monitor::FlowMonitor;
pub use windivert::prelude::*;
