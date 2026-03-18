use std::collections::HashMap;
use std::env;
use tracing::info;

use crate::config::Config;

const FORWARD_PARENT_VARS: &[&str] = &[
	"PATH",
	"HOME",
	"USER",
	"SHELL",
	"DBUS_SESSION_BUS_ADDRESS",
	"XDG_RUNTIME_DIR",
	"EDITOR",
	"TERMINAL",
	"BROWSER",
];

#[derive(Debug)]
pub struct SessionEnvironment {
	wayland_display: Option<String>,

	xwayland_display: Option<String>,

	extra_env: HashMap<String, String>,
}

impl SessionEnvironment {
	pub fn new(config: &Config) -> Self {
		let mut env = Self {
			wayland_display: None,
			xwayland_display: None,
			extra_env: HashMap::new(),
		};

		env.bootstrap(config);
		env
	}

	pub fn set_wayland_socket(&mut self, socket_name: &str) {
		self.wayland_display = Some(socket_name.to_string());
		unsafe {
			env::set_var("WAYLAND_DISPLAY", socket_name);
		}
		unsafe {
			env::set_var("XDG_SESSION_TYPE", "wayland");
		}
		unsafe {
			env::set_var("XDG_CURRENT_DESKTOP", "Vitrum");
		}
		info!(socket = %socket_name, "Session env: WAYLAND_DISPLAY set");
	}

	pub fn set_xwayland_display(&mut self, display_number: u32) {
		let x_display = format!(":{}", display_number);
		self.xwayland_display = Some(x_display.clone());
		unsafe {
			env::set_var("DISPLAY", &x_display);
		}
		info!(display = %x_display, "Session env: DISPLAY set");
	}

	pub fn update_config(&mut self, config: &Config) {
		let theme_env = vitrum_theme::ThemeState::build_env(config);
		for (key, val) in &theme_env {
			self.extra_env.insert(key.clone(), val.clone());
			unsafe {
				env::set_var(key, val);
			}
		}
		info!("Session environment updated from config");
	}

	pub fn child_env(&self) -> HashMap<String, String> {
		let mut env_map = HashMap::new();

		for key in FORWARD_PARENT_VARS {
			if let Ok(val) = env::var(key) {
				env_map.insert(key.to_string(), val);
			}
		}

		if !env_map.contains_key("XDG_DATA_DIRS") {
			env_map.insert("XDG_DATA_DIRS".into(), "/usr/local/share:/usr/share".into());
		}
		if !env_map.contains_key("XDG_CONFIG_DIRS") {
			env_map.insert("XDG_CONFIG_DIRS".into(), "/etc/xdg".into());
		}
		if !env_map.contains_key("XDG_DATA_HOME") {
			if let Some(home) = env::var("HOME").ok() {
				env_map.insert("XDG_DATA_HOME".into(), format!("{}/.local/share", home));
			}
		}
		if !env_map.contains_key("XDG_CONFIG_HOME") {
			if let Some(home) = env::var("HOME").ok() {
				env_map.insert("XDG_CONFIG_HOME".into(), format!("{}/.config", home));
			}
		}
		if !env_map.contains_key("XDG_CACHE_HOME") {
			if let Some(home) = env::var("HOME").ok() {
				env_map.insert("XDG_CACHE_HOME".into(), format!("{}/.cache", home));
			}
		}
		if !env_map.contains_key("XDG_STATE_HOME") {
			if let Some(home) = env::var("HOME").ok() {
				env_map.insert("XDG_STATE_HOME".into(), format!("{}/.local/state", home));
			}
		}

		if let Some(ref wd) = self.wayland_display {
			env_map.insert("WAYLAND_DISPLAY".into(), wd.clone());
		}
		env_map.insert("XDG_SESSION_TYPE".into(), "wayland".into());
		env_map.insert("XDG_CURRENT_DESKTOP".into(), "Vitrum".into());

		if let Some(ref xd) = self.xwayland_display {
			env_map.insert("DISPLAY".into(), xd.clone());
		}

		env_map.extend(self.extra_env.clone());

		env_map
	}

	fn bootstrap(&mut self, config: &Config) {
		let theme_env = vitrum_theme::ThemeState::build_env(config);
		self.extra_env = theme_env;

		for (key, val) in &self.extra_env {
			unsafe {
				env::set_var(key, val);
			}
		}

		info!(
			cursor = %config.theme.cursor_size,
			gtk_backend = "wayland,x11",
			qt_platform = "wayland;xcb",
			dpi = config.theme.dpi,
			"Session environment bootstrapped from config"
		);
	}
}

impl Default for SessionEnvironment {
	fn default() -> Self {
		Self::new(&Config::default())
	}
}
