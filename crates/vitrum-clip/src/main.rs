use anyhow::{Context, Result};
use iced::widget::{button, column, container, row, scrollable, text};
use iced::{Alignment, Element, Length, Subscription, Task};
use iced_layershell::actions::LayershellCustomActionWithId;
use iced_layershell::reexport::{Anchor, KeyboardInteractivity, Layer};
use iced_layershell::settings::{LayerShellSettings, Settings};
use std::path::{Path, PathBuf};
use tokio::sync::mpsc;
use tracing::{error, info};

use wayland_client::protocol::{wl_registry, wl_seat};
use wayland_client::{Connection, Dispatch, Proxy, QueueHandle};
use wayland_protocols_wlr::data_control::v1::client::{
	zwlr_data_control_device_v1, zwlr_data_control_manager_v1, zwlr_data_control_offer_v1,
};

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct ClipState {
	history: Vec<ClipItem>,
	visible: bool,
	cache_dir: PathBuf,
	#[serde(skip)]
	receiver: Option<mpsc::UnboundedReceiver<Message>>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
enum ClipItem {
	Text(String),
	Image(PathBuf),
}

#[derive(Debug, Clone)]
enum Message {
	Hide,
	Select(usize),
	IpcEvent(vitrum_ipc::protocol::Opcode, Vec<u8>),
	NewItem(ClipItem),
	Tick,
}

impl TryFrom<Message> for LayershellCustomActionWithId {
	type Error = Message;
	fn try_from(msg: Message) -> std::result::Result<Self, Self::Error> {
		Err(msg)
	}
}

fn boot(receiver: mpsc::UnboundedReceiver<Message>, cache_dir: PathBuf) -> (ClipState, Task<Message>) {
	let history = if let Ok(data) = std::fs::read_to_string(cache_dir.join("history.toml")) {
		toml::from_str(&data).unwrap_or_default()
	} else {
		Vec::new()
	};

	(
		ClipState {
			history,
			visible: false,
			cache_dir,
			receiver: Some(receiver),
		},
		Task::none(),
	)
}

fn update(state: &mut ClipState, message: Message) -> Task<Message> {
	match message {
		Message::Hide => {
			state.visible = false;
		}
		Message::Select(idx) => {
			if let Some(item) = state.history.get(idx) {
				let item = item.clone();
				return Task::perform(async move {
					match item {
						ClipItem::Text(t) => {
							let _ = copy_to_clipboard(t).await;
						}
						ClipItem::Image(p) => {
							let _ = copy_image_to_clipboard(p).await;
						}
					}
					Message::Hide
				}, |m| m);
			}
		}
		Message::IpcEvent(opcode, _payload) => {
			if let vitrum_ipc::protocol::Opcode::EventShowClipboard = opcode {
				info!("Received ShowClipboard event");
				state.visible = true;
			}
		}
		Message::NewItem(item) => {
			if let ClipItem::Text(new_text) = &item {
				if state.history.iter().any(|h| match h {
					ClipItem::Text(t) => t == new_text,
					_ => false,
				}) {
					return Task::none();
				}
			}

			state.history.insert(0, item);
			if state.history.len() > 100 {
				state.history.pop();
			}
			let _ = save_history(&state.cache_dir, &state.history);
		}
		Message::Tick => {
			let mut tasks = Vec::new();
			if let Some(mut rx) = state.receiver.take() {
				while let Ok(msg) = rx.try_recv() {
					tasks.push(update(state, msg));
				}
				state.receiver = Some(rx);
			}
			if !tasks.is_empty() {
				return Task::batch(tasks);
			}
		}
	}
	Task::none()
}

fn save_history(cache_dir: &Path, history: &[ClipItem]) -> Result<()> {
	let data = toml::to_string(history)?;
	std::fs::write(cache_dir.join("history.toml"), data)?;
	Ok(())
}

fn view(state: &ClipState) -> Element<'_, Message> {
	if !state.visible {
		return container(iced::widget::Space::new()).into();
	}

	let content = column![
		row![
			text("Clipboard History").size(24).width(Length::Fill),
			button(text("Close")).on_press(Message::Hide),
		]
		.align_y(Alignment::Center),
		scrollable(
			column(
				state
					.history
					.iter()
					.enumerate()
					.map(|(i, item)| {
						let label = match item {
							ClipItem::Text(t) => {
								let mut s = t.replace('\n', " ").trim().to_string();
								if s.len() > 80 {
									s.truncate(77);
									s.push_str("...");
								}
								if s.is_empty() {
									"Empty content".into()
								} else {
									s
								}
							}
							ClipItem::Image(p) => format!("Image: {}", p.file_name().unwrap_or_default().to_string_lossy()),
						};
						button(text(label))
							.width(Length::Fill)
							.padding(12)
							.on_press(Message::Select(i))
							.style(button::secondary)
							.into()
					})
					.collect::<Vec<_>>()
			)
			.spacing(8)
		)
		.height(Length::Fill)
	]
	.spacing(15)
	.padding(25);

	container(content)
		.width(500)
		.height(700)
		.style(|_| container::Style {
			background: Some(iced::Background::Color(iced::Color::from_rgb8(24, 24, 37))),
			text_color: Some(iced::Color::WHITE),
			border: iced::Border {
				color: iced::Color::from_rgb8(137, 180, 250),
				width: 2.0,
				radius: 12.0.into(),
			},
			shadow: iced::Shadow {
				color: iced::Color::from_rgba(0.0, 0.0, 0.0, 0.5),
				offset: iced::Vector::new(0.0, 10.0),
				blur_radius: 20.0,
			},
			..Default::default()
		})
		.into()
}

fn subscription(_state: &ClipState) -> Subscription<Message> {
	iced::time::every(std::time::Duration::from_millis(50)).map(|_| Message::Tick)
}

async fn copy_to_clipboard(text: String) -> Result<()> {
	info!("Pasting to clipboard: {}...", &text[..text.len().min(20)]);
	use wl_clipboard_rs::copy::{MimeType, Options, Source};
	let opts = Options::default();
	opts.copy(Source::Bytes(text.into_bytes().into()), MimeType::Text)
		.context("Failed to copy to clipboard")?;
	Ok(())
}

async fn copy_image_to_clipboard(path: PathBuf) -> Result<()> {
	info!("Pasting image to clipboard: {:?}", path);
	use wl_clipboard_rs::copy::{MimeType, Options, Source};
	let data = std::fs::read(path).context("Failed to read image")?;
	let opts = Options::default();
	opts.copy(Source::Bytes(data.into()), MimeType::Specific("image/png".to_string()))
		.context("Failed to copy image to clipboard")?;
	Ok(())
}

struct MonitorState {
	tx: mpsc::UnboundedSender<Message>,
	manager: Option<zwlr_data_control_manager_v1::ZwlrDataControlManagerV1>,
	seat: Option<wl_seat::WlSeat>,
	offers: std::collections::HashMap<wayland_client::backend::ObjectId, Vec<String>>,
}

impl Dispatch<wl_registry::WlRegistry, ()> for MonitorState {
	fn event(state: &mut Self, proxy: &wl_registry::WlRegistry, event: wl_registry::Event, _: &(), _: &Connection, qh: &QueueHandle<Self>) {
		if let wl_registry::Event::Global {
			name,
			interface,
			version,
		} = event
		{
			match interface.as_str() {
				"zwlr_data_control_manager_v1" => {
					state.manager = Some(proxy.bind(name, version, qh, ()));
				}
				"wl_seat" => {
					state.seat = Some(proxy.bind(name, 1, qh, ()));
				}
				_ => {}
			}
		}
	}
}

impl Dispatch<zwlr_data_control_manager_v1::ZwlrDataControlManagerV1, ()> for MonitorState {
	fn event(_: &mut Self, _: &zwlr_data_control_manager_v1::ZwlrDataControlManagerV1, _: zwlr_data_control_manager_v1::Event, _: &(), _: &Connection, _: &QueueHandle<Self>) {
	}
}

impl Dispatch<wl_seat::WlSeat, ()> for MonitorState {
	fn event(_: &mut Self, _: &wl_seat::WlSeat, _: wl_seat::Event, _: &(), _: &Connection, _: &QueueHandle<Self>) {}
}

impl Dispatch<zwlr_data_control_device_v1::ZwlrDataControlDeviceV1, ()> for MonitorState {
	fn event(state: &mut Self, _: &zwlr_data_control_device_v1::ZwlrDataControlDeviceV1, event: zwlr_data_control_device_v1::Event, _: &(), _: &Connection, _: &QueueHandle<Self>) {
		match event {
			zwlr_data_control_device_v1::Event::DataOffer { id } => {
				state.offers.insert(id.id(), Vec::new());
			}
			zwlr_data_control_device_v1::Event::Selection { id } => {
				if let Some(offer) = id {
					let mimes = state.offers.get(&offer.id()).cloned().unwrap_or_default();
					if mimes.iter().any(|m| m == "text/plain" || m == "text/plain;charset=utf-8") {
						if let Ok(text) = capture_text(&offer) {
							let _ = state.tx.send(Message::NewItem(ClipItem::Text(text)));
						}
					}
				}
			}
			_ => {}
		}
	}
}

fn capture_text(offer: &zwlr_data_control_offer_v1::ZwlrDataControlOfferV1) -> Result<String> {
	use std::os::unix::io::AsRawFd;
	let (mut read_pipe, write_pipe) = os_pipe::pipe()?;
	offer.receive("text/plain".into(), unsafe { std::os::unix::io::BorrowedFd::borrow_raw(write_pipe.as_raw_fd()) });
	drop(write_pipe);

	let mut content = String::new();
	use std::io::Read;
	read_pipe.read_to_string(&mut content)?;
	Ok(content)
}

impl Dispatch<zwlr_data_control_offer_v1::ZwlrDataControlOfferV1, ()> for MonitorState {
	fn event(state: &mut Self, offer: &zwlr_data_control_offer_v1::ZwlrDataControlOfferV1, event: zwlr_data_control_offer_v1::Event, _: &(), _: &Connection, _: &QueueHandle<Self>) {
		match event {
			zwlr_data_control_offer_v1::Event::Offer { mime_type } => {
				state.offers.entry(offer.id()).or_default().push(mime_type);
			}
			_ => {}
		}
	}
}

fn run_monitor(tx: mpsc::UnboundedSender<Message>) -> Result<()> {
	let conn = Connection::connect_to_env().context("Failed to connect to Wayland")?;
	let display = conn.display();
	let mut event_queue = conn.new_event_queue();
	let qh = event_queue.handle();

	let mut state = MonitorState {
		tx,
		manager: None,
		seat: None,
		offers: std::collections::HashMap::new(),
	};

	let _registry = display.get_registry(&qh, ());
	event_queue.roundtrip(&mut state)?;
	event_queue.roundtrip(&mut state)?;

	if let (Some(manager), Some(seat)) = (&state.manager, &state.seat) {
		let _device = manager.get_data_device(seat, &qh, ());
	} else {
		return Err(anyhow::anyhow!("Data control or seat not available"));
	}

	loop {
		event_queue.blocking_dispatch(&mut state)?;
	}
}

#[tokio::main]
async fn main() -> Result<()> {
	tracing_subscriber::fmt::init();
	info!("Starting vitrum-clip daemon");

	let cache_dir = dirs::cache_dir()
		.unwrap_or_else(|| PathBuf::from("/tmp"))
		.join("vitrum")
		.join("clip");
	std::fs::create_dir_all(&cache_dir)?;

	let (app_tx, app_rx) = mpsc::unbounded_channel();

	let ipc_tx = app_tx.clone();
	tokio::spawn(async move {
		let socket = vitrum_ipc::event_socket_path();
		if let Ok(mut subscriber) = vitrum_ipc::IpcEventSubscriber::connect(&socket).await {
			let _ = subscriber
				.send_request(
					vitrum_ipc::Opcode::Subscribe,
					&vitrum_ipc::protocol::IpcSubscribe {
						mask: vitrum_ipc::protocol::IpcEventMask::CLIPBOARD,
					},
				)
				.await;

			while let Ok((opcode, payload)) = subscriber.next_event().await {
				let _ = ipc_tx.send(Message::IpcEvent(opcode, payload));
			}
		}
	});

	let monitor_tx = app_tx.clone();
	std::thread::spawn(move || {
		if let Err(e) = run_monitor(monitor_tx) {
			error!("Monitor error: {}", e);
		}
	});

	let settings = Settings {
		layer_settings: LayerShellSettings {
			anchor: Anchor::Top | Anchor::Right,
			layer: Layer::Overlay,
			keyboard_interactivity: KeyboardInteractivity::OnDemand,
			margin: (20, 20, 20, 20),
			..Default::default()
		},
		..Default::default()
	};

	let cache_dir_clone = cache_dir.clone();
	let rx_mutex = std::sync::Mutex::new(Some(app_rx));

	iced_layershell::application(
		move || boot(rx_mutex.lock().unwrap().take().expect("boot called twice"), cache_dir_clone.clone()),
		"vitrum-clip",
		update,
		view,
	)
	.settings(settings)
	.subscription(subscription)
	.run()?;

	Ok(())
}
