//! LobbyGuard GUI - Network session manager for GTA Online.
//!
//! Redesigned with modular components, action-based navigation,
//! and real-time updates via async workers.

mod actions;
mod components;
mod theme;
mod view;

use std::sync::{Arc, Mutex};
use std::time::Instant;

pub use actions::{Action, Page, SessionMode};
use lobbyguard_common::config::WhitelistEntry;
use lobbyguard_common::{
	FlowMonitor, GuardConfig, GuardEvent, MonitorEvent, PacketGuard, SharedConfig, shared_config,
};
pub use theme::{Theme, ThemeMode};
use xilem::core::map_action;
use xilem::winit::error::EventLoopError;
use xilem::{EventLoop, WidgetView, WindowOptions, Xilem, tokio};

/// Application state.
pub struct AppState {
	// UI State
	pub page: Page,
	pub session_mode: SessionMode,
	pub theme_mode: ThemeMode,

	// Engine State
	pub monitor: Arc<FlowMonitor>,
	pub packet_guard: Arc<Mutex<Option<Arc<PacketGuard>>>>,
	pub shared_config: SharedConfig,
	pub guard_tx: kanal::Sender<GuardEvent>,

	// Real-time stats (plain types updated by workers)
	pub is_active: bool,
	pub blocked_count: u64,
	pub flow_count: usize,
	pub process_count: usize,
	pub start_time: Option<Instant>,

	// Form state
	pub new_name: String,
	pub new_ip: String,

	// Event channels for workers
	pub monitor_rx: kanal::Receiver<MonitorEvent>,
	pub guard_rx: kanal::Receiver<GuardEvent>,
}

impl AppState {
	fn new(
		monitor: Arc<FlowMonitor>, monitor_rx: kanal::Receiver<MonitorEvent>,
		guard_tx: kanal::Sender<GuardEvent>, guard_rx: kanal::Receiver<GuardEvent>,
		shared_config: SharedConfig,
	) -> Self {
		Self {
			page: Page::default(),
			session_mode: SessionMode::default(),
			theme_mode: ThemeMode::default(),

			monitor,
			packet_guard: Arc::new(Mutex::new(None)),
			shared_config,
			guard_tx,

			is_active: false,
			blocked_count: 0,
			flow_count: 0,
			process_count: 0,
			start_time: None,

			new_name: String::new(),
			new_ip: String::new(),

			monitor_rx,
			guard_rx,
		}
	}

	/// Get the current theme.
	pub fn theme(&self) -> Theme { Theme::for_mode(self.theme_mode) }

	/// Handle an action.
	pub fn handle_action(&mut self, action: Action) {
		match action {
			Action::Navigate(page) => self.page = page,
			Action::SetMode(mode) => self.set_mode(mode),
			Action::ToggleTheme => self.theme_mode = self.theme_mode.toggle(),
			Action::AddWhitelist { name, ip } => {
				self.new_name = name;
				self.new_ip = ip;
				self.add_whitelist();
			}
			Action::RemoveWhitelist(id) => self.remove_whitelist(id),
			Action::None => {}
		}
	}

	/// Set the session mode, starting/stopping the engine as needed.
	pub fn set_mode(&mut self, mode: SessionMode) {
		self.session_mode = mode;

		match mode.to_filter_mode() {
			None => {
				// Standard mode - stop engine
				self.stop();
			}
			Some(filter_mode) => {
				// Update config
				self.shared_config.write().filter_mode = filter_mode;

				// Restart if running, or start if not
				if self.is_active {
					self.stop();
					self.start();
				} else {
					self.start();
				}
			}
		}
	}

	// Config helpers
	pub fn get_whitelist(&self) -> Vec<WhitelistEntry> { self.shared_config.read().whitelist.clone() }

	pub fn add_whitelist(&mut self) {
		if self.new_name.is_empty() || self.new_ip.is_empty() {
			return;
		}
		let entry = WhitelistEntry {
			id: std::time::SystemTime::now()
				.duration_since(std::time::UNIX_EPOCH)
				.unwrap()
				.as_nanos() as u64,
			name: self.new_name.clone(),
			ip: self.new_ip.clone(),
		};
		self.shared_config.write().whitelist.push(entry);
		self.new_name.clear();
		self.new_ip.clear();
	}

	pub fn remove_whitelist(&mut self, id: u64) {
		self.shared_config.write().whitelist.retain(|e| e.id != id);
	}

	pub fn get_uptime(&self) -> u64 { self.start_time.map(|t| t.elapsed().as_secs()).unwrap_or(0) }

	fn start(&mut self) {
		let mut lock = self.packet_guard.lock().unwrap();
		if lock.is_some() {
			return;
		}

		let guard = match PacketGuard::new(
			self.shared_config.clone(),
			self.monitor.gta_flows.clone(),
			self.monitor.gta_pids.clone(),
			Some(self.guard_tx.clone()),
		) {
			Ok(g) => Arc::new(g),
			Err(e) => {
				tracing::error!("Failed to start PacketGuard: {}", e);
				return;
			}
		};

		*lock = Some(guard.clone());
		self.is_active = true;
		self.start_time = Some(Instant::now());

		let packet_guard_store = Arc::clone(&self.packet_guard);

		tokio::task::spawn_blocking(move || {
			guard.run();
			*packet_guard_store.lock().unwrap() = None;
		});
	}

	fn stop(&mut self) {
		if let Some(guard) = self.packet_guard.lock().unwrap().take() {
			let _ = guard.shutdown();
		}
		self.is_active = false;
		self.start_time = None;
	}
}

#[derive(Debug, Clone)]
enum WorkerEvent {
	Monitor(MonitorEvent),
	Guard(GuardEvent),
}

/// Main view function with action handling.
fn app_logic(state: &mut AppState) -> impl WidgetView<AppState> + use<> {
	let monitor_rx = state.monitor_rx.clone();
	let guard_rx = state.guard_rx.clone();

	xilem::core::fork(
		map_action(view::app_view(state), |state, action| {
			state.handle_action(action);
		}),
		(xilem::view::worker_raw(
			move |result, _recv: xilem::tokio::sync::mpsc::UnboundedReceiver<()>| {
				let monitor_rx = monitor_rx.clone();
				let guard_rx = guard_rx.clone();

				async move {
					loop {
						tokio::select! {
							event = monitor_rx.as_async().recv() => {
								if let Ok(e) = event {
									let _ = result.message(WorkerEvent::Monitor(e));
								}
							}
							event = guard_rx.as_async().recv() => {
								if let Ok(e) = event {
									let _ = result.message(WorkerEvent::Guard(e));
								}
							}
						}
					}
				}
			},
			|_app_state, _sender| {},
			|state: &mut AppState, event| match event {
				WorkerEvent::Monitor(e) => match e {
					MonitorEvent::FlowEstablished(_) => state.flow_count += 1,
					MonitorEvent::FlowDeleted(_) => state.flow_count = state.flow_count.saturating_sub(1),
					MonitorEvent::ProcessFound(_) => state.process_count += 1,
					MonitorEvent::ProcessLost(_) => {
						state.process_count = state.process_count.saturating_sub(1)
					}
				},
				WorkerEvent::Guard(e) => {
					if let GuardEvent::PacketBlocked = e {
						state.blocked_count += 1;
					}
				}
			},
		),),
	)
}

fn main() -> Result<(), EventLoopError> {
	// Initialize tracing
	tracing_subscriber::fmt::init();

	// Create and enter a Tokio runtime.
	// Entering the runtime on the main thread ensures that any subsequent
	// calls like FlowMonitor::start or PacketGuard::start (triggered via UI)
	// will have an active reactor handle.
	let rt = Arc::new(
		tokio::runtime::Builder::new_multi_thread()
			.enable_all()
			.build()
			.expect("Failed to create Tokio runtime"),
	);
	let _guard = rt.enter();

	let config = GuardConfig::default();
	let shared_cfg = shared_config(config);

	let monitor =
		Arc::new(FlowMonitor::new(shared_cfg.clone()).expect("Failed to create FlowMonitor"));

	let (monitor_tx, monitor_rx) = kanal::unbounded();
	let (guard_tx, guard_rx) = kanal::unbounded();

	// Start monitor immediately (now safe because we entered the runtime)
	monitor.start(monitor_tx);

	let state = AppState::new(monitor, monitor_rx, guard_tx, guard_rx, shared_cfg);

	let app = Xilem::new_simple_with_tokio(state, app_logic, WindowOptions::new("LobbyGuard"), rt);
	app.run_in(EventLoop::with_user_event())
}
