//! Persistent flow monitoring and process tracking.

use std::sync::Arc;

use dashmap::DashMap;
use tracing::{info, warn};
use windivert::prelude::*;
use wmi::WMIConnection;

use crate::config::SharedConfig;
use crate::events::MonitorEvent;
use crate::flow::{normalize_ip, FlowKey};
use crate::process::{get_matching_pids, monitor_processes_blocking};

/// Persistent monitor for GTA flows and processes.
/// This component should remain active throughout the application lifecycle
/// to ensure we don't lose flow context during filter mode switches.
pub struct FlowMonitor {
	config: SharedConfig,
	flow_divert: Arc<WinDivert<FlowLayer>>,
	pub gta_flows: Arc<DashMap<FlowKey, ()>>,
	pub gta_pids: Arc<DashMap<u32, ()>>,
}

impl FlowMonitor {
	/// Create a new flow monitor.
	pub fn new(config: SharedConfig) -> Result<Self, WinDivertError> {
		// Listen for flows to identify target process connections
		let flow_divert = WinDivert::<FlowLayer>::flow("ip", 0, Default::default())?;

		Ok(Self {
			config,
			flow_divert: Arc::new(flow_divert),
			gta_flows: Arc::new(DashMap::new()),
			gta_pids: Arc::new(DashMap::new()),
		})
	}

	/// Run the monitor. This spawns background tasks on the CURRENT Tokio runtime.
	/// Returns immediately.
	pub fn start(&self, event_tx: kanal::Sender<MonitorEvent>) {
		let gta_flows = self.gta_flows.clone();
		let gta_pids = self.gta_pids.clone();
		let flow_divert = self.flow_divert.clone();
		let config = self.config.clone();
		let event_tx_pids = event_tx.clone();

		// Spawn WMI monitoring task in a blocking task (handled by Tokio thread pool)
		// We use spawn_blocking because WMI calls are blocking and WMIConnection structure is !Send/!Sync for async usage across threads usually,
		// but inside spawn_blocking it runs on a dedicated thread and should be fine if created there.
		tokio::task::spawn_blocking(move || {
			let Ok(con) = WMIConnection::new() else {
				warn!("Failed to create WMI connection");
				return;
			};

			// Initial scan
			if let Ok(pids) = get_matching_pids(&con, &config) {
				for pid in pids {
					gta_pids.insert(pid, ());
					let _ = event_tx_pids.send(MonitorEvent::ProcessFound(pid));
				}
				info!("Initial scan found {} matching processes", gta_pids.len());
			}

			// Continuous monitoring (Blocking loop)
			let _ = monitor_processes_blocking(&con, config, gta_pids, Some(event_tx_pids));
		});

		// Spawn Flow monitoring task
		// WinDivert recv is blocking. We use spawn_blocking to avoid blocking the async runtime.
		let gta_pids_flow = self.gta_pids.clone();
		let event_tx_flow = event_tx.clone();

		tokio::task::spawn_blocking(move || loop {
			match flow_divert.recv() {
				Ok(packet) => {
					let flow = packet.address;

					let key = FlowKey {
						local_addr: normalize_ip(flow.local_address()),
						local_port: flow.local_port(),
						remote_addr: normalize_ip(flow.remote_address()),
						remote_port: flow.remote_port(),
					};

					match flow.event() {
						WinDivertEvent::FlowEstablished => {
							if gta_pids_flow.contains_key(&flow.process_id()) {
								gta_flows.insert(key, ());
								let _ = event_tx_flow.send(MonitorEvent::FlowEstablished(key));
							}
						}
						WinDivertEvent::FlowDeleted => {
							if gta_flows.remove(&key).is_some() {
								let _ = event_tx_flow.send(MonitorEvent::FlowDeleted(key));
							}
						}
						_ => {}
					}
				}
				Err(_) => break,
			}
		});
	}
}
