//! LobbyGuard GUI with runtime configuration support.

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use lobbyguard_common::{
	FilterMode, FlowMonitor, GuardConfig, GuardEvent, PacketGuard, SharedConfig, shared_config,
};
use xilem::view::{FlexExt as _, button, flex_col, flex_row, label};
use xilem::winit::error::EventLoopError;
use xilem::{EventLoop, WidgetView, WindowOptions, Xilem, tokio};

/// Application state.
struct AppState {
	is_active: Arc<AtomicBool>,
	blocked_count: Arc<AtomicU64>,
	error: String,
	monitor: Arc<FlowMonitor>,
	packet_guard: Arc<Mutex<Option<Arc<PacketGuard>>>>,
	guard_tx: kanal::Sender<GuardEvent>,
	/// Shared config for hot-switching
	shared_config: SharedConfig,
	/// UI state for filter mode selection
	filter_mode_index: usize,
}

const FILTER_MODES: [(&str, FilterMode); 3] = [
	("Solo", FilterMode::Solo),
	("Locked", FilterMode::Locked),
	("Disconnect", FilterMode::Disconnect),
];

impl AppState {
	fn new(
		monitor: Arc<FlowMonitor>, guard_tx: kanal::Sender<GuardEvent>, shared_config: SharedConfig,
	) -> Self {
		Self {
			is_active: Arc::new(AtomicBool::new(false)),
			blocked_count: Arc::new(AtomicU64::new(0)),
			error: String::new(),
			monitor,
			packet_guard: Arc::new(Mutex::new(None)),
			guard_tx,
			shared_config,
			filter_mode_index: 0,
		}
	}

	fn toggle_filter_mode(&mut self) {
		self.filter_mode_index = (self.filter_mode_index + 1) % FILTER_MODES.len();
		let (_, mode) = FILTER_MODES[self.filter_mode_index];

		// Update config
		self.shared_config.write().filter_mode = mode;

		// Restart engine if it's running (WinDivert filter requires restart)
		let was_active = self.is_active.load(Ordering::Relaxed);
		if was_active {
			self.stop();
			// Small delay to ensure clean shutdown

			self.start();
		}
	}

	fn current_filter_label(&self) -> &'static str { FILTER_MODES[self.filter_mode_index].0 }

	fn start(&mut self) {
		let mut lock = self.packet_guard.lock().unwrap();
		if lock.is_some() {
			return;
		}

		// Reset counters on start? Or keep cumulative?
		// self.blocked_count.store(0, Ordering::Relaxed);

		let guard = match PacketGuard::new(
			self.shared_config.clone(),
			self.monitor.gta_flows.clone(),
			self.monitor.gta_pids.clone(),
			Some(self.guard_tx.clone()),
		) {
			Ok(g) => Arc::new(g),
			Err(e) => {
				self.error = format!("Failed to start: {}", e);
				return;
			}
		};

		*lock = Some(guard.clone());
		self.is_active.store(true, Ordering::Relaxed);
		self.error.clear();

		// Clone handles for background thread
		let is_active = Arc::clone(&self.is_active);
		let packet_guard_store = Arc::clone(&self.packet_guard);

		// Spawn blocking task for PacketGuard::run
		// Spawn blocking task for PacketGuard::run
		tokio::task::spawn_blocking(move || {
			guard.run();
			// When run returns, it stopped
			is_active.store(false, Ordering::Relaxed);
			*packet_guard_store.lock().unwrap() = None;
		});
	}

	fn stop(&mut self) {
		if let Some(guard) = self.packet_guard.lock().unwrap().take() {
			let _ = guard.shutdown();
		}
		self.is_active.store(false, Ordering::Relaxed);
		self.error.clear();
	}
}

fn app_logic(state: &mut AppState) -> impl WidgetView<AppState> + use<> {
	let active = state.is_active.load(Ordering::Relaxed);
	let blocked = state.blocked_count.load(Ordering::Relaxed);
	let filter_label = state.current_filter_label();
	let error_msg = state.error.clone();

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
		// Filter mode control
		flex_row((
			label(format!("Filter: {}", filter_label))
				.text_size(14.0)
				.flex(1.0),
			button(label("Toggle"), |state: &mut AppState| {
				state.toggle_filter_mode()
			})
			.flex(0.0),
		))
		.flex(0.0),
		if !error_msg.is_empty() {
			label(format!("⚠️ Error: {}", error_msg))
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

fn main() -> Result<(), EventLoopError> {
	// Let xilem start the tokio runtime by using new_simple,
	// but we need a handle to it to spawn background tasks.
	// Actually, we'll create the runtime and pass it to xilem to stay in control
	// of the background tasks while fulfilling the user's request.
	let rt =
		std::sync::Arc::new(tokio::runtime::Runtime::new().expect("Failed to create tokio runtime"));
	let _guard = rt.enter();
	// Initialize persistent components
	let config = GuardConfig::default();
	let shared_cfg = shared_config(config);

	let monitor =
		Arc::new(FlowMonitor::new(shared_cfg.clone()).expect("Failed to create FlowMonitor"));
	let (monitor_tx, monitor_rx) = kanal::unbounded();
	monitor.start(monitor_tx);

	let (guard_tx, guard_rx) = kanal::unbounded();

	let state = AppState::new(monitor, guard_tx, shared_cfg);

	// Access to atomics for background loop
	let blocked_count = state.blocked_count.clone();

	// Background event consumer
	tokio::spawn(async move {
		loop {
			tokio::select! {
				Ok(_) = monitor_rx.as_async().recv() => {
					// Update UI for process/flow stats if we had a UI for it
				}
				Ok(event) = guard_rx.as_async().recv() => {
					match event {
						GuardEvent::PacketBlocked => {
							blocked_count.fetch_add(1, Ordering::Relaxed);
						},
						_ => {}
					}
				}
			}
		}
	});

	let app = Xilem::new_simple_with_tokio(state, app_logic, WindowOptions::new("LobbyGuard"), rt);
	app.run_in(EventLoop::with_user_event())
}
