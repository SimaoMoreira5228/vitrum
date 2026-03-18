use chrono::Local;
use iced::widget::{container, row, svg, text};
use iced::{Alignment, Background, Color, Element, Length, Subscription, Task, Theme};
use iced_layershell::actions::LayershellCustomActionWithId;
use iced_layershell::reexport::{Anchor, KeyboardInteractivity, Layer};
use iced_layershell::settings::{LayerShellSettings, Settings};
use serde::{Deserialize, Serialize};
use sysinfo::System;
use vitrum_config::script::eval::BuiltinHandler;
use vitrum_config::script::eval::EvalError;
use vitrum_config::script::{Arena, Evaluator, Scope, Span, Value, ValueDeserializer};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct BarConfig {
	#[serde(default)]
	left: Vec<BarWidget>,
	#[serde(default)]
	center: Vec<BarWidget>,
	#[serde(default)]
	right: Vec<BarWidget>,
	#[serde(default = "default_height")]
	height: u32,
	#[serde(default)]
	position: BarPosition,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
enum BarPosition {
	#[default]
	Top,
	Bottom,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "widget")]
enum BarWidget {
	#[serde(rename = "text")]
	Text {
		content: String,
		#[serde(default)]
		color: Option<String>,
		#[serde(default)]
		size: Option<u32>,
		#[serde(default)]
		font: Option<String>,
	},
	#[serde(rename = "svg")]
	Svg {
		path: String,
		#[serde(default)]
		color: Option<String>,
		#[serde(default)]
		size: Option<u32>,
	},
	#[serde(rename = "workspaces")]
	Workspaces {
		#[serde(default)]
		show_empty: bool,
	},
	#[serde(rename = "cpu")]
	Cpu {
		#[serde(default)]
		show_label: bool,
	},
	#[serde(rename = "memory")]
	Memory {
		#[serde(default)]
		show_label: bool,
	},
	#[serde(rename = "battery")]
	Battery,
	#[serde(rename = "clock")]
	Clock {
		#[serde(default = "default_clock_format")]
		format: String,
	},
	#[serde(rename = "active_window")]
	ActiveWindow {
		#[serde(default)]
		max_length: Option<u32>,
	},
	#[serde(rename = "spacer")]
	Spacer {
		#[serde(default)]
		width: Option<u32>,
	},
	#[serde(rename = "row")]
	Row {
		children: Vec<BarWidget>,
		#[serde(default)]
		spacing: Option<u32>,
	},
}

impl BarConfig {
	fn load() -> Self {
		let _path = dirs::config_dir().map(|d| d.join("vitrum").join("bar.vt"));

		Self::default()
	}
}

fn default_height() -> u32 {
	32
}
fn default_clock_format() -> String {
	"%H:%M".into()
}

struct BarBuiltins;

impl BuiltinHandler for BarBuiltins {
	fn call(&self, name: &str, args: &[Value], span: &Span) -> Result<Value, EvalError> {
		let mut map = std::collections::HashMap::new();
		map.insert("type".to_string(), Value::String("bar_widget".to_string()));
		map.insert("widget".to_string(), Value::String(name.to_string()));

		match name {
			"text" | "svg" | "spacer" | "workspaces" | "cpu" | "memory" | "battery" | "clock" | "active_window" => {
				if let Some(arg) = args.first() {
					if let Value::Map(m) = arg {
						for (k, v) in m {
							map.insert(k.clone(), v.clone());
						}
					} else if name == "text" {
						map.insert("content".to_string(), arg.clone());
					} else if name == "svg" {
						map.insert("path".to_string(), arg.clone());
					}
				}
				Ok(Value::Map(map))
			}
			"row" | "column" => {
				if let Some(arg) = args.first() {
					map.insert("children".to_string(), arg.clone());
				}
				Ok(Value::Map(map))
			}
			_ => Err(EvalError {
				line: span.line,
				col: span.col,
				msg: format!("unknown bar widget: {}", name),
			}),
		}
	}
}

const CPU_SVG: &[u8] = include_bytes!("../resources/cpu.svg");
const MEM_SVG: &[u8] = include_bytes!("../resources/memory.svg");
const BAT_EMPTY_SVG: &[u8] = include_bytes!("../resources/batery-empty.svg");
const BAT_1_SVG: &[u8] = include_bytes!("../resources/batery-1.svg");
const BAT_2_SVG: &[u8] = include_bytes!("../resources/batery-2.svg");
const BAT_3_SVG: &[u8] = include_bytes!("../resources/batery-3.svg");
const BAT_4_SVG: &[u8] = include_bytes!("../resources/batery-4.svg");

fn color_or(s: &str, default: Color) -> Color {
	if s.is_empty() {
		return default;
	}
	if s.starts_with('#') && s.len() == 7 {
		if let (Ok(r), Ok(g), Ok(b)) = (
			u8::from_str_radix(&s[1..3], 16),
			u8::from_str_radix(&s[3..5], 16),
			u8::from_str_radix(&s[5..7], 16),
		) {
			return Color::from_rgb8(r, g, b);
		}
	}
	default
}

#[derive(Debug, Clone)]
enum LocalIpcEvent {
	WorkspaceChanged(vitrum_ipc::WorkspaceChanged),
	WindowOpened(vitrum_ipc::WindowOpened),
	WindowMoved(vitrum_ipc::WindowMoved),
}

#[derive(Debug, Clone)]
enum Message {
	IpcEvent(LocalIpcEvent),
	WorkspaceClick(u32),
	Tick,
	SystemUpdate,
	Loaded(InitData),
}

impl TryFrom<Message> for LayershellCustomActionWithId {
	type Error = Message;
	fn try_from(msg: Message) -> Result<Self, Self::Error> {
		Err(msg)
	}
}

#[derive(Debug, Clone)]
struct InitData {
	workspaces: Vec<vitrum_ipc::WorkspaceInfo>,
	active_window: Option<vitrum_ipc::WindowInfo>,
	theme: vitrum_ipc::ThemeSnapshot,
}

struct BarState {
	workspaces: Vec<WsState>,
	active_workspace: u32,
	focused_title: String,
	clock: String,
	cpu_usage: f32,
	mem_usage: f32,
	battery_percent: Option<u8>,
	config: BarConfig,
	sys: System,
	theme: vitrum_ipc::ThemeSnapshot,
}

#[derive(Debug, Clone)]
struct WsState {
	id: u32,
	window_count: usize,
	active: bool,
}

fn boot() -> (BarState, Task<Message>) {
	let config = BarConfig::load();
	let mut sys = System::new_all();
	sys.refresh_all();

	let state = BarState {
		workspaces: (1..=10)
			.map(|id| WsState {
				id,
				window_count: 0,
				active: id == 1,
			})
			.collect(),
		active_workspace: 1,
		focused_title: String::new(),
		clock: format_time(),
		cpu_usage: 0.0,
		mem_usage: 0.0,
		battery_percent: None,
		config,
		sys,
		theme: vitrum_ipc::ThemeSnapshot {
			accent: String::new(),
			background: String::new(),
			surface: String::new(),
			surface_raised: String::new(),
			text: String::new(),
			text_muted: String::new(),
			border: String::new(),
			error: String::new(),
			warning: String::new(),
			success: String::new(),
			border_width: 1,
			gaps_inner: 4,
			gaps_outer: 4,
			corner_radius: 0,
			cursor_theme: String::new(),
			cursor_size: 24,
			icon_theme: String::new(),
			color_scheme: "dark".into(),
			font_ui: "sans-serif".into(),
			font_ui_size: 11,
			font_mono: "monospace".into(),
			font_mono_size: 11,
			dpi: 96,
			gdk_scale: 1,
		},
	};

	let init = Task::perform(
		async {
			let socket = vitrum_ipc::command_socket_path();
			let mut c = vitrum_ipc::IpcClient::connect(&socket).await?;

			let (_, payload) = c.send_request(vitrum_ipc::Opcode::GetWorkspaces, &()).await?;
			let ws: vitrum_ipc::WorkspacesResponse = rmp_serde::from_slice(&payload)?;

			let (_, payload) = c.send_request(vitrum_ipc::Opcode::GetActiveWindow, &()).await?;
			let aw: vitrum_ipc::ActiveWindowResponse = rmp_serde::from_slice(&payload)?;

			let (_, payload) = c.send_request(vitrum_ipc::Opcode::GetTheme, &()).await?;
			let th: vitrum_ipc::ThemeResponse = rmp_serde::from_slice(&payload)?;

			Ok::<_, anyhow::Error>(InitData {
				workspaces: ws.workspaces,
				active_window: aw.window,
				theme: th.theme,
			})
		},
		|result| match result {
			Ok(d) => Message::Loaded(d),
			Err(_) => Message::Tick,
		},
	);

	(state, init)
}

fn update(s: &mut BarState, msg: Message) -> Task<Message> {
	match msg {
		Message::Loaded(data) => {
			s.theme = data.theme;
			for ws in &mut s.workspaces {
				if let Some(info) = data.workspaces.iter().find(|w| w.id == ws.id) {
					ws.window_count = info.window_count;
					ws.active = info.active;
				}
			}
			if let Some(active) = data.workspaces.iter().find(|w| w.active) {
				s.active_workspace = active.id;
			}
			if let Some(w) = data.active_window {
				s.focused_title = if !w.title.is_empty() { w.title } else { w.app_id };
			}

			evaluate_bar_vt(s);

			Task::none()
		}
		Message::IpcEvent(event) => {
			match event {
				LocalIpcEvent::WorkspaceChanged(ev) => {
					s.active_workspace = ev.workspace;
					for ws in &mut s.workspaces {
						ws.active = ws.id == ev.workspace;
					}
				}
				LocalIpcEvent::WindowOpened(ev) => {
					if let Some(ws) = s.workspaces.iter_mut().find(|w| w.id == ev.window.workspace) {
						ws.window_count += 1;
					}
					if ev.window.workspace == s.active_workspace {
						s.focused_title = if !ev.window.title.is_empty() {
							ev.window.title
						} else {
							ev.window.app_id
						};
					}
				}
				LocalIpcEvent::WindowMoved(ev) => {
					if let Some(ws) = s.workspaces.iter_mut().find(|w| w.id == ev.from_workspace) {
						ws.window_count = ws.window_count.saturating_sub(1);
					}
					if let Some(ws) = s.workspaces.iter_mut().find(|w| w.id == ev.to_workspace) {
						ws.window_count += 1;
					}
				}
			}
			Task::none()
		}
		Message::WorkspaceClick(ws_id) => Task::perform(
			async move {
				let socket = vitrum_ipc::command_socket_path();
				let mut c = vitrum_ipc::IpcClient::connect(&socket).await?;
				c.send_request(
					vitrum_ipc::Opcode::SwitchWorkspace,
					&vitrum_ipc::SwitchWorkspace { id: ws_id },
				)
				.await?;
				Ok::<_, anyhow::Error>(())
			},
			|_| Message::Tick,
		),
		Message::SystemUpdate => {
			s.sys.refresh_cpu_all();
			s.sys.refresh_memory();

			s.cpu_usage = s.sys.global_cpu_usage();
			s.mem_usage = (s.sys.used_memory() as f32 / s.sys.total_memory() as f32) * 100.0;

			if let Ok(cap) = std::fs::read_to_string("/sys/class/power_supply/BAT0/capacity") {
				s.battery_percent = cap.trim().parse().ok();
			} else if let Ok(cap) = std::fs::read_to_string("/sys/class/power_supply/BAT1/capacity") {
				s.battery_percent = cap.trim().parse().ok();
			}

			Task::none()
		}
		Message::Tick => {
			s.clock = format_time();
			Task::none()
		}
	}
}

fn render_widget(w: &BarWidget, s: &BarState) -> Element<'static, Message> {
	match w {
		BarWidget::Text {
			content, color, size, ..
		} => {
			let color = color.as_ref().map(|c| color_or(c, Color::WHITE)).unwrap_or(Color::WHITE);
			text(content.clone()).size(size.unwrap_or(13) as f32).color(color).into()
		}
		BarWidget::Svg { path, color: _, size } => {
			let handle = match path.as_str() {
				"cpu" => svg::Handle::from_memory(CPU_SVG),
				"memory" => svg::Handle::from_memory(MEM_SVG),
				"battery" => svg::Handle::from_memory(BAT_4_SVG),
				_ => {
					let icon_path = dirs::config_dir()
						.map(|d| d.join("vitrum").join("icons").join(path))
						.filter(|p| p.exists());
					if let Some(p) = icon_path {
						svg::Handle::from_path(p)
					} else {
						svg::Handle::from_memory(CPU_SVG)
					}
				}
			};
			let s = size.unwrap_or(16) as f32;
			svg(handle).width(Length::Fixed(s)).height(Length::Fixed(s)).into()
		}
		BarWidget::Workspaces { .. } => {
			let mut ws_row = row![].spacing(4);
			for ws in &s.workspaces {
				let dot = if ws.window_count > 0 { "•" } else { " " };
				let label = format!("{} {}", ws.id, dot);
				let id = ws.id;
				let active = ws.active;

				ws_row = ws_row.push(
					iced::widget::button(
						container(text(label).font(iced::Font::MONOSPACE).size(13))
							.padding([2, 8])
							.center_x(Length::Shrink)
							.center_y(Length::Shrink),
					)
					.on_press(Message::WorkspaceClick(id))
					.style(move |_: &Theme, status| {
						let mut style = iced::widget::button::Style {
							background: if active {
								Some(Background::Color(Color::from_rgba(1.0, 1.0, 1.0, 0.08)))
							} else {
								None
							},
							border: iced::Border {
								radius: 6.0.into(),
								color: if active {
									Color::from_rgb(0.1, 0.42, 0.54)
								} else {
									Color::TRANSPARENT
								},
								width: if active { 1.0 } else { 0.0 },
							},
							..Default::default()
						};
						if matches!(status, iced::widget::button::Status::Hovered) {
							style.background = Some(Background::Color(Color::from_rgba(1.0, 1.0, 1.0, 0.12)));
						}
						style
					}),
				);
			}
			ws_row.into()
		}
		BarWidget::Cpu { show_label } => {
			let mut r = row![
				svg(svg::Handle::from_memory(CPU_SVG))
					.width(Length::Fixed(16.0))
					.height(Length::Fixed(16.0))
			]
			.spacing(4)
			.align_y(Alignment::Center);
			if *show_label {
				r = r.push(text("CPU"));
			}
			r.push(text(format!("{:.0}%", s.cpu_usage)).size(12)).into()
		}
		BarWidget::Memory { show_label } => {
			let mut r = row![
				svg(svg::Handle::from_memory(MEM_SVG))
					.width(Length::Fixed(16.0))
					.height(Length::Fixed(16.0))
			]
			.spacing(4)
			.align_y(Alignment::Center);
			if *show_label {
				r = r.push(text("MEM"));
			}
			r.push(text(format!("{:.0}%", s.mem_usage)).size(12)).into()
		}
		BarWidget::Battery => {
			if let Some(p) = s.battery_percent {
				let handle = if p > 85 {
					svg::Handle::from_memory(BAT_4_SVG)
				} else if p > 60 {
					svg::Handle::from_memory(BAT_3_SVG)
				} else if p > 35 {
					svg::Handle::from_memory(BAT_2_SVG)
				} else if p > 10 {
					svg::Handle::from_memory(BAT_1_SVG)
				} else {
					svg::Handle::from_memory(BAT_EMPTY_SVG)
				};

				row![
					svg(handle).width(Length::Fixed(16.0)).height(Length::Fixed(16.0)),
					text(format!("{}%", p)).size(12)
				]
				.spacing(4)
				.align_y(Alignment::Center)
				.into()
			} else {
				row![].into()
			}
		}
		BarWidget::Clock { format } => text(Local::now().format(format).to_string())
			.size(13)
			.font(iced::Font::MONOSPACE)
			.into(),
		BarWidget::ActiveWindow { max_length } => {
			let mut t = s.focused_title.clone();
			if let Some(max) = max_length {
				if t.len() > *max as usize {
					t.truncate((*max as usize).saturating_sub(1));
					t.push('…');
				}
			}
			text(t).size(13).into()
		}
		BarWidget::Spacer { width } => {
			let w = width.unwrap_or(8) as f32;
			container(row![]).width(Length::Fixed(w)).into()
		}
		BarWidget::Row { children, spacing } => {
			let mut r = row![].spacing(spacing.unwrap_or(4) as f32);
			for child in children {
				r = r.push(render_widget(child, s));
			}
			r.into()
		}
	}
}

fn view(s: &BarState) -> Element<'_, Message> {
	let left = row(s.config.left.iter().map(|w| render_widget(w, s))).align_y(Alignment::Center);
	let center = container(row(s.config.center.iter().map(|w| render_widget(w, s))))
		.width(Length::Fill)
		.center_x(Length::Fill);
	let right = row(s.config.right.iter().map(|w| render_widget(w, s))).align_y(Alignment::Center);

	container(row![left, center, right].align_y(Alignment::Center).width(Length::Fill))
		.width(Length::Fill)
		.height(Length::Fixed(s.config.height as f32))
		.style(move |_: &Theme| container::Style {
			background: Some(Background::Color(Color::from_rgba(0.11, 0.11, 0.18, 0.95))),
			..Default::default()
		})
		.into()
}

fn subscription(_s: &BarState) -> Subscription<Message> {
	let clock = iced::time::every(std::time::Duration::from_secs(1)).map(|_| Message::Tick);

	let ipc = iced::Subscription::run_with(0u64, |_| {
		iced::stream::channel(32, |mut output: futures::channel::mpsc::Sender<Message>| async move {
			loop {
				match vitrum_ipc::IpcEventSubscriber::connect(&vitrum_ipc::event_socket_path()).await {
					Ok(mut sub) => loop {
						match sub.next_event().await {
							Ok((opcode, payload)) => {
								let event = match opcode {
									vitrum_ipc::Opcode::EventWorkspaceChanged => rmp_serde::from_slice(&payload)
										.ok()
										.map(|ev| LocalIpcEvent::WorkspaceChanged(ev)),
									vitrum_ipc::Opcode::EventWindowOpened => {
										rmp_serde::from_slice(&payload).ok().map(|ev| LocalIpcEvent::WindowOpened(ev))
									}
									vitrum_ipc::Opcode::EventWindowMoved => {
										rmp_serde::from_slice(&payload).ok().map(|ev| LocalIpcEvent::WindowMoved(ev))
									}
									_ => None,
								};

								if let Some(event) = event {
									let _ = output.try_send(Message::IpcEvent(event));
								}
							}
							Err(_) => break,
						}
					},
					Err(_) => tokio::time::sleep(std::time::Duration::from_secs(2)).await,
				}
			}
		})
	});

	let sys_timer = iced::time::every(std::time::Duration::from_secs(2)).map(|_| Message::SystemUpdate);

	Subscription::batch([clock, ipc, sys_timer])
}

fn format_time() -> String {
	Local::now().format("%H:%M  %a %d %b").to_string()
}

fn evaluate_bar_vt(s: &mut BarState) {
	let path = dirs::config_dir().map(|d| d.join("vitrum").join("bar.vt"));
	let source = if let Some(p) = path.filter(|p| p.exists()) {
		std::fs::read_to_string(p).unwrap_or_else(|_| include_str!("../resources/default.vt").to_string())
	} else {
		include_str!("../resources/default.vt").to_string()
	};

	let arena = Arena::new(64 * 1024);
	let eval = Evaluator::with_handler(&arena, &BarBuiltins);

	let mut scope = Scope::new();
	scope.define(arena.alloc_str("accent"), Value::String(s.theme.accent.clone()));
	scope.define(arena.alloc_str("bg"), Value::String(s.theme.background.clone()));
	scope.define(arena.alloc_str("surface"), Value::String(s.theme.surface.clone()));
	scope.define(arena.alloc_str("text"), Value::String(s.theme.text.clone()));
	scope.define(arena.alloc_str("muted"), Value::String(s.theme.text_muted.clone()));

	if let Ok(acc) = vitrum_config::script::eval_string_with_scope(&source, &eval, scope) {
		if let Some(bar_val) = acc.sections.get("bar") {
			if let Ok(config) = BarConfig::deserialize(ValueDeserializer(bar_val.clone())) {
				s.config = config;
			}
		}
	}
}

fn main() -> iced_layershell::Result {
	tracing_subscriber::fmt()
		.with_env_filter(tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
		.init();

	let config = BarConfig::default();

	let anchor = match config.position {
		BarPosition::Top => Anchor::Top | Anchor::Left | Anchor::Right,
		BarPosition::Bottom => Anchor::Bottom | Anchor::Left | Anchor::Right,
	};

	let layer = LayerShellSettings {
		size: Some((0, config.height)),
		anchor,
		layer: Layer::Top,
		keyboard_interactivity: KeyboardInteractivity::None,
		exclusive_zone: config.height as i32,
		..Default::default()
	};

	let settings = Settings {
		id: Some("vitrum-bar".to_string()),
		layer_settings: layer,
		..Default::default()
	};

	iced_layershell::application(boot, "vitrum-bar", update, view)
		.settings(settings)
		.subscription(subscription)
		.run()
}
