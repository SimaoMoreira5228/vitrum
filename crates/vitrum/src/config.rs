use std::path::Path;
use std::sync::mpsc::{Sender, channel};

use anyhow::{Context, Result};
use notify::{Config as NotifyConfig, Event, RecommendedWatcher, RecursiveMode, Watcher};
use tracing::{error, info, warn};

use crate::ipc_handler::IpcEvent;
pub use vitrum_config::{Action, Config, config_path};

pub struct ConfigManager {
	_config: Config,
	_watcher: Option<RecommendedWatcher>,
	_sender: Sender<ConfigEvent>,
	_path: std::path::PathBuf,
}

#[derive(Debug, Clone)]
pub enum ConfigEvent {
	Reloaded,
	Error(String),
}

impl ConfigManager {
	pub fn new() -> Result<(
		Self,
		Config,
		std::sync::mpsc::Sender<ConfigEvent>,
		std::sync::mpsc::Receiver<ConfigEvent>,
	)> {
		vitrum_config::init_default_config().context("Failed to initialize default config")?;

		let path = config_path()?;
		let config = Config::load().context("Failed to load config")?;

		info!(path = %path.display(), "Config loaded successfully");

		let (sender, receiver) = channel::<ConfigEvent>();

		let watcher = Self::create_watcher(&path, sender.clone())?;

		let manager = Self {
			_config: config.clone(),
			_watcher: Some(watcher),
			_sender: sender.clone(),
			_path: path,
		};

		Ok((manager, config, sender, receiver))
	}

	fn create_watcher(path: &Path, sender: Sender<ConfigEvent>) -> Result<RecommendedWatcher> {
		let expected_path = path.to_path_buf();
		let mut watcher = RecommendedWatcher::new(
			move |res: Result<Event, notify::Error>| match res {
				Ok(event) => {
					if event.paths.iter().any(|p| p == &expected_path) {
						let _ = sender.send(ConfigEvent::Reloaded);
					}
				}
				Err(e) => {
					let _ = sender.send(ConfigEvent::Error(e.to_string()));
				}
			},
			NotifyConfig::default(),
		)
		.context("Failed to create file watcher")?;

		let parent_dir = path.parent().unwrap_or(path);
		watcher.watch(parent_dir, RecursiveMode::NonRecursive)?;

		info!("Config directory watcher started on {:?}", parent_dir);
		Ok(watcher)
	}

	pub fn _reload(&mut self) -> Result<Config> {
		let config = Config::load_from(&self._path).context("Failed to reload config")?;

		self._config = config.clone();
		info!("Config reloaded successfully");
		Ok(config)
	}

	pub fn _config(&self) -> &Config {
		&self._config
	}

	#[cfg(test)]
	pub fn new_test(sender: std::sync::mpsc::Sender<ConfigEvent>) -> Self {
		Self {
			_config: Config::default(),
			_watcher: None,
			_sender: sender,
			_path: std::path::PathBuf::from("/dev/null"),
		}
	}
}

pub fn apply_config(config: &Config, state: &mut crate::backend::State) {
	info!("Applying configuration");

	state
		.layout_engine
		.set_gaps(config.layout.gaps_inner as i32, config.layout.gaps_outer as i32);
	state.layout_engine.set_master_ratio(config.layout.master_ratio);

	let default_mode = match config.layout.default_mode.as_str() {
		"dwindle" => crate::layout::LayoutMode::Dwindle,
		"master" | "master-stack" | "master_stack" => crate::layout::LayoutMode::MasterStack,
		"floating" => crate::layout::LayoutMode::Floating,
		_ => {
			warn!(mode = %config.layout.default_mode, "Unknown layout mode, using Dwindle");
			crate::layout::LayoutMode::Dwindle
		}
	};

	for workspace_id in 1..=10 {
		state.layout_engine.set_workspace_layout(workspace_id, default_mode);
	}

	info!(
		layout = %config.layout.default_mode,
		gaps_inner = config.layout.gaps_inner,
		gaps_outer = config.layout.gaps_outer,
		master_ratio = config.layout.master_ratio,
		"Layout configuration applied"
	);

	state.wallpaper.update_config(&config.wallpaper);
	info!("Wallpaper configuration applied");

	state.keybind_manager.update_keybinds(config.keybind.clone());
	info!("Keybind configuration applied");

	let repeat_delay = i32::try_from(config.input.repeat_delay).unwrap_or(i32::MAX);
	let repeat_rate = i32::try_from(config.input.repeat_rate).unwrap_or(i32::MAX);
	state.keyboard.change_repeat_info(repeat_rate, repeat_delay);
	info!(repeat_delay, repeat_rate, "Keyboard repeat configuration applied");

	if let Err(e) = vitrum_theme::ThemeState::default().apply(config) {
		error!(error = %e, "Failed to apply theme propagation");
	} else {
		info!("Theme propagation applied");
	}

	state.session_env.update_config(config);

	let theme_snapshot = crate::ipc_handler::build_theme_snapshot(config);
	state.emit_ipc_event(IpcEvent::ThemeChanged { theme: theme_snapshot });
	state.emit_ipc_event(IpcEvent::WallpaperChanged);

	state.apply_layout();
}

pub fn reload_runtime_config(state: &mut crate::backend::State) -> Result<()> {
	let config = Config::load().context("Failed to load config for runtime reload")?;
	apply_config(&config, state);
	state.config = config;
	info!("Runtime config reloaded and applied");
	Ok(())
}

pub fn match_window_rules(config: &Config, app_id: &str, title: &str) -> Option<(Option<u32>, Option<bool>, Option<bool>)> {
	for rule in &config.window_rule {
		let matches_class = rule
			.match_class
			.as_ref()
			.map(|class| app_id.to_lowercase().contains(&class.to_lowercase()))
			.unwrap_or(false);

		let matches_title = rule
			.match_title
			.as_ref()
			.map(|t| title.to_lowercase().contains(&t.to_lowercase()))
			.unwrap_or(false);

		if matches_class || matches_title {
			return Some((rule.workspace, rule.floating, rule.pin));
		}
	}

	None
}

pub fn execute_action(action: &Action, state: &mut crate::backend::State) {
	use vitrum_config::Action;

	match action {
		Action::Spawn { cmd } => {
			info!(cmd = %cmd, "Spawning command from config");
			if let Err(e) = crate::launcher::spawn_command(cmd, "config") {
				error!(error = %e, cmd = %cmd, "Failed to spawn command from config");
			}
		}
		Action::KillFocused => {
			if let Some(ref focused) = state.focused_surface {
				if let Some(window_id) = state.window_for_surface(focused) {
					state.kill_window(window_id);
				}
			}
		}
		Action::FocusDirection { dir } => {
			let direction = match dir.to_lowercase().as_str() {
				"left" => vitrum_ipc::Direction::Left,
				"right" => vitrum_ipc::Direction::Right,
				"up" => vitrum_ipc::Direction::Up,
				"down" => vitrum_ipc::Direction::Down,
				_ => {
					warn!(dir = %dir, "Unknown focus direction");
					return;
				}
			};

			if !state.focus_direction(direction) {
				warn!(dir = %dir, "No focusable window for focus direction");
			}
		}
		Action::MoveToWorkspace { workspace } => {
			if let Some(ref focused) = state.focused_surface {
				if let Some(window_id) = state.window_for_surface(focused) {
					state.move_window_to_workspace(window_id, *workspace);
				}
			}
		}
		Action::SwitchWorkspace { workspace } => {
			state.switch_workspace(*workspace);
		}
		Action::NextWorkspace => {
			let next = if state.active_workspace_id() >= 10 {
				1
			} else {
				state.active_workspace_id() + 1
			};
			state.switch_workspace(next);
		}
		Action::PrevWorkspace => {
			let prev = if state.active_workspace_id() <= 1 {
				10
			} else {
				state.active_workspace_id() - 1
			};
			state.switch_workspace(prev);
		}
		Action::ToggleFloat => {
			if let Some(ref focused) = state.focused_surface {
				if let Some(window_id) = state.window_for_surface(focused) {
					state.toggle_floating(window_id);
				}
			}
		}
		Action::ToggleFullscreen => {
			if let Some(ref focused) = state.focused_surface {
				if let Some(window_id) = state.window_for_surface(focused) {
					state.toggle_fullscreen(window_id);
				}
			}
		}
		Action::TogglePin => {
			if let Some(ref focused) = state.focused_surface {
				if let Some(window_id) = state.window_for_surface(focused) {
					state.toggle_pinned(window_id);
				}
			}
		}
		Action::SetLayout { mode } => {
			let layout_mode = match mode.as_str() {
				"dwindle" => crate::layout::LayoutMode::Dwindle,
				"master" | "master-stack" | "master_stack" => crate::layout::LayoutMode::MasterStack,
				"floating" => crate::layout::LayoutMode::Floating,
				_ => {
					warn!(mode = %mode, "Unknown layout mode");
					return;
				}
			};
			state
				.layout_engine
				.set_workspace_layout(state.active_workspace_id(), layout_mode);
			state.apply_layout();
		}
		Action::ReloadConfig => {
			if let Err(e) = reload_runtime_config(state) {
				error!(error = %e, "Failed to reload config from action");
			}
		}
		Action::Quit => {
			info!("Quit action triggered");
			std::process::exit(0);
		}
		Action::SwapWindow => {
			if let Some(ref focused) = state.focused_surface {
				if let Some(focused_id) = state.window_for_surface(focused) {
					let focused_ws = state.windows.get(&focused_id).map(|w| w.workspace);
					if let Some(ws) = focused_ws {
						let next_id = state
							.window_order
							.iter()
							.filter(|id| **id != focused_id)
							.find(|id| state.windows.get(id).is_some_and(|w| w.workspace == ws))
							.copied();
						if let Some(next) = next_id {
							state.swap_windows(focused_id, next);
						}
					}
				}
			}
		}
		Action::SetOpacity { value } => {
			if let Some(ref focused) = state.focused_surface {
				if let Some(window_id) = state.window_for_surface(focused) {
					if let Some(wd) = state.windows.get_mut(&window_id) {
						wd.opacity = value.clamp(0.0, 1.0);
						state.mark_redraw();
					}
				}
			}
		}
		Action::ResizeDelta { dw, dh } => {
			if let Some(ref focused) = state.focused_surface {
				if let Some(window_id) = state.window_for_surface(focused) {
					if let Some(wd) = state.windows.get_mut(&window_id) {
						wd.resize_delta.0 += dw;
						wd.resize_delta.1 += dh;
						state.apply_layout();
					}
				}
			}
		}
		Action::Lock => {
			let lock_cmd = &state.config.session.lock_command;
			info!(command = %lock_cmd, "Lock action triggered");
			if let Err(e) = crate::launcher::spawn_command(lock_cmd, "keybind-lock") {
				warn!(error = %e, command = %lock_cmd, "Failed to spawn lock client");
			}
		}
		Action::Dispatch { cmd } => match cmd.as_str() {
			"lock" => {
				let lock_cmd = &state.config.session.lock_command;
				if let Err(e) = crate::launcher::spawn_command(lock_cmd, "keybind-dispatch") {
					warn!(error = %e, command = %lock_cmd, "Failed to spawn lock client");
				}
			}
			"reload-config" => {
				let _ = crate::config::reload_runtime_config(state);
			}
			_ => {
				info!(command = %cmd, "Dispatch action triggered");
				if let Err(e) = crate::launcher::spawn_command(cmd, "keybind-dispatch") {
					warn!(error = %e, command = %cmd, "Failed to spawn dispatch command");
				}
			}
		},
	}
}
