use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand, ValueEnum};
use vitrum_ipc::{Opcode, protocol::IpcEventMask};

#[derive(Parser)]
#[command(name = "vitrumctl")]
#[command(about = "Control the Vitrum compositor", long_about = None)]
struct Cli {
	#[command(subcommand)]
	command: Commands,

	#[arg(long, global = true)]
	json: bool,
}

#[derive(Subcommand)]
enum Commands {
	Kill {
		#[arg(long)]
		id: Option<u64>,
	},

	Focus {
		direction: FocusDirection,
	},

	FocusWindow {
		id: u64,
	},

	Workspace {
		workspace: u32,
	},

	Move {
		workspace: u32,
		#[arg(long)]
		id: Option<u64>,
	},

	Layout {
		mode: LayoutModeArg,
		#[arg(long)]
		workspace: Option<u32>,
	},

	Float {
		#[arg(long)]
		id: Option<u64>,
	},

	Fullscreen {
		#[arg(long)]
		id: Option<u64>,
	},

	Reload,

	Spawn {
		#[arg(required = true, num_args = 1.., allow_hyphen_values = true)]
		cmd: Vec<String>,
	},

	Query {
		target: String,
	},

	Wallpaper {
		path: Option<String>,

		#[arg(long, default_value = "fill")]
		fit: String,

		#[arg(long)]
		color: Option<String>,

		#[arg(long)]
		slideshow_dir: Option<String>,

		#[arg(long, default_value = "300")]
		interval: u32,
	},

	Theme {
		#[command(subcommand)]
		command: ThemeCommands,
	},

	Events {
		#[arg(long)]
		json: bool,
	},

	SwapWindow {
		a: u64,
		#[arg(long)]
		with: u64,
	},

	SetOpacity {
		value: f32,
		#[arg(long)]
		id: Option<u64>,
	},

	Resize {
		#[arg(long)]
		dw: i32,
		#[arg(long)]
		dh: i32,
		#[arg(long)]
		id: Option<u64>,
	},

	Lock,

	Pin {
		#[arg(long)]
		id: Option<u64>,
	},

	Osd {
		icon: String,

		#[arg(long)]
		value: Option<u8>,

		#[arg(long)]
		text: Option<String>,
	},

	Notify {
		app_name: Option<String>,
		#[arg(long)]
		summary: String,
		#[arg(long)]
		body: String,
		#[arg(long)]
		icon: Option<String>,
		#[arg(long)]
		timeout: Option<u32>,
	},

	#[command(hide = true)]
	Dispatch {
		cmd: String,
	},

	Clip,
}

#[derive(Subcommand)]
enum ThemeCommands {
	Set {
		#[arg(long)]
		accent: Option<String>,
		#[arg(long)]
		background: Option<String>,
		#[arg(long)]
		surface: Option<String>,
		#[arg(long)]
		text: Option<String>,
		#[arg(long)]
		border: Option<String>,
		#[arg(long)]
		border_width: Option<u32>,
		#[arg(long)]
		gaps_inner: Option<u32>,
		#[arg(long)]
		gaps_outer: Option<u32>,
		#[arg(long)]
		corner_radius: Option<u32>,
		#[arg(long)]
		cursor_theme: Option<String>,
		#[arg(long)]
		cursor_size: Option<u32>,
		#[arg(long)]
		icon_theme: Option<String>,
		#[arg(long)]
		color_scheme: Option<String>,
		#[arg(long)]
		sound_theme: Option<String>,
		#[arg(long)]
		font_ui: Option<String>,
		#[arg(long)]
		font_ui_size: Option<u32>,
		#[arg(long)]
		font_mono: Option<String>,
		#[arg(long)]
		font_mono_size: Option<u32>,
		#[arg(long)]
		font_document: Option<String>,
		#[arg(long)]
		font_document_size: Option<u32>,
		#[arg(long)]
		dpi: Option<u32>,
		#[arg(long)]
		gdk_scale: Option<u32>,
	},
}

#[derive(Copy, Clone, Debug, ValueEnum)]
enum FocusDirection {
	Up,
	Down,
	Left,
	Right,
}

#[derive(Copy, Clone, Debug, ValueEnum)]
enum LayoutModeArg {
	Dwindle,
	MasterStack,
	Floating,
}

#[tokio::main]
async fn main() -> Result<()> {
	let cli = Cli::parse();

	tracing_subscriber::fmt::init();

	match cli.command {
		Commands::Kill { id } => {
			dispatch_kill(id).await?;
		}
		Commands::Focus { direction } => {
			dispatch_focus(direction).await?;
		}
		Commands::FocusWindow { id } => {
			dispatch_focus_window(id).await?;
		}
		Commands::Workspace { workspace } => {
			dispatch_workspace(workspace).await?;
		}
		Commands::Move { workspace, id } => {
			dispatch_move(workspace, id).await?;
		}
		Commands::Layout { mode, workspace } => {
			dispatch_layout(mode, workspace).await?;
		}
		Commands::Float { id } => {
			dispatch_float(id).await?;
		}
		Commands::Fullscreen { id } => {
			dispatch_fullscreen(id).await?;
		}
		Commands::Reload => {
			dispatch_reload().await?;
		}
		Commands::Spawn { cmd } => {
			dispatch_spawn(cmd).await?;
		}
		Commands::Query { target } => {
			handle_query(&target, cli.json).await?;
		}
		Commands::Wallpaper {
			path,
			fit,
			color,
			slideshow_dir,
			interval,
		} => {
			dispatch_wallpaper(path, fit, color, slideshow_dir, interval).await?;
		}
		Commands::Theme { command } => {
			dispatch_theme(command).await?;
		}
		Commands::Events { json } => {
			dispatch_events(json).await?;
		}
		Commands::SwapWindow { a, with } => {
			dispatch_swap(a, with).await?;
		}
		Commands::SetOpacity { value, id } => {
			dispatch_opacity(value, id).await?;
		}
		Commands::Resize { dw, dh, id } => {
			dispatch_resize(dw, dh, id).await?;
		}
		Commands::Lock => {
			dispatch_lock().await?;
		}
		Commands::Pin { id } => {
			dispatch_pin(id).await?;
		}
		Commands::Osd { icon, value, text } => {
			dispatch_osd(icon, value, text).await?;
		}
		Commands::Dispatch { cmd } => {
			dispatch_command(&cmd).await?;
		}
		Commands::Notify {
			app_name,
			summary,
			body,
			icon,
			timeout,
		} => {
			dispatch_notify(app_name, summary, body, icon, timeout).await?;
		}
		Commands::Clip => {
			dispatch_clip().await?;
		}
	}

	Ok(())
}

async fn dispatch_focus(direction: FocusDirection) -> Result<()> {
	use vitrum_ipc::protocol::commands::FocusDirection as FocusDirectionCmd;
	use vitrum_ipc::{Direction, IpcClient, Opcode};

	let socket = vitrum_ipc::command_socket_path();
	let mut client = IpcClient::connect(&socket).await?;

	let dir = match direction {
		FocusDirection::Up => Direction::Up,
		FocusDirection::Down => Direction::Down,
		FocusDirection::Left => Direction::Left,
		FocusDirection::Right => Direction::Right,
	};

	let (opcode, payload) = client
		.send_request(Opcode::FocusDirection, &FocusDirectionCmd { dir })
		.await?;
	ensure_ok(opcode, &payload)?;
	println!("Focused window moved {direction:?}");
	Ok(())
}

async fn dispatch_focus_window(id: u64) -> Result<()> {
	use vitrum_ipc::protocol::commands::FocusWindow;
	use vitrum_ipc::{IpcClient, Opcode, WindowId};

	let socket = vitrum_ipc::command_socket_path();
	let mut client = IpcClient::connect(&socket).await?;

	let (opcode, payload) = client
		.send_request(Opcode::FocusWindow, &FocusWindow { id: WindowId(id) })
		.await?;
	ensure_ok(opcode, &payload)?;
	println!("Focused window {}", id);
	Ok(())
}

async fn dispatch_kill(id: Option<u64>) -> Result<()> {
	use vitrum_ipc::protocol::commands::KillWindow;
	use vitrum_ipc::{IpcClient, Opcode};

	let socket = vitrum_ipc::command_socket_path();
	let mut client = IpcClient::connect(&socket).await?;
	let Some(window) = resolve_target_window(&mut client, id).await? else {
		println!("No matching window found");
		return Ok(());
	};

	let (opcode, payload) = client.send_request(Opcode::KillWindow, &KillWindow { id: window.id }).await?;
	ensure_ok(opcode, &payload)?;
	println!("Killed window {}", window.id.0);
	Ok(())
}

async fn dispatch_workspace(workspace: u32) -> Result<()> {
	use vitrum_ipc::protocol::commands::SwitchWorkspace;
	use vitrum_ipc::{IpcClient, Opcode};

	let socket = vitrum_ipc::command_socket_path();
	let mut client = IpcClient::connect(&socket).await?;

	let (opcode, payload) = client
		.send_request(Opcode::SwitchWorkspace, &SwitchWorkspace { id: workspace })
		.await?;
	ensure_ok(opcode, &payload)?;
	println!("Switched to workspace {}", workspace);
	Ok(())
}

async fn dispatch_move(workspace: u32, id: Option<u64>) -> Result<()> {
	use vitrum_ipc::protocol::commands::MoveToWorkspace;
	use vitrum_ipc::{IpcClient, Opcode};

	let socket = vitrum_ipc::command_socket_path();
	let mut client = IpcClient::connect(&socket).await?;
	let Some(window) = resolve_target_window(&mut client, id).await? else {
		println!("No matching window found");
		return Ok(());
	};

	let (opcode, payload) = client
		.send_request(
			Opcode::MoveToWorkspace,
			&MoveToWorkspace {
				window: window.id,
				workspace,
			},
		)
		.await?;
	ensure_ok(opcode, &payload)?;
	println!("Moved window {} to workspace {}", window.id.0, workspace);
	Ok(())
}

async fn dispatch_layout(mode: LayoutModeArg, workspace: Option<u32>) -> Result<()> {
	use vitrum_ipc::protocol::commands::SetLayout;
	use vitrum_ipc::{IpcClient, LayoutMode, Opcode};

	let socket = vitrum_ipc::command_socket_path();
	let mut client = IpcClient::connect(&socket).await?;

	let workspace_id = match workspace {
		Some(id) => id,
		None => {
			let Some(id) = resolve_active_workspace_id(&mut client).await? else {
				println!("Unable to determine active workspace");
				return Ok(());
			};
			id
		}
	};

	let layout_mode = match mode {
		LayoutModeArg::Dwindle => LayoutMode::Dwindle,
		LayoutModeArg::MasterStack => LayoutMode::MasterStack,
		LayoutModeArg::Floating => LayoutMode::Floating,
	};

	let (opcode, payload) = client
		.send_request(
			Opcode::SetLayout,
			&SetLayout {
				workspace: workspace_id,
				mode: layout_mode,
			},
		)
		.await?;
	ensure_ok(opcode, &payload)?;
	println!("Set workspace {} layout to {:?}", workspace_id, mode);
	Ok(())
}

async fn dispatch_float(id: Option<u64>) -> Result<()> {
	use vitrum_ipc::protocol::commands::FloatWindow;
	use vitrum_ipc::{IpcClient, Opcode};

	let socket = vitrum_ipc::command_socket_path();
	let mut client = IpcClient::connect(&socket).await?;
	let Some(window) = resolve_target_window(&mut client, id).await? else {
		println!("No matching window found");
		return Ok(());
	};

	let new_floating = !window.floating;
	let (opcode, payload) = client
		.send_request(
			Opcode::FloatWindow,
			&FloatWindow {
				id: window.id,
				floating: new_floating,
			},
		)
		.await?;
	ensure_ok(opcode, &payload)?;
	println!("Window {} floating: {}", window.id.0, new_floating);

	Ok(())
}

async fn dispatch_fullscreen(id: Option<u64>) -> Result<()> {
	use vitrum_ipc::protocol::commands::ToggleFullscreen;
	use vitrum_ipc::{IpcClient, Opcode};

	let socket = vitrum_ipc::command_socket_path();
	let mut client = IpcClient::connect(&socket).await?;
	let Some(window) = resolve_target_window(&mut client, id).await? else {
		println!("No matching window found");
		return Ok(());
	};

	let (opcode, payload) = client
		.send_request(Opcode::ToggleFullscreen, &ToggleFullscreen { id: window.id })
		.await?;
	ensure_ok(opcode, &payload)?;
	println!("Toggled fullscreen for window {}", window.id.0);
	Ok(())
}

async fn dispatch_reload() -> Result<()> {
	use vitrum_ipc::{IpcClient, Opcode};

	let socket = vitrum_ipc::command_socket_path();
	let mut client = IpcClient::connect(&socket).await?;

	let (opcode, payload) = client.send_request(Opcode::ReloadConfig, &()).await?;
	ensure_ok(opcode, &payload)?;
	println!("Config reloaded");
	Ok(())
}

async fn dispatch_spawn(cmd: Vec<String>) -> Result<()> {
	use vitrum_ipc::protocol::commands::Spawn;
	use vitrum_ipc::{IpcClient, Opcode};

	let socket = vitrum_ipc::command_socket_path();
	let mut client = IpcClient::connect(&socket).await?;
	let command_str = cmd.join(" ");

	let (opcode, payload) = client
		.send_request(
			Opcode::Spawn,
			&Spawn {
				cmd: command_str.clone(),
			},
		)
		.await?;
	ensure_ok(opcode, &payload)?;
	println!("Spawned command: {}", command_str);
	Ok(())
}

async fn dispatch_wallpaper(
	path: Option<String>,
	fit: String,
	color: Option<String>,
	slideshow_dir: Option<String>,
	interval: u32,
) -> Result<()> {
	use vitrum_ipc::IpcClient;

	let socket = vitrum_ipc::command_socket_path();
	let mut client = IpcClient::connect(&socket).await?;

	let (opcode, payload) = match color {
		Some(color) => {
			client
				.send_request(
					Opcode::WallpaperColor,
					&vitrum_ipc::protocol::commands::WallpaperColor { color },
				)
				.await?
		}
		None => match slideshow_dir {
			Some(dir) => {
				client
					.send_request(
						Opcode::WallpaperSlideshow,
						&vitrum_ipc::protocol::commands::WallpaperSlideshow {
							dir,
							interval_secs: interval,
						},
					)
					.await?
			}
			None => match path {
				Some(path) => {
					let wallpaper_fit = match fit.as_str() {
						"fit" => vitrum_ipc::WallpaperFit::Fit,
						"center" => vitrum_ipc::WallpaperFit::Center,
						"tile" => vitrum_ipc::WallpaperFit::Tile,
						_ => vitrum_ipc::WallpaperFit::Fill,
					};
					client
						.send_request(
							Opcode::WallpaperSet,
							&vitrum_ipc::protocol::commands::WallpaperSet {
								path,
								fit: wallpaper_fit,
							},
						)
						.await?
				}
				None => bail!("Specify a wallpaper path, --color, or --slideshow-dir"),
			},
		},
	};

	ensure_ok(opcode, &payload)?;
	println!("Wallpaper updated");
	Ok(())
}

async fn dispatch_theme(command: ThemeCommands) -> Result<()> {
	use vitrum_ipc::{IpcClient, ThemePatch};

	let socket = vitrum_ipc::command_socket_path();
	let mut client = IpcClient::connect(&socket).await?;

	match command {
		ThemeCommands::Set {
			accent,
			background,
			surface,
			text,
			border,
			border_width,
			gaps_inner,
			gaps_outer,
			corner_radius,
			cursor_theme,
			cursor_size,
			icon_theme,
			color_scheme,
			sound_theme,
			font_ui,
			font_ui_size,
			font_mono,
			font_mono_size,
			font_document,
			font_document_size,
			dpi,
			gdk_scale,
		} => {
			let patch = ThemePatch {
				accent,
				background,
				surface,
				surface_raised: None,
				text,
				text_muted: None,
				border,
				error: None,
				warning: None,
				success: None,
				border_width,
				gaps_inner,
				gaps_outer,
				corner_radius,
				cursor_theme,
				cursor_size,
				icon_theme,
				color_scheme,
				sound_theme,
				font_ui,
				font_ui_size,
				font_mono,
				font_mono_size,
				font_document,
				font_document_size,
				dpi,
				gdk_scale,
			};
			let (opcode, payload) = client
				.send_request(Opcode::SetTheme, &vitrum_ipc::protocol::commands::SetTheme { patch })
				.await?;
			ensure_ok(opcode, &payload)?;
			println!("Theme updated");
		}
	}

	Ok(())
}

async fn handle_query(target: &str, json: bool) -> Result<()> {
	use vitrum_ipc::protocol::responses::*;
	use vitrum_ipc::{IpcClient, Opcode};

	let socket = vitrum_ipc::command_socket_path();
	let mut client = IpcClient::connect(&socket).await?;

	match target {
		"windows" => {
			let (opcode, payload) = client.send_request(Opcode::GetWindows, &()).await?;
			ensure_ok(opcode, &payload)?;
			let resp: WindowsResponse = rmp_serde::from_slice(&payload)?;
			if json {
				println!("{}", serde_json::to_string_pretty(&resp.windows)?);
			} else {
				println!("Windows:");
				for window in resp.windows {
					println!(
						"  {} - {} (ws:{}, float:{}, fs:{}, opacity:{:.0}%)",
						window.id.0,
						window.title,
						window.workspace,
						window.floating,
						window.fullscreen,
						window.opacity * 100.0
					);
				}
			}
		}
		"workspaces" => {
			let (opcode, payload) = client.send_request(Opcode::GetWorkspaces, &()).await?;
			ensure_ok(opcode, &payload)?;
			let resp: WorkspacesResponse = rmp_serde::from_slice(&payload)?;
			if json {
				println!("{}", serde_json::to_string_pretty(&resp.workspaces)?);
			} else {
				println!("Workspaces:");
				for ws in resp.workspaces {
					println!(
						"  {}: {} windows{}",
						ws.id,
						ws.window_count,
						if ws.active { " (active)" } else { "" }
					);
				}
			}
		}
		"outputs" => {
			let (opcode, payload) = client.send_request(Opcode::GetOutputs, &()).await?;
			ensure_ok(opcode, &payload)?;
			let resp: OutputsResponse = rmp_serde::from_slice(&payload)?;
			if json {
				println!("{}", serde_json::to_string_pretty(&resp.outputs)?);
			} else {
				println!("Outputs:");
				for output in resp.outputs {
					println!(
						"  {} ({}) {}x{} scale={}{}",
						output.id,
						output.name,
						output.size.0,
						output.size.1,
						output.scale,
						if output.primary { " primary" } else { "" }
					);
				}
			}
		}
		"layouts" => {
			let (opcode, payload) = client.send_request(Opcode::GetLayouts, &()).await?;
			ensure_ok(opcode, &payload)?;
			let resp: LayoutsResponse = rmp_serde::from_slice(&payload)?;
			if json {
				println!("{}", serde_json::to_string_pretty(&resp.layouts)?);
			} else {
				println!("Layouts:");
				for layout in resp.layouts {
					println!(
						"  workspace {}: {:?}{}",
						layout.workspace,
						layout.mode,
						if layout.active { " (active)" } else { "" }
					);
				}
			}
		}
		"state" => {
			let (opcode, payload) = client.send_request(Opcode::GetState, &()).await?;
			ensure_ok(opcode, &payload)?;
			let resp: StateResponse = rmp_serde::from_slice(&payload)?;
			if json {
				println!("{}", serde_json::to_string_pretty(&resp.state)?);
			} else {
				println!("State:");
				println!("  active workspace: {}", resp.state.active_workspace);
				println!(
					"  focused window: {}",
					resp.state
						.focused_window
						.map(|id| id.0.to_string())
						.unwrap_or_else(|| "none".to_string())
				);
				println!("  windows: {}", resp.state.windows.len());
			}
		}
		"focused" => {
			let (opcode_s, payload_s) = client.send_request(Opcode::GetState, &()).await?;
			ensure_ok(opcode_s, &payload_s)?;
			let state_resp: StateResponse = rmp_serde::from_slice(&payload_s)?;

			let (opcode_w, payload_w) = client.send_request(Opcode::GetWindows, &()).await?;
			ensure_ok(opcode_w, &payload_w)?;
			let windows_resp: WindowsResponse = rmp_serde::from_slice(&payload_w)?;

			let focused_window = state_resp
				.state
				.focused_window
				.and_then(|id| windows_resp.windows.into_iter().find(|w| w.id == id));

			if json {
				println!("{}", serde_json::to_string_pretty(&focused_window)?);
			} else {
				match focused_window {
					Some(window) => {
						println!("Focused window:");
						println!("  id: {}", window.id.0);
						println!("  title: {}", window.title);
						println!("  app id: {}", window.app_id);
						println!("  workspace: {}", window.workspace);
						println!("  floating: {}", window.floating);
						println!("  fullscreen: {}", window.fullscreen);
					}
					None => println!("No focused window"),
				}
			}
		}
		"theme" => {
			let (opcode, payload) = client.send_request(Opcode::GetTheme, &()).await?;
			ensure_ok(opcode, &payload)?;
			let resp: ThemeResponse = rmp_serde::from_slice(&payload)?;
			let theme = resp.theme;

			if json {
				println!("{}", serde_json::to_string_pretty(&theme)?);
			} else {
				println!("Theme:");
				println!("  accent: {}", theme.accent);
				println!("  background: {}", theme.background);
				println!("  surface: {}", theme.surface);
				println!("  surface_raised: {}", theme.surface_raised);
				println!("  text: {}", theme.text);
				println!("  text_muted: {}", theme.text_muted);
				println!("  border: {}", theme.border);
				println!("  error: {}", theme.error);
				println!("  warning: {}", theme.warning);
				println!("  success: {}", theme.success);
				println!("  border_width: {}", theme.border_width);
				println!("  gaps_inner: {}", theme.gaps_inner);
				println!("  gaps_outer: {}", theme.gaps_outer);
				println!("  corner_radius: {}", theme.corner_radius);
				println!("  cursor_theme: {}", theme.cursor_theme);
				println!("  cursor_size: {}", theme.cursor_size);
				println!("  icon_theme: {}", theme.icon_theme);
				println!("  color_scheme: {}", theme.color_scheme);
				println!("  font_ui: {} {}", theme.font_ui, theme.font_ui_size);
				println!("  font_mono: {} {}", theme.font_mono, theme.font_mono_size);
				println!("  dpi: {}", theme.dpi);
				println!("  gdk_scale: {}", theme.gdk_scale);
			}
		}
		"active" => {
			let (opcode_ws, payload_ws) = client.send_request(Opcode::GetWorkspaces, &()).await?;
			ensure_ok(opcode_ws, &payload_ws)?;
			let ws_resp: WorkspacesResponse = rmp_serde::from_slice(&payload_ws)?;

			let (opcode_w, payload_w) = client.send_request(Opcode::GetWindows, &()).await?;
			ensure_ok(opcode_w, &payload_w)?;
			let w_resp: WindowsResponse = rmp_serde::from_slice(&payload_w)?;

			let active_workspace = ws_resp.workspaces.into_iter().find(|ws| ws.active).map(|ws| ws.id);
			let active_windows = if let Some(aw) = active_workspace {
				w_resp.windows.into_iter().filter(|w| w.workspace == aw).collect::<Vec<_>>()
			} else {
				Vec::new()
			};

			if json {
				let output = serde_json::json!({
					"active_workspace": active_workspace,
					"window_count": active_windows.len(),
					"windows": active_windows,
				});
				println!("{}", serde_json::to_string_pretty(&output)?);
			} else {
				match active_workspace {
					Some(id) => {
						println!("Active workspace: {}", id);
						if active_windows.is_empty() {
							println!("No windows in active workspace");
						} else {
							println!("Windows in active workspace:");
							for window in active_windows {
								println!("  {} - {} ({})", window.id.0, window.title, window.app_id);
							}
						}
					}
					None => println!("No active workspace"),
				}
			}
		}
		_ => {
			println!(
				"Unknown query target: {}. Available: windows, workspaces, outputs, layouts, state, focused, theme, active",
				target
			);
			return Ok(());
		}
	};
	Ok(())
}

async fn dispatch_swap(a: u64, b: u64) -> Result<()> {
	use vitrum_ipc::protocol::commands::SwapWindow;
	use vitrum_ipc::{IpcClient, Opcode, WindowId};

	let socket = vitrum_ipc::command_socket_path();
	let mut client = IpcClient::connect(&socket).await?;

	let (opcode, payload) = client
		.send_request(
			Opcode::SwapWindow,
			&SwapWindow {
				a: WindowId(a),
				b: WindowId(b),
			},
		)
		.await?;
	ensure_ok(opcode, &payload)?;
	println!("Swapped window {} with {}", a, b);
	Ok(())
}

async fn dispatch_opacity(value: f32, id: Option<u64>) -> Result<()> {
	use vitrum_ipc::protocol::commands::SetOpacity;
	use vitrum_ipc::{IpcClient, Opcode};

	let socket = vitrum_ipc::command_socket_path();
	let mut client = IpcClient::connect(&socket).await?;
	let Some(window_id) = (if let Some(id) = id {
		Ok(Some(vitrum_ipc::WindowId(id)))
	} else {
		resolve_active_workspace_window(&mut client).await
	})?
	else {
		bail!("No matching window found");
	};

	let (opcode, payload) = client
		.send_request(Opcode::SetOpacity, &SetOpacity { id: window_id, value })
		.await?;
	ensure_ok(opcode, &payload)?;
	println!("Set window {} opacity to {:.0}%", window_id.0, value * 100.0);
	Ok(())
}

async fn dispatch_resize(dw: i32, dh: i32, id: Option<u64>) -> Result<()> {
	use vitrum_ipc::protocol::commands::ResizeDelta;
	use vitrum_ipc::{IpcClient, Opcode};

	let socket = vitrum_ipc::command_socket_path();
	let mut client = IpcClient::connect(&socket).await?;

	let Some(window_id) = (if let Some(id) = id {
		Ok(Some(vitrum_ipc::WindowId(id)))
	} else {
		resolve_active_workspace_window(&mut client).await
	})?
	else {
		bail!("No matching window found");
	};

	let (opcode, payload) = client
		.send_request(Opcode::ResizeDelta, &ResizeDelta { id: window_id, dw, dh })
		.await?;
	ensure_ok(opcode, &payload)?;
	println!("Resized window {} by {},{}", window_id.0, dw, dh);
	Ok(())
}

async fn dispatch_lock() -> Result<()> {
	use vitrum_ipc::{IpcClient, Opcode};

	let socket = vitrum_ipc::command_socket_path();
	let mut client = IpcClient::connect(&socket).await?;

	let (opcode, payload) = client.send_request(Opcode::Lock, &()).await?;
	ensure_ok(opcode, &payload)?;
	println!("Locked session");
	Ok(())
}

async fn dispatch_pin(id: Option<u64>) -> Result<()> {
	let _ = id;
	println!("Window pin toggled (use IPC SetOpacity/FloatWindow for now)");
	Ok(())
}

async fn resolve_active_workspace_window(client: &mut vitrum_ipc::IpcClient) -> Result<Option<vitrum_ipc::WindowId>> {
	let Some(active_workspace) = resolve_active_workspace_id(client).await? else {
		return Ok(None);
	};

	let (opcode, payload) = client.send_request(vitrum_ipc::Opcode::GetWindows, &()).await?;
	ensure_ok(opcode, &payload)?;

	let resp: vitrum_ipc::WindowsResponse = rmp_serde::from_slice(&payload)?;

	let window_id = resp
		.windows
		.into_iter()
		.filter(|w| w.workspace == active_workspace)
		.max_by_key(|w| w.id.0)
		.map(|w| w.id);

	Ok(window_id)
}

async fn resolve_active_workspace_id(client: &mut vitrum_ipc::IpcClient) -> Result<Option<u32>> {
	let (opcode, payload) = client.send_request(vitrum_ipc::Opcode::GetWorkspaces, &()).await?;
	ensure_ok(opcode, &payload)?;

	let resp: vitrum_ipc::WorkspacesResponse = rmp_serde::from_slice(&payload)?;
	let active_workspace = resp.workspaces.into_iter().find(|ws| ws.active).map(|ws| ws.id);

	Ok(active_workspace)
}

async fn resolve_target_window(
	client: &mut vitrum_ipc::IpcClient,
	id: Option<u64>,
) -> Result<Option<vitrum_ipc::WindowInfo>> {
	let (opcode, payload) = client.send_request(vitrum_ipc::Opcode::GetWindows, &()).await?;
	ensure_ok(opcode, &payload)?;

	let resp: vitrum_ipc::WindowsResponse = rmp_serde::from_slice(&payload)?;
	let windows = resp.windows;

	if let Some(id) = id {
		return Ok(windows.into_iter().find(|w| w.id.0 == id));
	}

	let Some(active_id) = resolve_active_workspace_window(client).await? else {
		return Ok(None);
	};

	Ok(windows.into_iter().find(|w| w.id == active_id))
}

async fn dispatch_clip() -> Result<()> {
	use vitrum_ipc::{IpcClient, Opcode};

	let socket = vitrum_ipc::command_socket_path();
	let mut client = IpcClient::connect(&socket).await?;

	let (opcode, payload) = client.send_request(Opcode::ShowClipboard, &()).await?;
	ensure_ok(opcode, &payload)?;
	println!("Clipboard picker triggered");
	Ok(())
}

fn ensure_ok(opcode: Opcode, payload: &[u8]) -> Result<()> {
	match opcode {
		Opcode::ResponseOk => Ok(()),
		Opcode::ResponseError => {
			let err: vitrum_ipc::protocol::IpcError =
				rmp_serde::from_slice(payload).context("Failed to deserialize error response")?;
			bail!("IPC error: {}", err.msg)
		}
		other => bail!("unexpected IPC response opcode: {:?}", other),
	}
}

async fn dispatch_events(json: bool) -> Result<()> {
	use vitrum_ipc::{IpcEventSubscriber, Opcode};

	let socket = vitrum_ipc::event_socket_path();
	let mut subscriber = IpcEventSubscriber::connect(&socket).await?;

	let (opcode, payload) = subscriber
		.send_request(
			Opcode::Subscribe,
			&vitrum_ipc::protocol::IpcSubscribe { mask: IpcEventMask::ALL },
		)
		.await?;
	ensure_ok(opcode, &payload)?;

	println!("Listening for events...");

	loop {
		match subscriber.next_event().await {
			Ok((opcode, payload)) => match opcode {
				Opcode::EventWindowOpened => {
					let ev: vitrum_ipc::protocol::events::WindowOpened = rmp_serde::from_slice(&payload)?;
					if json {
						println!("{}", serde_json::to_string(&ev)?);
					} else {
						println!(
							"WindowOpened: id={} app_id={} title='{}'",
							ev.window.id.0, ev.window.app_id, ev.window.title
						);
					}
				}
				Opcode::EventWindowClosed => {
					let ev: vitrum_ipc::protocol::events::WindowClosed = rmp_serde::from_slice(&payload)?;
					if json {
						println!("{}", serde_json::to_string(&ev)?);
					} else {
						println!("WindowClosed: id={}", ev.id.0);
					}
				}
				Opcode::EventWindowFocused => {
					let ev: vitrum_ipc::protocol::events::WindowFocused = rmp_serde::from_slice(&payload)?;
					if json {
						println!("{}", serde_json::to_string(&ev)?);
					} else {
						println!("WindowFocused: id={}", ev.id.0);
					}
				}
				Opcode::EventWorkspaceChanged => {
					let ev: vitrum_ipc::protocol::events::WorkspaceChanged = rmp_serde::from_slice(&payload)?;
					if json {
						println!("{}", serde_json::to_string(&ev)?);
					} else {
						println!("WorkspaceChanged: workspace={}", ev.workspace);
					}
				}
				Opcode::EventWindowMoved => {
					let ev: vitrum_ipc::protocol::events::WindowMoved = rmp_serde::from_slice(&payload)?;
					if json {
						println!("{}", serde_json::to_string(&ev)?);
					} else {
						println!(
							"WindowMoved: id={} from={} to={}",
							ev.id.0, ev.from_workspace, ev.to_workspace
						);
					}
				}
				Opcode::EventThemeChanged => {
					let ev: vitrum_ipc::protocol::events::ThemeChanged = rmp_serde::from_slice(&payload)?;
					if json {
						println!("{}", serde_json::to_string(&ev)?);
					} else {
						println!("ThemeChanged: accent={}", ev.theme.accent);
					}
				}
				Opcode::EventWallpaperChanged => {
					if json {
						println!("{{ \"event\": \"WallpaperChanged\" }}");
					} else {
						println!("WallpaperChanged");
					}
				}
				_ => {}
			},
			Err(e) => {
				eprintln!("Event stream error: {}", e);
				break;
			}
		}
	}

	Ok(())
}

async fn dispatch_osd(icon: String, value: Option<u8>, text: Option<String>) -> Result<()> {
	use vitrum_ipc::protocol::commands::Osd;
	use vitrum_ipc::{IpcClient, Opcode, OsdIcon};

	let socket = vitrum_ipc::command_socket_path();
	let mut client = IpcClient::connect(&socket).await?;

	let osd_icon = match icon.as_str() {
		"volume" => OsdIcon::Volume,
		"brightness" => OsdIcon::Brightness,
		"mute" => OsdIcon::Mute,
		"mic-mute" => OsdIcon::MicMute,
		"capslock" => OsdIcon::CapsLock,
		_ => OsdIcon::Custom,
	};

	let (opcode, payload) = client
		.send_request(
			Opcode::Osd,
			&Osd {
				icon: osd_icon,
				value,
				text,
			},
		)
		.await?;
	ensure_ok(opcode, &payload)?;
	Ok(())
}

async fn dispatch_command(cmd: &str) -> Result<()> {
	use vitrum_ipc::protocol::commands::Spawn;
	use vitrum_ipc::{IpcClient, Opcode};

	let socket = vitrum_ipc::command_socket_path();
	let mut client = IpcClient::connect(&socket).await?;

	let (opcode, payload) = match cmd {
		"lock" => client.send_request(Opcode::Lock, &()).await?,
		"reload-config" => client.send_request(Opcode::ReloadConfig, &()).await?,
		_ => client.send_request(Opcode::Spawn, &Spawn { cmd: cmd.to_string() }).await?,
	};

	ensure_ok(opcode, &payload)?;
	Ok(())
}

async fn dispatch_notify(
	app_name: Option<String>,
	summary: String,
	body: String,
	icon: Option<String>,
	timeout: Option<u32>,
) -> Result<()> {
	use vitrum_ipc::protocol::commands::IpcNotify;
	use vitrum_ipc::{IpcClient, Opcode};

	let socket = vitrum_ipc::command_socket_path();
	let mut client = IpcClient::connect(&socket).await?;

	let (opcode, payload) = client
		.send_request(
			Opcode::Notify,
			&IpcNotify {
				app_name: app_name.unwrap_or_else(|| "vitrumctl".to_string()),
				summary,
				body,
				icon,
				timeout,
			},
		)
		.await?;
	ensure_ok(opcode, &payload)?;
	println!("Notification sent via IPC");
	Ok(())
}
