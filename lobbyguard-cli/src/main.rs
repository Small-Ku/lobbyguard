use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use lobbyguard_common::GuardEngine;
use tokio::time::interval;
use tracing::{error, info};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() {
	tracing_subscriber::fmt()
		.with_env_filter(EnvFilter::from_default_env().add_directive(tracing::Level::INFO.into()))
		.init();

	let engine = match GuardEngine::start() {
		Ok(e) => Arc::new(e),
		Err(e) => {
			error!("Failed to start GuardEngine: {}", e);
			return;
		}
	};

	let blocked_count = Arc::new(AtomicU64::new(0));
	let engine_run = engine.clone();
	let blocked_count_run = blocked_count.clone();

	let handle = tokio::spawn(async move {
		info!("LobbyGuard CLI started. Monitoring traffic...");
		tokio::task::block_in_place(|| engine_run.run(blocked_count_run));
		info!("GuardEngine stopped.");
	});

	let mut stats_interval = interval(Duration::from_secs(5));
	let blocked_count_stats = blocked_count.clone();

	info!("Press Ctrl-C to exit.");

	loop {
		tokio::select! {
				_ = tokio::signal::ctrl_c() => {
						info!("\nCtrl-C received! Exiting gracefully.");
						break;
				}
				_ = stats_interval.tick() => {
						let count = blocked_count_stats.load(Ordering::Relaxed);
						info!("Status: Active | Blocked Packets: {}", count);
				}
		}
	}

	if let Err(e) = engine.shutdown() {
		error!("Failed to shutdown GuardEngine: {}", e);
	}

	let _ = handle.await;
	info!("Goodbye!");
}
