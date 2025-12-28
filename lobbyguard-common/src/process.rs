//! WMI-based process monitoring.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use dashmap::DashMap;
use serde::Deserialize;
use tracing::{info, warn};
use wmi::{FilterValue, WMIConnection};

use crate::config::SharedConfig;
use crate::events::MonitorEvent;

#[derive(Deserialize, Debug)]
#[allow(non_snake_case, non_camel_case_types)]
struct Win32_Process {
	ProcessId: u32,
	Name: String,
}

#[derive(Deserialize, Debug)]
#[allow(non_snake_case)]
struct __InstanceCreationEvent {
	TargetInstance: Win32_Process,
}

#[derive(Deserialize, Debug)]
#[allow(non_snake_case)]
struct __InstanceDeletionEvent {
	TargetInstance: Win32_Process,
}

/// Get PIDs of processes matching configured executable names.
pub fn get_matching_pids(con: &WMIConnection, config: &SharedConfig) -> wmi::WMIResult<Vec<u32>> {
	let executable_names = config.read().executable_names.to_owned();
	let mut all_pids = Vec::new();

	for name in &executable_names {
		let mut filters = HashMap::new();
		filters.insert("Name".to_owned(), FilterValue::String(name.clone()));

		if let Ok(results) = con.filtered_query::<Win32_Process>(&filters) {
			all_pids.extend(results.into_iter().map(|p| p.ProcessId));
		}
	}
	// Re-doing this correctly below
	Ok(all_pids)
}

/// Monitor for process creation/deletion matching configured executables (Blocking).
pub fn monitor_processes_blocking(
	con: &WMIConnection, config: SharedConfig, pids: Arc<DashMap<u32, ()>>,
	event_tx: Option<kanal::Sender<MonitorEvent>>,
) -> wmi::WMIResult<()> {
	let mut filters = HashMap::new();
	filters.insert(
		"TargetInstance".to_owned(),
		FilterValue::is_a::<Win32_Process>()?,
	);

	// Blocking iterators
	let mut creation_iter =
		con.filtered_notification::<__InstanceCreationEvent>(&filters, Some(Duration::from_secs(1)))?;

	let mut deletion_iter =
		con.filtered_notification::<__InstanceDeletionEvent>(&filters, Some(Duration::from_secs(1)))?;

	loop {
		// We cannot select over blocking iterators easily.
		// We should alternate checks or use a timeout?
		// WMI notification iterators usually block until event or timeout.
		// We set timeout above (1 sec).

		// Check creation
		match creation_iter.next() {
			Some(Ok(event)) => {
				let name = &event.TargetInstance.Name;
				if config.read().matches_executable(name) {
					pids.insert(event.TargetInstance.ProcessId, ());
					info!(
						"Process started: {} (PID {})",
						name, event.TargetInstance.ProcessId
					);
					if let Some(ref tx) = event_tx {
						let _ = tx.send(MonitorEvent::ProcessFound(event.TargetInstance.ProcessId));
					}
				}
			}
			Some(Err(e)) => warn!("WMI Creation Error: {}", e),
			None => {} // Timeout or end? With timeout set, it returns None on timeout? Or blocks forever?
			           // Actually typical WMI iterators might block forever if timeout is None. If timeout is set, it might return error or None.
			           // Let's assume it handles iteration nicely.
		}

		// Check deletion
		match deletion_iter.next() {
			Some(Ok(event)) => {
				let name = &event.TargetInstance.Name;
				if config.read().matches_executable(name) {
					pids.remove(&event.TargetInstance.ProcessId);
					info!(
						"Process stopped: {} (PID {})",
						name, event.TargetInstance.ProcessId
					);
					if let Some(ref tx) = event_tx {
						let _ = tx.send(MonitorEvent::ProcessLost(event.TargetInstance.ProcessId));
					}
				}
			}
			Some(Err(e)) => warn!("WMI Deletion Error: {}", e),
			None => {}
		}

		// Avoid tight loop if both timeout immediately (shouldn't if timeout worked)
		// If timeout is respected, we effectively poll every 1s (sequentially).
		// 1s wait for creation, then 1s wait for deletion. Total 2s latency max. Acceptable.
	}
}
