use std::fs::File;
use std::path::PathBuf;

use argh::FromArgs;

use etherparse::{SlicedPacket, TransportSlice};
use pcap_file::pcap::{PcapHeader, PcapPacket, PcapWriter};
use pcap_file::{DataLink, Endianness, TsResolution};
use windivert::prelude::*;

#[derive(FromArgs)]
/// Block the GTA connections you don't want.
struct Lobbyguard {
	/// optional path to output captured traffic
	#[argh(option, short = 'f')]
	file: Option<PathBuf>,
}

#[tokio::main]
async fn main() {
	let args: Lobbyguard = argh::from_env();

	const HEARTBEAT_SIZES: [usize; 3] = [12, 18, 63];

	let Ok(divert) = WinDivert::<NetworkLayer>::network(
		"udp.DstPort == 6672 and udp.PayloadLength > 0 and ip",
		0,
		Default::default(),
	) else {
		panic!("Failed to create WinDivert");
	};

	let shutdown_handle = divert.shutdown_handle();

	let handle = tokio::spawn(async move {
		let mut pcap_writer = None;
		if let Some(file) = args.file {
			let file_out = File::create(file).expect("Error creating file out");
			let pcap = PcapWriter::with_header(
				file_out,
				PcapHeader {
					version_major: 2,
					version_minor: 4,
					ts_correction: 0,
					ts_accuracy: 0,
					snaplen: 65535,
					datalink: DataLink::RAW,
					ts_resolution: TsResolution::MicroSecond,
					endianness: Endianness::native(),
				},
			)
			.expect("Error writing file");
			pcap_writer = Some(pcap);
		}

		let mut buffer = [0u8; 1500];

		println!("Start receiving packet");
		loop {
			let result = tokio::task::block_in_place(|| divert.recv_wait(&mut buffer, 0));
			let packet = match result {
				Ok(Some(packet)) => packet,
				Ok(None) => {
					continue;
				}
				Err(WinDivertError::Recv(WinDivertRecvError::NoData)) => {
					break;
				}
				Err(e) => {
					panic!("Error receiving packet: {}", e);
				}
			};

			if let Some(pcap_writer) = pcap_writer.as_mut() {
				let timestamp = std::time::SystemTime::now()
					.duration_since(std::time::SystemTime::UNIX_EPOCH)
					.expect("Time went backwards");
				let pcap_packet =
					PcapPacket::new(timestamp, packet.data.len() as u32, &packet.data);
				pcap_writer.write_packet(&pcap_packet).expect("Error writing packet");
			}

			let Ok(ip) = SlicedPacket::from_ip(&packet.data) else {
				unreachable!("Failed to parse IP headers");
				// continue;
			};
			let Some(TransportSlice::Udp(udp)) = ip.transport else {
				unreachable!("Failed to parse UDP headers");
				// continue;
			};

			let payload = udp.payload();
			let size = payload.len();

			if HEARTBEAT_SIZES.contains(&size) {
				println!("HEARTBEAT PACKET PASSED [L{:?}]", size);
				divert.send(&packet).expect("Failed to send packet");
			}
		}
	});

	println!("Press Ctrl-C to exit.");

	tokio::select! {
	_ = handle => {
		println!("Loop exit.");
	}
	_ = tokio::signal::ctrl_c() => {
		println!("Ctrl-C received! Exiting gracefully.");

		shutdown_handle
			.shutdown()
			.expect("Failed to shutdown WinDivert");
		}
	}
}
