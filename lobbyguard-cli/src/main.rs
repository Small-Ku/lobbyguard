use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use arc_swap::ArcSwap;
use argh::FromArgs;
use fastrace::collector::{Config, ConsoleReporter};
use lobbyguard_cli::connection_tracker::ConnectionTracker;
use lobbyguard_cli::filter::build_dynamic_filter;
use lobbyguard_cli::packet::processor::process_packets;
use lobbyguard_cli::wmi_monitor::{initialize_wmi, run_wmi_monitor};
use log::{debug, error, info};
use logforth::append;
use logforth::filter::env_filter::EnvFilterBuilder;
use windivert::prelude::*;

#[derive(FromArgs)]
/// Block the GTA connections you don't want.
struct Lobbyguard {
	/// optional path to output captured traffic
	#[argh(option, short = 'f')]
	file: Option<PathBuf>,

	/// whether to capture TCP traffic (ports 80 and 443)
	#[argh(option, default = "true")]
	capture_tcp: bool,
}

#[tokio::main]
async fn main() {
	fastrace::set_reporter(ConsoleReporter, Config::default());
	logforth::starter_log::builder()
		.dispatch(|d| {
			d.filter(EnvFilterBuilder::from_default_env_or("info").build())
				.append(append::Stdout::default())
		})
		.dispatch(|d| d.append(append::FastraceEvent::default()))
		.apply();

	let args: Lobbyguard = argh::from_env();

	// Initialize connection tracker
	// Initialize connection tracker
	let (tracker_obj, rebuild_signal) = ConnectionTracker::new();
	let tracker = Arc::new(tracker_obj);

	// Initialize WMI and query existing processes/connections
	let (default_con, standard_con) =
		initialize_wmi(Arc::clone(&tracker)).expect("Failed to initialize WMI connections");

	// Build initial WinDivert filter based on WMI state
	let udp_ports = tracker.get_all_udp_ports();
	let tcp_ports = tracker.get_all_tcp_ports();
	let net_filter = build_dynamic_filter(&udp_ports, &tcp_ports, args.capture_tcp);

	debug!(
		"Creating initial network divert with filter: {}",
		net_filter
	);
	let network_divert = WinDivert::<NetworkLayer>::network(&net_filter, 0, Default::default())
		.expect("Failed to create network layer WinDivert handle.");

	let network_divert_swap = Arc::new(ArcSwap::from_pointee(network_divert));
	let net_shutdown_handle = network_divert_swap.load().shutdown_handle();

	// Spawn background task to watch for filter changes
	let rebuild_rx = rebuild_signal.0.to_async();
	let tracker_filter = Arc::clone(&tracker);
	let swap_handle = Arc::clone(&network_divert_swap);
	let capture_tcp = args.capture_tcp;
	tokio::spawn(async move {
		while rebuild_rx.recv().await.is_ok() {
			// Debounce changes to avoid rapid re-opens
			tokio::time::sleep(Duration::from_millis(500)).await;

			let udp_ports = tracker_filter.get_all_udp_ports();
			let tcp_ports = tracker_filter.get_all_tcp_ports();

			let new_filter = build_dynamic_filter(&udp_ports, &tcp_ports, capture_tcp);
			info!(
				"Connection change detected, rebuilding filter: {}",
				new_filter
			);

			match WinDivert::<NetworkLayer>::network(&new_filter, 0, Default::default()) {
				Ok(new_divert) => {
					let old_divert = swap_handle.swap(Arc::new(new_divert));
					// Shutdown old handle to kick the packet processor into reloading
					if let Err(e) = old_divert.shutdown_handle().shutdown() {
						error!("Failed to shutdown old WinDivert handle: {}", e);
					}
					debug!("Successfully swapped WinDivert handle to new filter");
				}
				Err(e) => {
					error!(
						"Failed to create new WinDivert handle with filter '{}': {}",
						new_filter, e
					);
				}
			}
		}
	});

	// Spawn packet processing task
	let tracker_clone = Arc::clone(&tracker);
	let pcap_file = args.file.clone();
	let net_swap_clone = Arc::clone(&network_divert_swap);
	let net_handle = tokio::spawn(async move {
		process_packets(net_swap_clone, tracker_clone, pcap_file).await;
	});

	// Run WMI event monitoring loop
	if let Err(e) = run_wmi_monitor(default_con, standard_con, tracker).await {
		log::error!("WMI monitor error: {}", e);
	}

	// Cleanup
	if let Err(e) = net_shutdown_handle.shutdown() {
		log::error!("Failed to shutdown terminal network WinDivert: {}", e);
	}
	// Also shutdown the current active handle in the swap
	if let Err(e) = network_divert_swap.load().shutdown_handle().shutdown() {
		log::error!("Failed to shutdown active network WinDivert: {}", e);
	}

	net_handle.abort();
	fastrace::flush();
}
