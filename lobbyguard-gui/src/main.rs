#![cfg_attr(
    all(target_os = "windows", not(debug_assertions)),
    windows_subsystem = "windows"
)]
#![doc = include_str!("../README.md")]

//! # LobbyGuard GUI
//!
//! Graphical user interface for the LobbyGuard packet capture system.
//! Built with winio and compio for cross-platform Windows UI.

mod component;
mod error;

use component::MainModel;
use error::Result;
use snafu::ResultExt;
use winio::prelude::*;

/// Main entry point for GUI application
#[snafu::report]
fn main() -> Result<()> {
    App::new("de.kwoo.lobbyguard.winiogui")
        .context(error::NewAppSnafu)?
        .run::<MainModel>(())
}
