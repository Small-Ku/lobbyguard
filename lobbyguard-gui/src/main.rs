//! LobbyGuard GUI - Simplified version with direct WinDivert integration.

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use etherparse::{Ipv4Slice, UdpSlice};
use windivert::prelude::*;
use xilem::view::{FlexExt as _, button, flex_col, flex_row, label};
use xilem::{EventLoop, WidgetView, WindowOptions, Xilem};

const HEARTBEAT_SIZES: [usize; 3] = [12, 18, 63];

/// Application state.
struct AppState {
	is_active: Arc<AtomicBool>,
	blocked_count: Arc<AtomicU64>,
	error: String,
	shutdown_handle: Arc<Mutex<Option<ShutdownHandle>>>,
}

impl AppState {
	fn new() -> Self {
		Self {
			is_active: Arc::new(AtomicBool::new(false)),
			blocked_count: Arc::new(AtomicU64::new(0)),
			error: String::new(),
			shutdown_handle: Arc::new(Mutex::new(None)),
		}
	}

	fn start(&mut self) {
		let mut lock = self.shutdown_handle.lock().unwrap();
		if lock.is_some() {
			return;
		}

		let divert = match WinDivert::<NetworkLayer>::network(
			"udp.DstPort == 6672 and udp.PayloadLength > 0 and ip",
			0,
			Default::default(),
		) {
			Ok(d) => d,
			Err(e) => {
				self.error = format!("Failed to start: {}", e);
				return;
			}
		};

		let handle = divert.shutdown_handle();
		*lock = Some(handle);
		self.is_active.store(true, Ordering::Relaxed);
		self.error.clear();

		// Clone handles for background thread
		let is_active = Arc::clone(&self.is_active);
		let blocked_count = Arc::clone(&self.blocked_count);
		let shutdown_handle = Arc::clone(&self.shutdown_handle);

		tokio::spawn(async move {
			let mut buffer = [0u8; 1500];
			loop {
				// Use block_in_place as divert.recv is a blocking operation
				let result = tokio::task::block_in_place(|| divert.recv(&mut buffer));
				match result {
					Ok(packet) => {
						if is_heartbeat(&packet.data) {
							if let Err(e) = divert.send(&packet) {
								eprintln!("[Service] Send error: {}", e);
							}
						} else {
							blocked_count.fetch_add(1, Ordering::Relaxed);
						}
					}
					Err(_) => break, // Error or shutdown signaled
				}
			}
			is_active.store(false, Ordering::Relaxed);
			*shutdown_handle.lock().unwrap() = None;
		});
	}

	fn stop(&mut self) {
		if let Some(handle) = self.shutdown_handle.lock().unwrap().take() {
			let _ = handle.shutdown();
		}
		self.is_active.store(false, Ordering::Relaxed);
		self.error.clear();
	}
}

/// Helper to determine if a packet is a heartbeat.
fn is_heartbeat(data: &[u8]) -> bool {
	let Ok(ip) = Ipv4Slice::from_slice(data) else {
		return false;
	};
	let Ok(udp) = UdpSlice::from_slice(ip.payload().payload) else {
		return false;
	};
	HEARTBEAT_SIZES.contains(&udp.payload().len())
}

fn app_logic(state: &mut AppState) -> impl WidgetView<AppState> + use<> {
	let active = state.is_active.load(Ordering::Relaxed);
	let blocked = state.blocked_count.load(Ordering::Relaxed);

	flex_col((
		label("LOBBY GUARD").text_size(32.0).flex(0.0),
		label(if active {
			"🛡️ PROTECTION ACTIVE"
		} else {
			"⚪ PROTECTION INACTIVE"
		})
		.text_size(18.0)
		.flex(0.0),
		label(format!("Blocked Packets: {}", blocked))
			.text_size(16.0)
			.flex(0.0),
		if !state.error.is_empty() {
			label(format!("⚠️ Error: {}", state.error))
				.text_size(14.0)
				.flex(0.0)
		} else {
			label("").flex(0.0)
		},
		flex_row((
			button(label("START"), |state: &mut AppState| state.start()).flex(1.0),
			button(label("STOP"), |state: &mut AppState| state.stop()).flex(1.0),
		))
		.flex(0.0),
	))
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	let state = AppState::new();
	let app = Xilem::new_simple(state, app_logic, WindowOptions::new("LobbyGuard"));
	app
		.run_in(EventLoop::with_user_event())
		.map_err(|e| e.into())
}
