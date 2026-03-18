use clap::Parser;
use iced::widget::{column, container, progress_bar, svg, text};
use iced::{Alignment, Color, Element, Length, Task, Theme};
use iced_layershell::reexport::{Anchor, Layer};
use iced_layershell::settings::{LayerShellSettings, Settings};
use vitrum_ipc::OsdIcon;

#[derive(Parser, Debug, Clone)]
#[command(author, version, about, long_about = None)]
struct Args {
	#[arg(long)]
	icon: String,
	#[arg(long)]
	value: Option<u8>,
	#[arg(long)]
	text: Option<String>,
}

use iced_layershell::actions::LayershellCustomActionWithId;

pub fn main() -> Result<(), iced_layershell::Error> {
	let args = Args::parse();

	let settings = Settings {
		layer_settings: LayerShellSettings {
			layer: Layer::Overlay,
			anchor: Anchor::Bottom | Anchor::Top | Anchor::Left | Anchor::Right,
			exclusive_zone: -1,
			..Default::default()
		},
		..Default::default()
	};

	iced_layershell::application(move || boot(args.clone()), "vitrum-osd", Osd::update, Osd::view)
		.settings(settings)
		.subscription(Osd::subscription)
		.run()
}

struct Osd {
	icon: OsdIcon,
	value: Option<u8>,
	text: Option<String>,
	start_time: std::time::Instant,
}

#[derive(Debug, Clone, Copy)]
enum Message {
	Tick,
}

impl TryFrom<Message> for LayershellCustomActionWithId {
	type Error = Message;
	fn try_from(msg: Message) -> Result<Self, Self::Error> {
		Err(msg)
	}
}

fn boot(args: Args) -> (Osd, Task<Message>) {
	let icon = match args.icon.to_lowercase().as_str() {
		"volume" => OsdIcon::Volume,
		"brightness" => OsdIcon::Brightness,
		"mute" => OsdIcon::Mute,
		"micmute" => OsdIcon::MicMute,
		"capslock" => OsdIcon::CapsLock,
		_ => OsdIcon::Custom,
	};

	(
		Osd {
			icon,
			value: args.value,
			text: args.text,
			start_time: std::time::Instant::now(),
		},
		Task::none(),
	)
}

impl Osd {
	fn subscription(&self) -> iced::Subscription<Message> {
		iced::time::every(std::time::Duration::from_millis(100)).map(|_| Message::Tick)
	}

	fn update(&mut self, message: Message) -> Task<Message> {
		match message {
			Message::Tick => {
				if self.start_time.elapsed().as_secs_f32() > 2.0 {
					std::process::exit(0);
				}
				Task::none()
			}
		}
	}

	fn view(&self) -> Element<'_, Message> {
		let icon_bytes: &[u8] = match self.icon {
			OsdIcon::Volume => include_bytes!("../../vitrum-bar/resources/cpu.svg"),
			OsdIcon::Brightness => include_bytes!("../../vitrum-bar/resources/memory.svg"),
			_ => include_bytes!("../../vitrum-bar/resources/cpu.svg"),
		};

		let icon_handle = svg::Handle::from_memory(icon_bytes);

		let mut content = column![svg(icon_handle).width(64).height(64)]
			.spacing(20)
			.align_x(Alignment::Center);

		if let Some(v) = self.value {
			content = content.push(
				container(
					progress_bar(0.0..=100.0, v as f32).style(|_theme: &Theme| progress_bar::Style {
						background: Color::from_rgba(1.0, 1.0, 1.0, 0.1).into(),
						bar: Color::from_rgb(1.0, 1.0, 1.0).into(),
						border: iced::Border {
							radius: 4.0.into(),
							..Default::default()
						},
					}),
				)
				.width(Length::Fixed(200.0))
				.height(Length::Fixed(8.0)),
			);
		}

		if let Some(ref t) = self.text {
			content = content.push(text(t).size(16).color(Color::WHITE));
		}

		container(container(content).padding(30).style(|_theme: &Theme| container::Style {
			background: Some(Color::from_rgba(0.0, 0.0, 0.0, 0.85).into()),
			border: iced::Border {
				radius: 12.0.into(),
				color: Color::from_rgba(1.0, 1.0, 1.0, 0.15),
				width: 1.0,
			},
			..Default::default()
		}))
		.width(Length::Fill)
		.height(Length::Fill)
		.align_x(Alignment::Center)
		.align_y(Alignment::Center)
		.into()
	}
}
