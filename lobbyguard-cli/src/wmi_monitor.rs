use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use futures::StreamExt;
use log::{debug, info, trace};

use crate::connection_tracker::ConnectionTracker;
use crate::wmi::models::*;

/// Initialize WMI connections and query existing processes/connections
pub fn initialize_wmi(
	tracker: Arc<ConnectionTracker>,
) -> Result<(wmi::WMIConnection, wmi::WMIConnection), Box<dyn std::error::Error>> {
	let default_con = wmi::WMIConnection::new()?;
	let standard_con = wmi::WMIConnection::with_namespace_path("ROOT\\StandardCIMV2")?;

	// Find existing GTA processes
	let mut filters = HashMap::new();
	filters.insert(
		"Name".to_owned(),
		wmi::FilterValue::Str("GTA5_Enhanced.exe"),
	);
	if let Ok(processes) = default_con.filtered_query::<Process>(&filters) {
		for process in processes {
			info!("Found process: {} ({})", process.name, process.process_id);
			tracker.add_process(process.process_id);
		}
	}

	// Query existing TCP connections for tracked processes
	if let Ok(tcps) = standard_con.query::<NetTCPConnection>() {
		let count = tcps.len();
		debug!("Queried {} TCP connections from WMI", count);
		for tcp in tcps {
			if tracker.contains_process(tcp.owning_process) {
				tracker.add_tcp_connection(
					tcp.owning_process,
					tcp.local_port,
					tcp.remote_port,
				);
			}
		}
	}

	// Query existing UDP endpoints for tracked processes
	if let Ok(udps) = standard_con.query::<NetUDPEndpoint>() {
		let count = udps.len();
		debug!("Queried {} UDP endpoints from WMI", count);
		for udp in udps {
			if tracker.contains_process(udp.owning_process) {
				tracker.add_udp_endpoint(udp.owning_process, udp.local_port);
			}
		}
	}

	Ok((default_con, standard_con))
}

/// Run the WMI event monitoring loop
pub async fn run_wmi_monitor(
	default_con: wmi::WMIConnection, standard_con: wmi::WMIConnection,
	tracker: Arc<ConnectionTracker>,
) -> Result<(), Box<dyn std::error::Error>> {
	// Set up process event streams
	let mut filters = HashMap::new();
	filters.insert(
		"TargetInstance".to_owned(),
		wmi::FilterValue::is_a::<Process>()?,
	);
	let mut process_create_events = default_con
		.async_filtered_notification::<ProcessOpenEvent>(&filters, Some(Duration::from_secs(1)))?;
	let mut process_delete_events = default_con
		.async_filtered_notification::<ProcessCloseEvent>(&filters, Some(Duration::from_secs(1)))?;

	// Set up UDP connection event streams
	let mut filters = HashMap::new();
	filters.insert(
		"TargetInstance".to_owned(),
		wmi::FilterValue::is_a::<NetUDPEndpoint>()?,
	);
	let mut udp_create_events = standard_con.async_filtered_notification::<UDPInstCreateEvent>(
		&filters,
		Some(Duration::from_secs(1)),
	)?;
	let mut udp_delete_events = standard_con.async_filtered_notification::<UDPInstDeleteEvent>(
		&filters,
		Some(Duration::from_secs(1)),
	)?;
	let mut udp_update_events = standard_con
		.async_filtered_notification::<UDPInstModifyEvent>(
			&filters,
			Some(Duration::from_secs(1)),
		)?;

	// Set up TCP connection event streams
	let mut filters = HashMap::new();
	filters.insert(
		"TargetInstance".to_owned(),
		wmi::FilterValue::is_a::<NetTCPConnection>()?,
	);
	let mut tcp_create_events = standard_con.async_filtered_notification::<TCPInstCreateEvent>(
		&filters,
		Some(Duration::from_secs(1)),
	)?;
	let mut tcp_delete_events = standard_con.async_filtered_notification::<TCPInstDeleteEvent>(
		&filters,
		Some(Duration::from_secs(1)),
	)?;
	let mut tcp_update_events = standard_con
		.async_filtered_notification::<TCPInstModifyEvent>(
			&filters,
			Some(Duration::from_secs(1)),
		)?;

	info!("Press Ctrl-C to exit.");

	// Notes: 
	// WMI connection instance are modified when the connection is closed instead of deleted
	// When being deleted the instance is just contain zeros for address and port
	loop {
		tokio::select! {
			Some(Ok(event)) = process_create_events.next() => {
				let process = event.target_instance;
				let process_id = process.process_id;
				let process_name = process.name;
				if process_name == "GTA5_Enhanced.exe" {
					info!("Process {} ({}) created", process_name, process_id);
					tracker.add_process(process_id);
				}
			}
			Some(Ok(event)) = process_delete_events.next() => {
				let process = event.target_instance;
				let process_id = process.process_id;
				let process_name = process.name;
				if process_name == "GTA5_Enhanced.exe" {
					info!("Process {} ({}) deleted", process_name, process_id);
				}
				tracker.remove_process(process_id);
			}
			Some(Ok(event)) = udp_create_events.next() => {
				let udp = event.target_instance;
				if tracker.contains_process(udp.owning_process) {
					trace!("UDP connection created for PID {:?}", udp);
					tracker.add_udp_endpoint(udp.owning_process, udp.local_port);
				}
			}
			Some(Ok(event)) = udp_delete_events.next() => {
				let udp = event.target_instance;
				if tracker.contains_process(udp.owning_process) {
					trace!("UDP connection deleted for PID {:?}", udp);
					tracker.remove_udp_endpoint(udp.owning_process, udp.local_port);
				}
			}
			Some(Ok(event)) = udp_update_events.next() => {
				let udp = event.target_instance;
				let previous_udp = event.previous_instance;
				if tracker.contains_process(previous_udp.owning_process) {
					trace!("UDP connection updated for PID {:?}->{:?}", previous_udp, udp);
					tracker.remove_udp_endpoint(previous_udp.owning_process, previous_udp.local_port);
					if tracker.contains_process(udp.owning_process) {
						tracker.add_udp_endpoint(udp.owning_process, udp.local_port);
					}
				}
			}
			Some(Ok(event)) = tcp_create_events.next() => {
				let tcp = event.target_instance;
				if tracker.contains_process(tcp.owning_process) {
					trace!("TCP connection created for PID {:?}", tcp);
					tracker.add_tcp_connection(
						tcp.owning_process,
						tcp.local_port,
						tcp.remote_port,
					);
				}
			}
			Some(Ok(event)) = tcp_delete_events.next() => {
				let tcp = event.target_instance;
				if tracker.contains_process(tcp.owning_process) {
					trace!("TCP connection deleted for PID {:?}", tcp);
					tracker.remove_tcp_connection(
						tcp.owning_process,
						tcp.local_port,
						tcp.remote_port,
					);
				}
			}
			Some(Ok(event)) = tcp_update_events.next() => {
				let tcp = event.target_instance;
				let previous_tcp = event.previous_instance;
				if previous_tcp.owning_process != tcp.owning_process && tracker.contains_process(previous_tcp.owning_process) {
					trace!("TCP connection updated for PID {:?}->{:?}", previous_tcp, tcp);
						tracker.remove_tcp_connection(
							previous_tcp.owning_process,
							previous_tcp.local_port,
							previous_tcp.remote_port,
						);
					if tracker.contains_process(tcp.owning_process) {
						tracker.add_tcp_connection(
							tcp.owning_process,
							tcp.local_port,
							tcp.remote_port,
						);
					}
				}
			}
			_ = tokio::signal::ctrl_c() => {
				info!("Ctrl-C received! Exiting gracefully.");
				break;
			}
		}
	}

	Ok(())
}
