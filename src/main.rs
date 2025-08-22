use etherparse::{Ipv4Slice, UdpSlice};
use windivert::prelude::*;

fn main() {
	const HEARTBEAT_SIZES: [usize; 3] = [12, 18, 63];

	let Ok(divert) = WinDivert::<NetworkLayer>::network(
		"udp.DstPort == 6672 and udp.PayloadLength > 0 and ip",
		0,
		Default::default(),
	) else {
		panic!("Failed to create WinDivert");
	};

	let handle = std::thread::spawn(move || {
		let mut buffer = [0u8; 1500];

		println!("Start receiving packet");
		loop {
			match divert.recv(&mut buffer) {
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

	handle.join().unwrap();
	println!("shutting down...");
}
