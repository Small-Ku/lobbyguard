#![cfg_attr(
	all(target_os = "windows", not(debug_assertions)),
	windows_subsystem = "windows"
)]

use snafu::prelude::*;
use winio::prelude::*;

use etherparse::{Ipv4Slice, UdpSlice};
use windivert::prelude::*;
const HEARTBEAT_SIZES: [usize; 3] = [12, 18, 63];

#[derive(Debug, Snafu)]
enum Error {
	#[snafu(display("Unable to create application"))]
	NewApp { source: winio::Error },

	#[snafu(display("Unable to show window"))]
	ShowWindow { source: winio::Error },

	#[snafu(context(false), display("WinIO UI error occurred: {cause}"))]
	Ui {
		#[snafu(source(from(winio::Error, Box::new)))]
		cause: Box<winio::Error>,
	},

	#[snafu(context(false), display("Taffy layout error occurred: {cause}"))]
	Layout {
		#[snafu(source(from(LayoutError<winio::Error>, Box::new)))]
		cause: Box<LayoutError<winio::Error>>,
	},
}

type Result<T, E = Error> = std::result::Result<T, E>;
type JoinHandle<T> = compio::runtime::Task<std::result::Result<T, Box<dyn std::any::Any + Send>>>;

#[snafu::report]
fn main() -> Result<()> {
	App::new("de.kwoo.lobbyguard.winiogui")
		.context(NewAppSnafu)?
		.run::<MainModel>(())
}

struct MainModel {
	window: Child<Window>,
	button: Child<Button>,
	is_running: bool,
	divert_handle: Option<windivert::ShutdownHandle>,
	runtime_handle: Option<JoinHandle<()>>,
}

enum MainMessage {
	Noop,
	ToggleButton,
	Redraw,
	Close,
}

impl Component for MainModel {
	type Error = Error;
	type Event = ();
	type Init<'a> = ();
	type Message = MainMessage;

	async fn init(_init: Self::Init<'_>, _sender: &ComponentSender<Self>) -> Result<Self> {
		init! {
			window: Window = (()) => {
				text: "LobbyGuard",
				size: Size::new(300.0, 200.0),
			},
			button: Button = (&window) => {
				text: "Start",
			}
		}
		_sender.post(MainMessage::Redraw);
		window.show().context(ShowWindowSnafu)?;
		Ok(Self {
			window,
			button,
			is_running: false,
			divert_handle: None,
			runtime_handle: None,
		})
	}

	async fn start(&mut self, sender: &ComponentSender<Self>) -> ! {
		start! {
			sender, default: MainMessage::Noop,
			self.window => {
				WindowEvent::Close => MainMessage::Close,
								WindowEvent::Resize | WindowEvent::ThemeChanged => MainMessage::Redraw,
			},
			self.button => {
				ButtonEvent::Click => MainMessage::ToggleButton,
			}
		}
	}

	async fn update(
		&mut self, message: Self::Message, sender: &ComponentSender<Self>,
	) -> Result<bool> {
		match message {
			MainMessage::Noop => Ok(false),
			MainMessage::Redraw => Ok(true),
			MainMessage::ToggleButton => {
				// Toggle the running state
				self.is_running = !self.is_running;

				// Update button text based on state
				if self.is_running {
					self.button.set_text("Stop")?;
					// Start your function here
					println!("Function started!");

					let Ok(divert) = WinDivert::<NetworkLayer>::network(
						"udp.DstPort == 6672 and udp.PayloadLength > 0 and ip",
						0,
						Default::default(),
					) else {
						println!("Failed to create WinDivert");
						self.is_running = false;
						self.button.set_text("Make sure executed with ADMIN right")?;
						return Ok(false);
					};

					self.divert_handle = Some(divert.shutdown_handle());

					self.runtime_handle = Some(compio::runtime::spawn_blocking(move || {
						let mut buffer = [0u8; 1500];

						println!("Start receiving packet");
						loop {
							let result = divert.recv(&mut buffer);
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
					}));
				} else {
					self.button.set_text("Start")?;
					// Stop your function here
					println!("Function stopped!");
					if let Some(shutdown_handle) = self.divert_handle.take() {
						shutdown_handle
							.shutdown()
							.expect("Failed to shutdown WinDivert");
					}
					if let Some(handle) = self.runtime_handle.take() {
						handle.cancel().await;
					}
				}

				Ok(false)
			}
			MainMessage::Close => {
				sender.output(());
				Ok(false)
			}
		}
	}

	async fn update_children(&mut self) -> Result<bool> {
		update_children!(self.window, self.button)
	}

	fn render(&mut self, _sender: &ComponentSender<Self>) -> Result<()> {
		let csize = self.window.client_size()?;

		let mut grid = layout! {
				Grid::from_str("1*", "1*").unwrap(),
				self.button => {
						column: 0,
						row: 0,
						halign: HAlign::Center,
						valign: VAlign::Center
				},
		};
		grid.set_rect(Rect::new(Point::zero(), csize))?;
		Ok(())
	}
}
