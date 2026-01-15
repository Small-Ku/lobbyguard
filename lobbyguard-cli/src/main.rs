#![feature(ip)]

use std::path::PathBuf;
use std::sync::Arc;

use argh::FromArgs;
use fastrace::collector::{Config, ConsoleReporter};
use log::debug;
use logforth::append;
use logforth::filter::env_filter::EnvFilterBuilder;
use windivert::prelude::*;

use lobbyguard_cli::connection_tracker::ConnectionTracker;
use lobbyguard_cli::filter::build_network_filter;
use lobbyguard_cli::packet_processor::process_packets;
use lobbyguard_cli::wmi_monitor::{initialize_wmi, run_wmi_monitor};

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
	let tracker = Arc::new(ConnectionTracker::new());

	// Initialize WMI and query existing processes/connections
	let (default_con, standard_con) = initialize_wmi(Arc::clone(&tracker))
		.expect("Failed to initialize WMI connections");

	// Build WinDivert filter
	let net_filter = build_network_filter(args.capture_tcp);
	debug!("Creating network divert with filter: {}", net_filter);
	let network_divert = WinDivert::<NetworkLayer>::network(&net_filter, 0, Default::default())
		.expect("Failed to create network layer WinDivert handle.");

	let net_shutdown_handle = network_divert.shutdown_handle();

	// Spawn packet processing task
	let tracker_clone = Arc::clone(&tracker);
	let pcap_file = args.file.clone();
	let net_handle = tokio::spawn(async move {
		process_packets(network_divert, tracker_clone, pcap_file);
	});

	// Run WMI event monitoring loop
	if let Err(e) = run_wmi_monitor(default_con, standard_con, tracker).await {
		log::error!("WMI monitor error: {}", e);
	}

	// Cleanup
	if let Err(e) = net_shutdown_handle.shutdown() {
		log::error!("Failed to shutdown network WinDivert: {}", e);
	}
	net_handle.abort();
	fastrace::flush();
}
