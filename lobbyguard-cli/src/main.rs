use etherparse::{SlicedPacket, TransportSlice};
use windivert::prelude::*;

#[tokio::main]
async fn main() {
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
