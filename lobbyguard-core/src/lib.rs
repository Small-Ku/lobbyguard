//! # LobbyGuard Core Library
//!
//! Shared packet capture and network filtering logic for LobbyGuard CLI and GUI.
//!
//! ## Modules
//!
//! - `capture` - Packet capture and filtering
//! - `error` - Error types and handling
//! - `constants` - Shared constants
//!
//! ## Usage with AI Copilot
//!
//! When adding new features:
//! 1. Define error types in `error.rs` using snafu
//! 2. Create new modules in the appropriate submodule
//! 3. Always use `Result<T>` type alias instead of unwrap/panic
//! 4. Document public APIs with examples
//! 5. Follow the error handling patterns shown in `capture.rs`

pub mod capture;
pub mod constants;
pub mod error;

pub use error::{Error, Result};
