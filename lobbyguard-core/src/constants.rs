//! Shared constants for LobbyGuard
//!
//! Centralized configuration values used across CLI and GUI.

/// Target UDP destination port for packet filtering
pub const TARGET_UDP_PORT: u16 = 6672;

/// Heartbeat packet sizes (in bytes) that indicate valid game heartbeats
/// These sizes are characteristic of the game's heartbeat protocol
pub const HEARTBEAT_SIZES: [usize; 3] = [12, 18, 63];

/// Maximum packet buffer size for capture operations
pub const MAX_PACKET_SIZE: usize = 1500;

/// WinDivert filter string for capturing relevant packets
/// Filters for UDP packets destined to TARGET_UDP_PORT with payload
pub fn divert_filter() -> String {
    format!(
        "udp.DstPort == {} and udp.PayloadLength > 0 and ip",
        TARGET_UDP_PORT
    )
}
