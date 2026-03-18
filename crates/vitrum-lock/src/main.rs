use chrono::Local;
use iced::widget::{column, container, image, text, text_input};
use iced::{Background, Color, Element, Length, Task, Theme};
use iced_sessionlock::settings::Settings;
use iced_sessionlock::to_session_message;

#[to_session_message]
#[derive(Debug, Clone)]
enum Message {
	Tick,

	PasswordChanged(String),

	PasswordSubmit,

	AuthFailed,

	ShakeTick,

	_ShowPrompt,

	WindowOpened(iced::window::Id),
}

struct LockState {
	password: String,
	show_prompt: bool,
	auth_failed: bool,
	shaking: bool,
	shake_x: i32,
	clock: String,
	date: String,
	config: vitrum_config::Config,
	primary_window: Option<iced::window::Id>,
}

fn boot() -> LockState {
	let (clock, date) = format_clock_date();
	let config = vitrum_config::Config::load().unwrap_or_default();
	LockState {
		password: String::new(),
		show_prompt: false,
		auth_failed: false,
		shaking: false,
		shake_x: 0,
		clock,
		date,
		config,
		primary_window: None,
	}
}

fn update(state: &mut LockState, message: Message) -> Task<Message> {
	match message {
		Message::Tick => {
			let (clock, date) = format_clock_date();
			state.clock = clock;
			state.date = date;
			Task::none()
		}
		Message::PasswordChanged(val) => {
			state.password = val;
			state.auth_failed = false;
			Task::none()
		}
		Message::PasswordSubmit => {
			if state.password.is_empty() {
				return Task::none();
			}
			if pam_authenticate(&state.password) {
				Task::done(Message::UnLock)
			} else {
				Task::done(Message::AuthFailed)
			}
		}
		Message::AuthFailed => {
			state.password.clear();
			state.auth_failed = true;
			state.shaking = true;
			state.shake_x = 12;
			Task::done(Message::ShakeTick)
		}
		Message::ShakeTick => {
			if state.shaking {
				state.shake_x = -state.shake_x / 2;
				if state.shake_x == 0 {
					state.shaking = false;
				} else {
					return Task::done(Message::ShakeTick);
				}
			}
			Task::none()
		}
		Message::_ShowPrompt => {
			state.show_prompt = true;
			Task::none()
		}
		Message::UnLock => Task::none(),
		Message::WindowOpened(id) => {
			if state.primary_window.is_none() {
				state.primary_window = Some(id);
			}
			Task::none()
		}
	}
}

fn view_primary(state: &LockState, window_id: iced::window::Id) -> Element<'_, Message> {
	if let Some(primary) = state.primary_window {
		if primary != window_id {
			return view_secondary(state, window_id);
		}
	}

	let theme = &state.config.theme;
	let accent = hex_to_color(&theme.accent);
	let text_color = hex_to_color(&theme.text);
	let bg_color = hex_to_color(&theme.background);

	let clock_w = text(&state.clock).size(80).color(text_color);

	let date_w = text(&state.date).size(18).color(hex_to_color(&theme.text_muted));

	let prompt: Element<Message> = if state.show_prompt || !state.password.is_empty() {
		let input = text_input("Password", &state.password)
			.on_input(Message::PasswordChanged)
			.on_submit(Message::PasswordSubmit)
			.secure(true)
			.size(16)
			.width(Length::Fixed(280.0));

		let border_color = if state.auth_failed {
			hex_to_color(&theme.error)
		} else {
			accent
		};

		column![
			if state.auth_failed {
				text("Wrong password").size(13).color(hex_to_color(&theme.error))
			} else {
				text("").size(13)
			},
			container(input).padding(8).style(move |_: &Theme| container::Style {
				background: Some(Background::Color(Color::from_rgba(0.0, 0.0, 0.0, 0.5))),
				border: iced::Border {
					color: border_color,
					width: theme.border_width as f32,
					radius: (theme.corner_radius as f32).into(),
				},
				..Default::default()
			}),
		]
		.spacing(6)
		.align_x(iced::Alignment::Center)
		.into()
	} else {
		text("Press any key to unlock")
			.size(15)
			.color(hex_to_color(&theme.text_muted))
			.into()
	};

	let content = column![
		clock_w,
		date_w,
		container(prompt).padding(iced::Padding {
			top: 40.0,
			..Default::default()
		}),
	]
	.align_x(iced::Alignment::Center)
	.spacing(0);

	let pad_left = if state.shaking {
		state.shake_x.unsigned_abs() as f32
	} else {
		0.0
	};

	let mut background = container(container(content).center_x(Length::Fill).center_y(Length::Fill).padding(
		iced::Padding {
			left: pad_left,
			..Default::default()
		},
	))
	.width(Length::Fill)
	.height(Length::Fill);

	if state.config.wallpaper.mode == "image" {
		if let Some(ref path_str) = state.config.wallpaper.path {
			let path = path_str.replace("~", &std::env::var("HOME").unwrap_or_default());
			let path_buf = std::path::PathBuf::from(path);

			background = background.style(move |_: &Theme| container::Style {
				background: Some(Background::Color(Color { a: 0.85, ..bg_color })),
				..Default::default()
			});

			return iced::widget::stack![
				image(path_buf)
					.width(Length::Fill)
					.height(Length::Fill)
					.content_fit(iced::ContentFit::Cover),
				container(text(""))
					.width(Length::Fill)
					.height(Length::Fill)
					.style(|_: &Theme| container::Style {
						background: Some(Background::Color(Color::from_rgba(0.0, 0.0, 0.0, 0.75))),
						..Default::default()
					}),
				background
			]
			.into();
		}
	}

	background
		.style(move |_: &Theme| container::Style {
			background: Some(Background::Color(Color { a: 0.95, ..bg_color })),
			..Default::default()
		})
		.into()
}

fn view_secondary(state: &LockState, _window_id: iced::window::Id) -> Element<'_, Message> {
	let theme = &state.config.theme;
	let text_color = hex_to_color(&theme.text);
	let bg_color = hex_to_color(&theme.background);

	let clock_w = text(&state.clock).size(60).color(text_color);

	let content = container(clock_w).center_x(Length::Fill).center_y(Length::Fill);

	container(content)
		.width(Length::Fill)
		.height(Length::Fill)
		.style(move |_: &Theme| container::Style {
			background: Some(Background::Color(Color::from_rgba(bg_color.r, bg_color.g, bg_color.b, 0.95))),
			..Default::default()
		})
		.into()
}

fn format_clock_date() -> (String, String) {
	let now = Local::now();
	(now.format("%H:%M:%S").to_string(), now.format("%A, %-d %B %Y").to_string())
}

fn pam_authenticate(password: &str) -> bool {
	if password.is_empty() {
		return false;
	}

	let username = std::env::var("USER").unwrap_or_else(|_| "root".to_string());

	match pam::Client::with_password("login") {
		Ok(mut client) => {
			client.conversation_mut().set_credentials(&username, password);
			match client.authenticate() {
				Ok(()) => {
					let _ = client.open_session();
					true
				}
				Err(e) => {
					tracing::debug!("PAM auth failed: {:?}", e);
					false
				}
			}
		}
		Err(e) => {
			tracing::error!("Failed to create PAM client: {:?}. Falling back to su.", e);

			fallback_su_auth(password, &username)
		}
	}
}

fn fallback_su_auth(password: &str, username: &str) -> bool {
	match std::process::Command::new("su")
		.arg("-c")
		.arg("true")
		.arg(username)
		.stdin(std::process::Stdio::piped())
		.stdout(std::process::Stdio::null())
		.stderr(std::process::Stdio::null())
		.spawn()
	{
		Ok(mut child) => {
			if let Some(mut stdin) = child.stdin.take() {
				use std::io::Write;
				let _ = writeln!(stdin, "{}", password);
				let _ = writeln!(stdin, "");
			}
			child.wait().map(|s| s.success()).unwrap_or(false)
		}
		Err(_) => false,
	}
}

fn hex_to_color(hex: &str) -> Color {
	let hex = hex.trim_start_matches('#');
	if hex.len() == 6 {
		let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(0);
		let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(0);
		let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(0);
		Color::from_rgb8(r, g, b)
	} else {
		Color::WHITE
	}
}

fn main() -> iced_sessionlock::Result {
	tracing_subscriber::fmt()
		.with_env_filter(tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
		.init();

	let settings = Settings {
		id: Some("vitrum-lock".to_string()),
		..Default::default()
	};

	iced_sessionlock::application(boot, update, view_primary)
		.settings(settings)
		.subscription(|_state| {
			use iced::time;
			let tick = time::every(std::time::Duration::from_secs(1)).map(|_| Message::Tick);
			let window_events = iced::event::listen_with(|event, _status, id| match event {
				iced::Event::Window(iced::window::Event::Opened { .. }) => Some(Message::WindowOpened(id)),
				_ => None,
			});
			iced::Subscription::batch(vec![tick, window_events])
		})
		.run()
}
