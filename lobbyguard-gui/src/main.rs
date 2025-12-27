//! LobbyGuard GUI - Simplified version with direct WinDivert integration.

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use lobbyguard_common::GuardEngine;
use xilem::view::{FlexExt as _, button, flex_col, flex_row, label};
use xilem::{EventLoop, WidgetView, WindowOptions, Xilem};

/// Application state.
struct AppState {
	is_active: Arc<AtomicBool>,
	blocked_count: Arc<AtomicU64>,
	error: String,
	engine: Arc<Mutex<Option<Arc<GuardEngine>>>>,
}

impl AppState {
	fn new() -> Self {
		Self {
			is_active: Arc::new(AtomicBool::new(false)),
			blocked_count: Arc::new(AtomicU64::new(0)),
			error: String::new(),
			engine: Arc::new(Mutex::new(None)),
		}
	}

	fn start(&mut self) {
		let mut lock = self.engine.lock().unwrap();
		if lock.is_some() {
			return;
		}

		let engine = match GuardEngine::start() {
			Ok(e) => Arc::new(e),
			Err(e) => {
				self.error = format!("Failed to start: {}", e);
				return;
			}
		};

		*lock = Some(engine.clone());
		self.is_active.store(true, Ordering::Relaxed);
		self.error.clear();

		// Clone handles for background thread
		let is_active = Arc::clone(&self.is_active);
		let blocked_count = Arc::clone(&self.blocked_count);
		let engine_store = Arc::clone(&self.engine);

		tokio::spawn(async move {
			tokio::task::block_in_place(|| engine.run(blocked_count));
			is_active.store(false, Ordering::Relaxed);
			*engine_store.lock().unwrap() = None;
		});
	}

	fn stop(&mut self) {
		if let Some(engine) = self.engine.lock().unwrap().take() {
			let _ = engine.shutdown();
		}
		self.is_active.store(false, Ordering::Relaxed);
		self.error.clear();
	}
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
