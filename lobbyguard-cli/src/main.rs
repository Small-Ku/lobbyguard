use etherparse::{Ipv4Slice, UdpSlice};
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
			let result = tokio::task::block_in_place(|| divert.recv(&mut buffer));
			match result {
				Ok(packet) => {
					let Ok(ip) = Ipv4Slice::from_slice(&packet.data) else {
						eprintln!("Failed to parse IP headers");
						continue;
					};
					let Ok(udp) = UdpSlice::from_slice(&ip.payload().payload) else {
						eprintln!("Failed to parse UDP headers");
						continue;
					};

					let payload = udp.payload();
					let size = payload.len();

					if HEARTBEAT_SIZES.iter().any(|&x| x == size) {
						println!("HEARTBEAT PACKET PASSED [{:?}]", size);
						divert.send(&packet).expect("Failed to send packet");
					}
				}
				Err(WinDivertError::Recv(WinDivertRecvError::NoData)) => {
					break;
				}
				Err(e) => {
					eprintln!("Error receiving packet: {}", e);
				}
			}
		}
	});

	println!("Press Ctrl-C to exit.");

	tokio::signal::ctrl_c().await.unwrap();

	println!("Ctrl-C received! Exiting gracefully.");

	shutdown_handle
		.shutdown()
		.expect("Failed to shutdown WinDivert");

	handle.await.unwrap();
}
