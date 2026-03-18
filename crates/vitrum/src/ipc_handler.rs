use std::sync::mpsc::{Receiver, Sender, channel};

use tokio::sync::oneshot;
use tracing::{error, info, warn};
use vitrum_ipc::{
	CompositorState, Opcode, OutputInfo, ThemeSnapshot, WallpaperFit, WindowInfo, WorkspaceInfo, WorkspaceLayoutInfo,
	protocol::IpcEventMask,
};

use crate::backend::State;

#[derive(Debug, Clone)]
pub enum IpcCommand {
	FocusWindow {
		id: vitrum_ipc::WindowId,
	},
	MoveToWorkspace {
		window: vitrum_ipc::WindowId,
		workspace: u32,
	},
	SetLayout {
		workspace: u32,
		mode: vitrum_ipc::LayoutMode,
	},
	FloatWindow {
		id: vitrum_ipc::WindowId,
		floating: bool,
	},
	KillWindow {
		id: vitrum_ipc::WindowId,
	},
	GetWindows,
	GetWorkspaces,
	GetOutputs,
	GetLayouts,
	GetState,
	ReloadConfig,
	SwitchWorkspace {
		id: u32,
	},
	NextWorkspace,
	PrevWorkspace,
	ToggleFullscreen {
		id: vitrum_ipc::WindowId,
	},
	FocusDirection {
		dir: vitrum_ipc::Direction,
	},
	Spawn {
		cmd: String,
	},
	WallpaperSet {
		path: String,
		fit: WallpaperFit,
	},
	WallpaperColor {
		color: String,
	},
	WallpaperSlideshow {
		dir: String,
		interval_secs: u32,
	},
	SetTheme {
		patch: vitrum_ipc::ThemePatch,
	},
	Osd {
		icon: vitrum_ipc::OsdIcon,
		value: Option<u8>,
		text: Option<String>,
	},
	GetTheme,
	GetEnvironment,
	GetActiveWindow,
	SwapWindow {
		a: vitrum_ipc::WindowId,
		b: vitrum_ipc::WindowId,
	},
	SetOpacity {
		id: vitrum_ipc::WindowId,
		value: f32,
	},
	ResizeDelta {
		id: vitrum_ipc::WindowId,
		dw: i32,
		dh: i32,
	},
	Lock,
	Notify {
		app_name: String,
		summary: String,
		body: String,
		icon: Option<String>,
		timeout: Option<u32>,
	},
	ShowClipboard,
}

#[derive(Debug, Clone)]
pub enum IpcResponse {
	Ok,
	Error {
		msg: String,
	},
	Windows {
		windows: Vec<WindowInfo>,
	},
	Workspaces {
		workspaces: Vec<WorkspaceInfo>,
	},
	Outputs {
		outputs: Vec<OutputInfo>,
	},
	Layouts {
		layouts: Vec<WorkspaceLayoutInfo>,
	},
	State {
		state: CompositorState,
	},
	Theme {
		theme: ThemeSnapshot,
	},
	Environment {
		env: vitrum_ipc::EnvironmentSnapshot,
	},
	ActiveWindow {
		window: Option<WindowInfo>,
	},
}

#[derive(Debug, Clone)]
pub enum IpcEvent {
	WindowOpened {
		window: WindowInfo,
	},
	WindowClosed {
		id: vitrum_ipc::WindowId,
	},
	WindowFocused {
		id: vitrum_ipc::WindowId,
	},
	WorkspaceChanged {
		workspace: u32,
	},
	WindowMoved {
		id: vitrum_ipc::WindowId,
		from_workspace: u32,
		to_workspace: u32,
	},
	ThemeChanged {
		theme: ThemeSnapshot,
	},
	WallpaperChanged,
	Notification {
		app_name: String,
		summary: String,
		body: String,
		icon: Option<String>,
		timeout: Option<u32>,
	},
	Clipboard,
}

pub enum IpcMessage {
	Command {
		cmd: IpcCommand,
		reply: oneshot::Sender<IpcResponse>,
	},
}

#[derive(Clone)]
pub struct IpcEventEmitter {
	sender: Sender<IpcEvent>,
}

impl IpcEventEmitter {
	pub fn emit(&self, event: IpcEvent) {
		let _ = self.sender.send(event);
	}
}

pub fn start_ipc_server() -> anyhow::Result<(Receiver<IpcMessage>, IpcEventEmitter)> {
	let (cmd_tx, cmd_rx) = channel::<IpcMessage>();
	let (event_tx, event_rx) = channel::<IpcEvent>();

	std::thread::spawn(move || {
		let rt = tokio::runtime::Runtime::new().expect("Failed to create tokio runtime");
		rt.block_on(async {
			if let Err(e) = run_ipc_server(cmd_tx, event_rx).await {
				error!(error = %e, "IPC server error");
			}
		});
	});

	info!("IPC server started on separate thread");

	Ok((cmd_rx, IpcEventEmitter { sender: event_tx }))
}

pub fn process_ipc_messages(receiver: &Receiver<IpcMessage>, state: &mut State) {
	while let Ok(msg) = receiver.try_recv() {
		match msg {
			IpcMessage::Command { cmd, reply } => {
				let response = handle_command(cmd, state);
				let _ = reply.send(response);
			}
		}
	}
}

pub fn build_theme_snapshot(config: &vitrum_config::Config) -> ThemeSnapshot {
	let t = &config.theme;
	let f = &config.fonts;
	ThemeSnapshot {
		accent: t.accent.clone(),
		background: t.background.clone(),
		surface: t.surface.clone(),
		surface_raised: t.surface_raised.clone(),
		text: t.text.clone(),
		text_muted: t.text_muted.clone(),
		border: t.border.clone(),
		error: t.error.clone(),
		warning: t.warning.clone(),
		success: t.success.clone(),
		border_width: t.border_width,
		gaps_inner: config.layout.gaps_inner,
		gaps_outer: config.layout.gaps_outer,
		corner_radius: t.corner_radius,
		cursor_theme: t.cursor_theme.clone(),
		cursor_size: t.cursor_size,
		icon_theme: t.icon_theme.clone(),
		color_scheme: t.color_scheme.clone(),
		font_ui: f.ui.clone(),
		font_ui_size: f.ui_size,
		font_mono: f.mono.clone(),
		font_mono_size: f.mono_size,
		dpi: t.dpi,
		gdk_scale: t.gdk_scale,
	}
}

fn handle_command(cmd: IpcCommand, state: &mut State) -> IpcResponse {
	info!(command = ?cmd, "Received IPC command");

	match cmd {
		IpcCommand::FocusWindow { id } => {
			state.focus_window(id);
			IpcResponse::Ok
		}
		IpcCommand::MoveToWorkspace { window, workspace } => {
			state.move_window_to_workspace(window, workspace);
			IpcResponse::Ok
		}
		IpcCommand::SetLayout { workspace, mode } => {
			let layout_mode = match mode {
				vitrum_ipc::LayoutMode::Dwindle => crate::layout::LayoutMode::Dwindle,
				vitrum_ipc::LayoutMode::MasterStack => crate::layout::LayoutMode::MasterStack,
				vitrum_ipc::LayoutMode::Floating => crate::layout::LayoutMode::Floating,
			};
			state.layout_engine.set_workspace_layout(workspace, layout_mode);
			state.apply_layout();
			IpcResponse::Ok
		}
		IpcCommand::FloatWindow { id, floating } => {
			if let Some(window) = state.windows.get_mut(&id) {
				window.flags.floating = floating;
				state.apply_layout();
			}
			IpcResponse::Ok
		}
		IpcCommand::KillWindow { id } => {
			state.kill_window(id);
			IpcResponse::Ok
		}
		IpcCommand::GetWindows => {
			let mut windows: Vec<WindowInfo> = state
				.windows
				.iter()
				.map(|(id, window)| WindowInfo {
					id: *id,
					title: window.title.clone(),
					app_id: window.app_id.clone(),
					workspace: window.workspace,
					floating: window.flags.floating,
					fullscreen: window.flags.fullscreen,
					opacity: window.opacity,
					pinned: window.flags.pinned,
					urgent: window.flags.urgent,
				})
				.collect();

			windows.sort_by_key(|w| w.id.0);

			IpcResponse::Windows { windows }
		}
		IpcCommand::GetWorkspaces => {
			let mut workspaces: Vec<WorkspaceInfo> = state
				.workspaces
				.all()
				.values()
				.map(|ws| WorkspaceInfo {
					id: ws.id,
					name: ws.name.clone(),
					window_count: ws.window_count(),
					active: ws.id == state.active_workspace_id(),
				})
				.collect();

			workspaces.sort_by_key(|w| w.id);

			IpcResponse::Workspaces { workspaces }
		}
		IpcCommand::GetOutputs => {
			let mut outputs: Vec<OutputInfo> = state
				.output_manager
				.map()
				.outputs()
				.values()
				.map(|output| OutputInfo {
					id: output.id.0,
					name: output.name.clone(),
					size: (output.size.w, output.size.h),
					position: (output.position.x, output.position.y),
					scale: output.scale,
					enabled: output.enabled,
					primary: output.primary,
					active_workspace: output.active_workspace,
					transform: format!("{:?}", output.transform),
				})
				.collect();

			outputs.sort_by(|a, b| a.position.1.cmp(&b.position.1).then_with(|| a.position.0.cmp(&b.position.0)));

			IpcResponse::Outputs { outputs }
		}
		IpcCommand::GetLayouts => {
			let mut layouts: Vec<WorkspaceLayoutInfo> = state
				.workspaces
				.all()
				.values()
				.map(|ws| {
					let mode = state.layout_engine.get_workspace_layout(ws.id);
					let mode = match mode {
						crate::layout::LayoutMode::Dwindle => vitrum_ipc::LayoutMode::Dwindle,
						crate::layout::LayoutMode::MasterStack => vitrum_ipc::LayoutMode::MasterStack,
						crate::layout::LayoutMode::Floating => vitrum_ipc::LayoutMode::Floating,
					};

					WorkspaceLayoutInfo {
						workspace: ws.id,
						active: ws.id == state.active_workspace_id(),
						mode,
					}
				})
				.collect();

			layouts.sort_by_key(|layout| layout.workspace);

			IpcResponse::Layouts { layouts }
		}
		IpcCommand::GetState => {
			let mut windows: Vec<WindowInfo> = state
				.windows
				.iter()
				.map(|(id, window)| WindowInfo {
					id: *id,
					title: window.title.clone(),
					app_id: window.app_id.clone(),
					workspace: window.workspace,
					floating: window.flags.floating,
					fullscreen: window.flags.fullscreen,
					opacity: window.opacity,
					pinned: window.flags.pinned,
					urgent: window.flags.urgent,
				})
				.collect();

			windows.sort_by_key(|w| w.id.0);

			let mut workspaces: Vec<WorkspaceInfo> = state
				.workspaces
				.all()
				.values()
				.map(|ws| WorkspaceInfo {
					id: ws.id,
					name: ws.name.clone(),
					window_count: ws.window_count(),
					active: ws.id == state.active_workspace_id(),
				})
				.collect();

			workspaces.sort_by_key(|w| w.id);

			let focused_window = state
				.focused_surface
				.as_ref()
				.and_then(|surface| state.window_for_surface(surface));

			IpcResponse::State {
				state: CompositorState {
					focused_window,
					active_workspace: state.active_workspace_id(),
					windows,
					workspaces,
					theme: build_theme_snapshot(&state.config),
				},
			}
		}
		IpcCommand::ReloadConfig => {
			if let Err(e) = crate::config::reload_runtime_config(state) {
				error!(error = %e, "Failed to reload config via IPC");
				return IpcResponse::Error {
					msg: format!("reload config failed: {e}"),
				};
			}
			IpcResponse::Ok
		}
		IpcCommand::SwitchWorkspace { id } => {
			state.switch_workspace(id);
			IpcResponse::Ok
		}
		IpcCommand::NextWorkspace => {
			let next = if state.active_workspace_id() >= 10 {
				1
			} else {
				state.active_workspace_id() + 1
			};
			state.switch_workspace(next);
			IpcResponse::Ok
		}
		IpcCommand::PrevWorkspace => {
			let prev = if state.active_workspace_id() <= 1 {
				10
			} else {
				state.active_workspace_id() - 1
			};
			state.switch_workspace(prev);
			IpcResponse::Ok
		}
		IpcCommand::ToggleFullscreen { id } => {
			state.toggle_fullscreen(id);
			IpcResponse::Ok
		}
		IpcCommand::FocusDirection { dir } => {
			if !state.focus_direction(dir) {
				warn!("Directional focus requested but no window was focusable");
			}
			IpcResponse::Ok
		}
		IpcCommand::Spawn { cmd } => {
			if let Err(e) = crate::launcher::spawn_command(&cmd, "ipc") {
				error!(error = %e, command = %cmd, "Failed to spawn command from IPC");
				return IpcResponse::Error {
					msg: format!("spawn failed: {e}"),
				};
			}
			IpcResponse::Ok
		}
		IpcCommand::WallpaperSet { path, fit } => {
			let wallpaper_fit = match fit {
				WallpaperFit::Fill => vitrum_config::WallpaperConfig {
					mode: "image".to_string(),
					path: Some(path),
					fit: "fill".to_string(),
					..Default::default()
				},
				WallpaperFit::Fit => vitrum_config::WallpaperConfig {
					mode: "image".to_string(),
					path: Some(path),
					fit: "fit".to_string(),
					..Default::default()
				},
				WallpaperFit::Center => vitrum_config::WallpaperConfig {
					mode: "image".to_string(),
					path: Some(path),
					fit: "center".to_string(),
					..Default::default()
				},
				WallpaperFit::Tile => vitrum_config::WallpaperConfig {
					mode: "image".to_string(),
					path: Some(path),
					fit: "tile".to_string(),
					..Default::default()
				},
			};
			state.wallpaper.update_config(&wallpaper_fit);
			state.config.wallpaper = wallpaper_fit;
			state.needs_redraw = true;
			info!("Wallpaper set via IPC");
			IpcResponse::Ok
		}
		IpcCommand::WallpaperColor { color } => {
			let wallpaper_config = vitrum_config::WallpaperConfig {
				mode: "solid".to_string(),
				color: color,
				..Default::default()
			};
			state.wallpaper.update_config(&wallpaper_config);
			state.config.wallpaper = wallpaper_config;
			state.needs_redraw = true;
			info!("Wallpaper color set via IPC");
			IpcResponse::Ok
		}
		IpcCommand::WallpaperSlideshow { dir, interval_secs } => {
			let wallpaper_config = vitrum_config::WallpaperConfig {
				mode: "slideshow".to_string(),
				dir: Some(dir),
				interval: Some(interval_secs),
				..Default::default()
			};
			state.wallpaper.update_config(&wallpaper_config);
			state.config.wallpaper = wallpaper_config;
			state.needs_redraw = true;
			info!("Wallpaper slideshow set via IPC");
			IpcResponse::Ok
		}
		IpcCommand::SetTheme { patch } => {
			apply_theme_patch(patch, state);
			IpcResponse::Ok
		}
		IpcCommand::GetTheme => IpcResponse::Theme {
			theme: build_theme_snapshot(&state.config),
		},
		IpcCommand::GetEnvironment => {
			let env = vitrum_ipc::EnvironmentSnapshot {
				wayland_display: std::env::var("WAYLAND_DISPLAY").ok(),
				xwayland_display: std::env::var("DISPLAY").ok(),
				env_vars: vitrum_theme::ThemeState::build_env(&state.config).into_iter().collect(),
			};
			IpcResponse::Environment { env }
		}
		IpcCommand::GetActiveWindow => {
			let window = state
				.focused_surface
				.as_ref()
				.and_then(|surface| state.window_for_surface(surface))
				.and_then(|id| state.windows.get(&id))
				.map(|wd| WindowInfo {
					id: wd.id,
					title: wd.title.clone(),
					app_id: wd.app_id.clone(),
					workspace: wd.workspace,
					floating: wd.flags.floating,
					fullscreen: wd.flags.fullscreen,
					opacity: wd.opacity,
					pinned: wd.flags.pinned,
					urgent: wd.flags.urgent,
				});
			IpcResponse::ActiveWindow { window }
		}
		IpcCommand::SwapWindow { a, b } => {
			state.swap_windows(a, b);
			IpcResponse::Ok
		}
		IpcCommand::SetOpacity { id, value } => {
			if let Some(wd) = state.windows.get_mut(&id) {
				wd.opacity = value.clamp(0.0, 1.0);
				info!(id = ?id, opacity = wd.opacity, "Window opacity changed");
				state.mark_redraw();
			}
			IpcResponse::Ok
		}
		IpcCommand::ResizeDelta { id, dw, dh } => {
			if let Some(wd) = state.windows.get_mut(&id) {
				wd.resize_delta.0 += dw;
				wd.resize_delta.1 += dh;
				info!(id = ?id, delta = ?wd.resize_delta, "Window resize delta applied");
				state.apply_layout();
			}
			IpcResponse::Ok
		}
		IpcCommand::Lock => {
			let lock_cmd = &state.config.session.lock_command;
			info!(command = %lock_cmd, "Lock command received, spawning lock client");
			if let Err(e) = crate::launcher::spawn_command(lock_cmd, "ipc-lock") {
				error!(error = %e, command = %lock_cmd, "Failed to spawn lock client");
				return IpcResponse::Error {
					msg: format!("lock spawn failed: {e}"),
				};
			}
			IpcResponse::Ok
		}
		IpcCommand::Osd { icon, value, text } => {
			let icon_str = match icon {
				vitrum_ipc::OsdIcon::Volume => "volume",
				vitrum_ipc::OsdIcon::Brightness => "brightness",
				vitrum_ipc::OsdIcon::Mute => "mute",
				vitrum_ipc::OsdIcon::MicMute => "mic-mute",
				vitrum_ipc::OsdIcon::CapsLock => "capslock",
				vitrum_ipc::OsdIcon::Custom => "custom",
			};
			info!(icon = icon_str, value = ?value, text = ?text, "OSD event received");

			let mut cmd = format!("vitrum-osd --icon {}", icon_str);
			if let Some(v) = value {
				cmd.push_str(&format!(" --value {}", v));
			}
			if let Some(ref t) = text {
				cmd.push_str(&format!(" --text \"{}\"", t));
			}
			if let Err(e) = crate::launcher::spawn_command(&cmd, "ipc-osd") {
				warn!(error = %e, "Failed to spawn vitrum-osd");
			}
			IpcResponse::Ok
		}
		IpcCommand::Notify {
			app_name,
			summary,
			body,
			icon,
			timeout,
		} => {
			if let Some(emitter) = &state.ipc_event_emitter {
				emitter.emit(IpcEvent::Notification {
					app_name,
					summary,
					body,
					icon,
					timeout,
				});
			}
			IpcResponse::Ok
		}
		IpcCommand::ShowClipboard => {
			if let Some(emitter) = &state.ipc_event_emitter {
				emitter.emit(IpcEvent::Clipboard);
			}
			IpcResponse::Ok
		}
	}
}

async fn run_ipc_server(tx: Sender<IpcMessage>, event_rx: Receiver<IpcEvent>) -> anyhow::Result<()> {
	use vitrum_ipc::IpcServer;

	let command_socket_path = vitrum_ipc::command_socket_path();
	let event_socket_path = vitrum_ipc::event_socket_path();
	let mut server = IpcServer::new(&command_socket_path, &event_socket_path).await?;

	info!(
		command = ?command_socket_path,
		event = ?event_socket_path,
		"IPC server listening"
	);

	loop {
		let cmd_result = tokio::time::timeout(std::time::Duration::from_millis(10), server.accept_command()).await;

		if let Ok(Ok(mut conn)) = cmd_result {
			let tx = tx.clone();
			tokio::spawn(async move {
				loop {
					match conn.receive_frame().await {
						Ok((opcode, payload)) => {
							let cmd = match opcode {
								Opcode::FocusWindow => rmp_serde::from_slice::<vitrum_ipc::FocusWindow>(&payload)
									.ok()
									.map(|c| IpcCommand::FocusWindow { id: c.id }),
								Opcode::MoveToWorkspace => rmp_serde::from_slice::<vitrum_ipc::MoveToWorkspace>(&payload)
									.ok()
									.map(|c| IpcCommand::MoveToWorkspace {
										window: c.window,
										workspace: c.workspace,
									}),
								Opcode::SetLayout => {
									rmp_serde::from_slice::<vitrum_ipc::SetLayout>(&payload).ok().map(|c| {
										IpcCommand::SetLayout {
											workspace: c.workspace,
											mode: c.mode,
										}
									})
								}
								Opcode::FloatWindow => {
									rmp_serde::from_slice::<vitrum_ipc::FloatWindow>(&payload).ok().map(|c| {
										IpcCommand::FloatWindow {
											id: c.id,
											floating: c.floating,
										}
									})
								}
								Opcode::KillWindow => rmp_serde::from_slice::<vitrum_ipc::KillWindow>(&payload)
									.ok()
									.map(|c| IpcCommand::KillWindow { id: c.id }),
								Opcode::GetWindows => Some(IpcCommand::GetWindows),
								Opcode::GetWorkspaces => Some(IpcCommand::GetWorkspaces),
								Opcode::GetOutputs => Some(IpcCommand::GetOutputs),
								Opcode::GetLayouts => Some(IpcCommand::GetLayouts),
								Opcode::GetState => Some(IpcCommand::GetState),
								Opcode::ReloadConfig => Some(IpcCommand::ReloadConfig),
								Opcode::SwitchWorkspace => rmp_serde::from_slice::<vitrum_ipc::SwitchWorkspace>(&payload)
									.ok()
									.map(|c| IpcCommand::SwitchWorkspace { id: c.id }),
								Opcode::NextWorkspace => Some(IpcCommand::NextWorkspace),
								Opcode::PrevWorkspace => Some(IpcCommand::PrevWorkspace),
								Opcode::ToggleFullscreen => rmp_serde::from_slice::<vitrum_ipc::ToggleFullscreen>(&payload)
									.ok()
									.map(|c| IpcCommand::ToggleFullscreen { id: c.id }),
								Opcode::FocusDirection => rmp_serde::from_slice::<vitrum_ipc::FocusDirection>(&payload)
									.ok()
									.map(|c| IpcCommand::FocusDirection { dir: c.dir }),
								Opcode::Spawn => rmp_serde::from_slice::<vitrum_ipc::Spawn>(&payload)
									.ok()
									.map(|c| IpcCommand::Spawn { cmd: c.cmd }),
								Opcode::SwapWindow => rmp_serde::from_slice::<vitrum_ipc::SwapWindow>(&payload)
									.ok()
									.map(|c| IpcCommand::SwapWindow { a: c.a, b: c.b }),
								Opcode::SetOpacity => {
									rmp_serde::from_slice::<vitrum_ipc::SetOpacity>(&payload).ok().map(|c| {
										IpcCommand::SetOpacity {
											id: c.id,
											value: c.value,
										}
									})
								}
								Opcode::ResizeDelta => {
									rmp_serde::from_slice::<vitrum_ipc::ResizeDelta>(&payload).ok().map(|c| {
										IpcCommand::ResizeDelta {
											id: c.id,
											dw: c.dw,
											dh: c.dh,
										}
									})
								}
								Opcode::Lock => Some(IpcCommand::Lock),
								Opcode::WallpaperSet => rmp_serde::from_slice::<vitrum_ipc::WallpaperSet>(&payload)
									.ok()
									.map(|c| IpcCommand::WallpaperSet {
										path: c.path,
										fit: c.fit,
									}),
								Opcode::WallpaperColor => rmp_serde::from_slice::<vitrum_ipc::WallpaperColor>(&payload)
									.ok()
									.map(|c| IpcCommand::WallpaperColor { color: c.color }),
								Opcode::WallpaperSlideshow => {
									rmp_serde::from_slice::<vitrum_ipc::WallpaperSlideshow>(&payload)
										.ok()
										.map(|c| IpcCommand::WallpaperSlideshow {
											dir: c.dir,
											interval_secs: c.interval_secs,
										})
								}
								Opcode::SetTheme => rmp_serde::from_slice::<vitrum_ipc::SetTheme>(&payload)
									.ok()
									.map(|c| IpcCommand::SetTheme { patch: c.patch }),
								Opcode::Osd => {
									rmp_serde::from_slice::<vitrum_ipc::Osd>(&payload)
										.ok()
										.map(|c| IpcCommand::Osd {
											icon: c.icon,
											value: c.value,
											text: c.text,
										})
								}
								Opcode::GetTheme => Some(IpcCommand::GetTheme),
								Opcode::GetEnvironment => Some(IpcCommand::GetEnvironment),
								Opcode::GetActiveWindow => Some(IpcCommand::GetActiveWindow),
								Opcode::Notify => rmp_serde::from_slice::<vitrum_ipc::IpcNotify>(&payload).ok().map(|c| {
									IpcCommand::Notify {
										app_name: c.app_name,
										summary: c.summary,
										body: c.body,
										icon: c.icon,
										timeout: c.timeout,
									}
								}),
								Opcode::ShowClipboard => Some(IpcCommand::ShowClipboard),
								_ => None,
							};

							let cmd = match cmd {
								Some(cmd) => cmd,
								None => {
									warn!(opcode = ?opcode, "Unknown or invalid IPC command");
									continue;
								}
							};

							let (reply_tx, reply_rx) = oneshot::channel();

							if let Err(e) = tx.send(IpcMessage::Command { cmd, reply: reply_tx }) {
								error!(error = %e, "Failed to send command to compositor");
								break;
							}

							let response = match reply_rx.await {
								Ok(response) => response,
								Err(e) => {
									error!(error = %e, "Failed to receive IPC response from compositor");
									IpcResponse::Error {
										msg: "compositor failed to process command".to_string(),
									}
								}
							};

							let (res_opcode, res_payload) = match response {
								IpcResponse::Ok => (Opcode::ResponseOk, rmp_serde::to_vec_named(&()).unwrap()),
								IpcResponse::Error { msg } => (
									Opcode::ResponseError,
									rmp_serde::to_vec_named(&vitrum_ipc::ErrorResponse { msg }).unwrap(),
								),
								IpcResponse::Windows { windows } => (
									Opcode::ResponseWindows,
									rmp_serde::to_vec_named(&vitrum_ipc::WindowsResponse { windows }).unwrap(),
								),
								IpcResponse::Workspaces { workspaces } => (
									Opcode::ResponseWorkspaces,
									rmp_serde::to_vec_named(&vitrum_ipc::WorkspacesResponse { workspaces }).unwrap(),
								),
								IpcResponse::Outputs { outputs } => (
									Opcode::ResponseOutputs,
									rmp_serde::to_vec_named(&vitrum_ipc::OutputsResponse { outputs }).unwrap(),
								),
								IpcResponse::Layouts { layouts } => (
									Opcode::ResponseLayouts,
									rmp_serde::to_vec_named(&vitrum_ipc::LayoutsResponse { layouts }).unwrap(),
								),
								IpcResponse::State { state } => (
									Opcode::ResponseState,
									rmp_serde::to_vec_named(&vitrum_ipc::StateResponse { state }).unwrap(),
								),
								IpcResponse::Theme { theme } => (
									Opcode::ResponseTheme,
									rmp_serde::to_vec_named(&vitrum_ipc::ThemeResponse { theme }).unwrap(),
								),
								IpcResponse::Environment { env } => (
									Opcode::ResponseEnvironment,
									rmp_serde::to_vec_named(&vitrum_ipc::EnvironmentResponse { env }).unwrap(),
								),
								IpcResponse::ActiveWindow { window } => (
									Opcode::ResponseActiveWindow,
									rmp_serde::to_vec_named(&vitrum_ipc::ActiveWindowResponse { window }).unwrap(),
								),
							};

							if let Err(e) = conn.send_response(res_opcode, &res_payload).await {
								error!(error = %e, "Failed to send response");
								break;
							}
						}
						Err(e) => {
							warn!(error = %e, "Failed to receive command, closing connection");
							break;
						}
					}
				}
			});
		}

		loop {
			match tokio::time::timeout(std::time::Duration::ZERO, server.accept_event_listener()).await {
				Ok(Ok(stream)) => {
					server.register_event_listener(stream, IpcEventMask::ALL);
					info!("New event listener connected (total: {})", server.event_listener_count());
				}
				_ => break,
			}
		}

		loop {
			match event_rx.try_recv() {
				Ok(event) => {
					let (mask, opcode, payload) = match event {
						IpcEvent::WorkspaceChanged { workspace } => (
							IpcEventMask::WORKSPACE,
							Opcode::EventWorkspaceChanged,
							rmp_serde::to_vec_named(&vitrum_ipc::WorkspaceChanged { workspace }).unwrap(),
						),
						IpcEvent::WindowOpened { window } => (
							IpcEventMask::WINDOW,
							Opcode::EventWindowOpened,
							rmp_serde::to_vec_named(&vitrum_ipc::WindowOpened { window }).unwrap(),
						),
						IpcEvent::WindowClosed { id } => (
							IpcEventMask::WINDOW,
							Opcode::EventWindowClosed,
							rmp_serde::to_vec_named(&vitrum_ipc::WindowClosed { id }).unwrap(),
						),
						IpcEvent::WindowFocused { id } => (
							IpcEventMask::WINDOW,
							Opcode::EventWindowFocused,
							rmp_serde::to_vec_named(&vitrum_ipc::WindowFocused { id }).unwrap(),
						),
						IpcEvent::WindowMoved {
							id,
							from_workspace,
							to_workspace,
						} => (
							IpcEventMask::WINDOW,
							Opcode::EventWindowMoved,
							rmp_serde::to_vec_named(&vitrum_ipc::WindowMoved {
								id,
								from_workspace,
								to_workspace,
							})
							.unwrap(),
						),
						IpcEvent::ThemeChanged { theme } => (
							IpcEventMask::THEME,
							Opcode::EventThemeChanged,
							rmp_serde::to_vec_named(&vitrum_ipc::ThemeChanged { theme }).unwrap(),
						),
						IpcEvent::WallpaperChanged => (
							IpcEventMask::WALLPAPER,
							Opcode::EventWallpaperChanged,
							rmp_serde::to_vec_named(&()).unwrap(),
						),
						IpcEvent::Notification {
							app_name,
							summary,
							body,
							icon,
							timeout,
						} => (
							IpcEventMask::NOTIFICATION,
							Opcode::EventNotification,
							rmp_serde::to_vec_named(&vitrum_ipc::IpcEventNotification {
								app_name,
								summary,
								body,
								icon,
								timeout,
							})
							.unwrap(),
						),
						IpcEvent::Clipboard => (IpcEventMask::CLIPBOARD, Opcode::EventShowClipboard, rmp_serde::to_vec_named(&()).unwrap()),
					};

					if let Err(e) = server.broadcast_event(mask, opcode, &payload).await {
						warn!(error = %e, "Failed to broadcast event");
					}
				}
				Err(std::sync::mpsc::TryRecvError::Empty) => break,
				Err(std::sync::mpsc::TryRecvError::Disconnected) => {
					warn!("Event channel disconnected");
					return Ok(());
				}
			}
		}
	}
}

fn apply_theme_patch(patch: vitrum_ipc::ThemePatch, state: &mut State) {
	let t = &mut state.config.theme;
	if let Some(v) = patch.accent {
		t.accent = v;
	}
	if let Some(v) = patch.background {
		t.background = v;
	}
	if let Some(v) = patch.surface {
		t.surface = v;
	}
	if let Some(v) = patch.surface_raised {
		t.surface_raised = v;
	}
	if let Some(v) = patch.text {
		t.text = v;
	}
	if let Some(v) = patch.text_muted {
		t.text_muted = v;
	}
	if let Some(v) = patch.border {
		t.border = v;
	}
	if let Some(v) = patch.error {
		t.error = v;
	}
	if let Some(v) = patch.warning {
		t.warning = v;
	}
	if let Some(v) = patch.success {
		t.success = v;
	}
	if let Some(v) = patch.border_width {
		t.border_width = v;
	}
	if let Some(v) = patch.corner_radius {
		t.corner_radius = v;
	}
	if let Some(v) = patch.cursor_theme {
		t.cursor_theme = v;
	}
	if let Some(v) = patch.cursor_size {
		t.cursor_size = v;
	}
	if let Some(v) = patch.icon_theme {
		t.icon_theme = v;
	}
	if let Some(v) = patch.color_scheme {
		t.color_scheme = v;
	}
	if let Some(v) = patch.sound_theme {
		t.sound_theme = v;
	}
	if let Some(v) = patch.dpi {
		t.dpi = v;
	}
	if let Some(v) = patch.gdk_scale {
		t.gdk_scale = v;
	}

	if let Some(v) = patch.gaps_inner {
		state.config.layout.gaps_inner = v;
	}
	if let Some(v) = patch.gaps_outer {
		state.config.layout.gaps_outer = v;
	}

	let f = &mut state.config.fonts;
	if let Some(v) = patch.font_ui {
		f.ui = v;
	}
	if let Some(v) = patch.font_ui_size {
		f.ui_size = v;
	}
	if let Some(v) = patch.font_mono {
		f.mono = v;
	}
	if let Some(v) = patch.font_mono_size {
		f.mono_size = v;
	}
	if let Some(v) = patch.font_document {
		f.document = v;
	}
	if let Some(v) = patch.font_document_size {
		f.document_size = v;
	}

	if let Err(e) = vitrum_theme::ThemeState::default().apply(&state.config) {
		error!(error = %e, "Failed to apply theme patch");
	}

	state.session_env.update_config(&state.config);

	state
		.layout_engine
		.set_gaps(state.config.layout.gaps_inner as i32, state.config.layout.gaps_outer as i32);
	state.apply_layout();
	state.needs_redraw = true;

	info!("Theme patch applied");
}
