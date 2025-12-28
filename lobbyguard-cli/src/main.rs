use std::sync::Arc;

use clap::Parser;
use lobbyguard_common::{
	FilterMode, FlowMonitor, GuardConfig, GuardEvent, MonitorEvent, PacketGuard, shared_config,
};
use tokio::signal;
use tracing::{error, info, warn};
use tracing_subscriber::EnvFilter;

#[derive(Parser, Debug)]
#[command(
	name = "lobbyguard-cli",
	about = "LobbyGuard CLI - Traffic filter for GTA Online"
)]
struct Args {
	/// Filter mode: "solo" for absolute solo lobby, "locked" for no new players, "disconnect" for blocking all GTA traffic
	#[arg(short, long, default_value = "solo")]
	filter_mode: String,

	/// Additional executable names to monitor (can be specified multiple times)
	#[arg(short, long)]
	executable: Vec<String>,
}

#[tokio::main]
async fn main() {
	tracing_subscriber::fmt()
		.with_env_filter(EnvFilter::from_default_env().add_directive(tracing::Level::INFO.into()))
		.init();

	let args = Args::parse();

	// Build configuration
	let filter_mode = match args.filter_mode.to_lowercase().as_str() {
		"solo" => FilterMode::Solo,
		"locked" => FilterMode::Locked,
		"disconnect" => FilterMode::Disconnect,
		_ => FilterMode::Solo,
	};

	let config = if args.executable.is_empty() {
		GuardConfig::new().with_filter_mode(filter_mode)
	} else {
		GuardConfig::new()
			.with_executables(args.executable)
			.with_filter_mode(filter_mode)
	};

	info!("Configuration: {:?}", config);
	let shared_cfg = shared_config(config);

	// Start FlowMonitor (Persistent)
	let monitor = match FlowMonitor::new(shared_cfg.clone()) {
		Ok(m) => Arc::new(m),
		Err(e) => {
			error!("Failed to initialize FlowMonitor: {}", e);
			return;
		}
	};

	let (monitor_tx, monitor_rx) = kanal::unbounded();
	monitor.start(monitor_tx);

	// Start PacketGuard (Ephemeral-ish, though strict separation here isn't dynamic in CLI)
	let (guard_tx, guard_rx) = kanal::unbounded();
	let guard = match PacketGuard::new(
		shared_cfg.clone(),
		monitor.gta_flows.clone(),
		monitor.gta_pids.clone(),
		Some(guard_tx),
	) {
		Ok(g) => Arc::new(g),
		Err(e) => {
			error!("Failed to start PacketGuard: {}", e);
			return;
		}
	};

	let guard_run = guard.clone();
	let handle = tokio::task::spawn_blocking(move || {
		info!("LobbyGuard CLI started. Monitoring traffic...");
		guard_run.run();
		info!("PacketGuard stopped.");
	});

	info!("Press Ctrl-C to exit.");

	let mut blocked_count = 0u64;

	loop {
		tokio::select! {
			_ = signal::ctrl_c() => {
				info!("\nCtrl-C received! Exiting gracefully.");
				break;
			}
			Ok(event) = monitor_rx.as_async().recv() => {
				match event {
					MonitorEvent::ProcessFound(pid) => info!("Monitor: Process found (PID {})", pid),
					MonitorEvent::ProcessLost(pid) => info!("Monitor: Process lost (PID {})", pid),
					MonitorEvent::FlowEstablished(key) => info!("Monitor: Flow established ({:?})", key),
					MonitorEvent::FlowDeleted(key) => info!("Monitor: Flow deleted ({:?})", key),
				}
			}
			Ok(event) = guard_rx.as_async().recv() => {
				match event {
					GuardEvent::PacketBlocked => {
						blocked_count += 1;
						// Throttle logging?
						if blocked_count % 10 == 0 {
							info!("Guard: Blocked {} packets total", blocked_count);
						}
					}
					GuardEvent::PacketAllowed => {}, // Not emitted currently
					GuardEvent::EngineStopped => warn!("Guard: Engine stopped unexpected"),
				}
			}
		}
	}

	if let Err(e) = guard.shutdown() {
		error!("Failed to shutdown GuardEngine: {}", e);
	}

	let _ = handle.await; // or handle.await if I used spawn_blocking
	info!("Goodbye!");
}
