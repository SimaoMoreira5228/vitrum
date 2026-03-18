use std::path::PathBuf;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

pub mod script;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
	#[serde(default)]
	pub session: SessionConfig,

	#[serde(default)]
	pub input: InputConfig,

	#[serde(default)]
	pub layout: LayoutConfig,

	#[serde(default)]
	pub theme: ThemeConfig,

	#[serde(default)]
	pub fonts: FontsConfig,

	#[serde(default)]
	pub locale: LocaleConfig,

	#[serde(default)]
	pub keybind: Vec<Keybind>,

	#[serde(default)]
	pub window_rule: Vec<WindowRule>,

	#[serde(default)]
	pub wallpaper: WallpaperConfig,

	#[serde(default)]
	pub autostart: Vec<AutostartEntry>,

	#[serde(default)]
	pub keyring: KeyringConfig,

	#[serde(default)]
	pub notifications: NotificationsConfig,

	#[serde(default)]
	pub disable_xdg_autostart: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionConfig {
	#[serde(default = "default_terminal")]
	pub default_terminal: String,
	#[serde(default = "default_lock_command")]
	pub lock_command: String,
}

impl Default for SessionConfig {
	fn default() -> Self {
		Self {
			default_terminal: default_terminal(),
			lock_command: default_lock_command(),
		}
	}
}

fn default_terminal() -> String {
	"foot".to_string()
}

fn default_lock_command() -> String {
	"vitrum-lock".to_string()
}

impl Config {
	pub fn load() -> Result<Self> {
		let config_dir = dirs::config_dir()
			.ok_or_else(|| anyhow::anyhow!("Could not determine config directory"))?
			.join("vitrum");

		let vt_path = config_dir.join("vitrum.vt");
		let toml_path = config_dir.join("vitrum.toml");

		if vt_path.exists() {
			Self::load_from(&vt_path)
		} else if toml_path.exists() {
			Self::load_from(&toml_path)
		} else {
			let config = Config::default();
			config.save_to(&toml_path)?;
			Ok(config)
		}
	}

	pub fn load_from(path: &std::path::Path) -> Result<Self> {
		let contents =
			std::fs::read_to_string(path).with_context(|| format!("Failed to read config from {}", path.display()))?;

		let is_vt = path
			.extension()
			.and_then(|e| e.to_str())
			.map(|e| e == "vt" || e == "vitrum")
			.unwrap_or(false);

		if is_vt {
			Self::load_from_vt(&contents, path)
		} else {
			let config: Config =
				toml::from_str(&contents).with_context(|| format!("Failed to parse config from {}", path.display()))?;
			Ok(config)
		}
	}

	fn load_from_vt(source: &str, path: &std::path::Path) -> Result<Self> {
		use serde::Deserialize;
		let acc = script::eval_file(path)
			.map_err(|e| anyhow::anyhow!("{}\n{}", e, script::render_error(&e, &path.display().to_string(), source)))?;

		let config = Config::deserialize(script::AccumulatorDeserializer::new(&acc))
			.with_context(|| format!("Failed to deserialize .vt config from {}", path.display()))?;

		Ok(config)
	}

	pub fn save(&self) -> Result<()> {
		let path = config_path()?;
		self.save_to(&path)
	}

	pub fn save_to(&self, path: &std::path::Path) -> Result<()> {
		if let Some(parent) = path.parent() {
			std::fs::create_dir_all(parent)?;
		}

		let contents = toml::to_string_pretty(self).context("Failed to serialize config")?;

		std::fs::write(path, contents).with_context(|| format!("Failed to write config to {}", path.display()))?;

		Ok(())
	}
}

impl Default for Config {
	fn default() -> Self {
		Self {
			session: SessionConfig::default(),
			input: InputConfig::default(),
			layout: LayoutConfig::default(),
			theme: ThemeConfig::default(),
			fonts: FontsConfig::default(),
			locale: LocaleConfig::default(),
			keybind: default_keybinds(),
			window_rule: Vec::new(),
			wallpaper: WallpaperConfig::default(),
			autostart: vec![
				AutostartEntry {
					cmd: "vitrum-clip".to_string(),
					args: Vec::new(),
				},
				AutostartEntry {
					cmd: "vitrum-bar".to_string(),
					args: Vec::new(),
				},
				AutostartEntry {
					cmd: "vitrum-keyring".to_string(),
					args: vec!["bootstrap".to_string()],
				},
			],
			keyring: KeyringConfig::default(),
			notifications: NotificationsConfig::default(),
			disable_xdg_autostart: false,
		}
	}
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputConfig {
	#[serde(default = "default_repeat_delay")]
	pub repeat_delay: u32,
	#[serde(default = "default_repeat_rate")]
	pub repeat_rate: u32,
	#[serde(default)]
	pub natural_scroll: bool,
}

impl Default for InputConfig {
	fn default() -> Self {
		Self {
			repeat_delay: default_repeat_delay(),
			repeat_rate: default_repeat_rate(),
			natural_scroll: false,
		}
	}
}

fn default_repeat_delay() -> u32 {
	300
}

fn default_repeat_rate() -> u32 {
	50
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayoutConfig {
	#[serde(default = "default_layout_mode")]
	pub default_mode: String,
	#[serde(default = "default_gaps_inner")]
	pub gaps_inner: u32,
	#[serde(default = "default_gaps_outer")]
	pub gaps_outer: u32,
	#[serde(default = "default_master_ratio")]
	pub master_ratio: f32,
}

impl Default for LayoutConfig {
	fn default() -> Self {
		Self {
			default_mode: default_layout_mode(),
			gaps_inner: default_gaps_inner(),
			gaps_outer: default_gaps_outer(),
			master_ratio: default_master_ratio(),
		}
	}
}

fn default_layout_mode() -> String {
	"dwindle".to_string()
}

fn default_gaps_inner() -> u32 {
	8
}

fn default_gaps_outer() -> u32 {
	12
}

fn default_master_ratio() -> f32 {
	0.5
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeConfig {
	#[serde(default = "default_accent")]
	pub accent: String,
	#[serde(default = "default_background")]
	pub background: String,
	#[serde(default = "default_surface")]
	pub surface: String,
	#[serde(default = "default_surface_raised")]
	pub surface_raised: String,
	#[serde(default = "default_text")]
	pub text: String,
	#[serde(default = "default_text_muted")]
	pub text_muted: String,
	#[serde(default = "default_border")]
	pub border: String,
	#[serde(default = "default_error")]
	pub error: String,
	#[serde(default = "default_warning")]
	pub warning: String,
	#[serde(default = "default_success")]
	pub success: String,

	#[serde(default = "default_border_width")]
	pub border_width: u32,
	#[serde(default = "default_gaps_inner")]
	pub gaps_inner: u32,
	#[serde(default = "default_gaps_outer")]
	pub gaps_outer: u32,
	#[serde(default = "default_corner_radius")]
	pub corner_radius: u32,

	#[serde(default = "default_cursor_theme")]
	pub cursor_theme: String,
	#[serde(default = "default_cursor_size")]
	pub cursor_size: u32,

	#[serde(default = "default_icon_theme")]
	pub icon_theme: String,

	#[serde(default = "default_color_scheme")]
	pub color_scheme: String,

	#[serde(default = "default_sound_theme")]
	pub sound_theme: String,

	#[serde(default = "default_dpi")]
	pub dpi: u32,
	#[serde(default = "default_gdk_scale")]
	pub gdk_scale: u32,
	#[serde(default = "default_gdk_dpi_scale")]
	pub gdk_dpi_scale: f32,
	#[serde(default = "default_qt_scale_factor")]
	pub qt_scale_factor: f32,
}

impl Default for ThemeConfig {
	fn default() -> Self {
		Self {
			accent: default_accent(),
			background: default_background(),
			surface: default_surface(),
			surface_raised: default_surface_raised(),
			text: default_text(),
			text_muted: default_text_muted(),
			border: default_border(),
			error: default_error(),
			warning: default_warning(),
			success: default_success(),
			border_width: default_border_width(),
			gaps_inner: default_gaps_inner_theme(),
			gaps_outer: default_gaps_outer_theme(),
			corner_radius: default_corner_radius(),
			cursor_theme: default_cursor_theme(),
			cursor_size: default_cursor_size(),
			icon_theme: default_icon_theme(),
			color_scheme: default_color_scheme(),
			sound_theme: default_sound_theme(),
			dpi: default_dpi(),
			gdk_scale: default_gdk_scale(),
			gdk_dpi_scale: default_gdk_dpi_scale(),
			qt_scale_factor: default_qt_scale_factor(),
		}
	}
}

fn default_accent() -> String {
	"#1A6B8A".to_string()
}
fn default_background() -> String {
	"#1C1C2E".to_string()
}
fn default_surface() -> String {
	"#2A2A3E".to_string()
}
fn default_surface_raised() -> String {
	"#323248".to_string()
}
fn default_text() -> String {
	"#E0E0F0".to_string()
}
fn default_text_muted() -> String {
	"#8888AA".to_string()
}
fn default_border() -> String {
	"#3A3A5C".to_string()
}
fn default_error() -> String {
	"#F38BA8".to_string()
}
fn default_warning() -> String {
	"#FAB387".to_string()
}
fn default_success() -> String {
	"#A6E3A1".to_string()
}
fn default_border_width() -> u32 {
	2
}
fn default_gaps_inner_theme() -> u32 {
	8
}
fn default_gaps_outer_theme() -> u32 {
	12
}
fn default_corner_radius() -> u32 {
	0
}
fn default_cursor_theme() -> String {
	"default".to_string()
}
fn default_cursor_size() -> u32 {
	24
}
fn default_icon_theme() -> String {
	"hicolor".to_string()
}
fn default_color_scheme() -> String {
	"dark".to_string()
}
fn default_sound_theme() -> String {
	"freedesktop".to_string()
}
fn default_dpi() -> u32 {
	96
}
fn default_gdk_scale() -> u32 {
	1
}
fn default_gdk_dpi_scale() -> f32 {
	1.0
}
fn default_qt_scale_factor() -> f32 {
	1.0
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Keybind {
	pub mods: Vec<String>,
	pub key: String,
	pub action: Action,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Action {
	#[serde(rename = "spawn")]
	Spawn {
		cmd: String,
	},
	#[serde(rename = "kill_focused")]
	KillFocused,
	#[serde(rename = "focus_direction")]
	FocusDirection {
		dir: String,
	},
	#[serde(rename = "move_to_workspace")]
	MoveToWorkspace {
		workspace: u32,
	},
	#[serde(rename = "switch_workspace")]
	SwitchWorkspace {
		workspace: u32,
	},
	#[serde(rename = "next_workspace")]
	NextWorkspace,
	#[serde(rename = "prev_workspace")]
	PrevWorkspace,
	#[serde(rename = "toggle_float")]
	ToggleFloat,
	#[serde(rename = "toggle_fullscreen")]
	ToggleFullscreen,
	#[serde(rename = "toggle_pin")]
	TogglePin,
	#[serde(rename = "set_layout")]
	SetLayout {
		mode: String,
	},
	#[serde(rename = "reload_config")]
	ReloadConfig,
	#[serde(rename = "quit")]
	Quit,
	#[serde(rename = "swap_window")]
	SwapWindow,
	#[serde(rename = "set_opacity")]
	SetOpacity {
		value: f32,
	},
	#[serde(rename = "resize_delta")]
	ResizeDelta {
		dw: i32,
		dh: i32,
	},
	#[serde(rename = "lock")]
	Lock,
	#[serde(rename = "dispatch")]
	Dispatch {
		cmd: String,
	},
}

fn default_keybinds() -> Vec<Keybind> {
	vec![
		Keybind {
			mods: vec!["super".to_string()],
			key: "Return".to_string(),
			action: Action::Spawn { cmd: "foot".to_string() },
		},
		Keybind {
			mods: vec!["super".to_string(), "shift".to_string()],
			key: "q".to_string(),
			action: Action::KillFocused,
		},
		Keybind {
			mods: vec!["super".to_string()],
			key: "h".to_string(),
			action: Action::FocusDirection { dir: "left".to_string() },
		},
		Keybind {
			mods: vec!["super".to_string()],
			key: "j".to_string(),
			action: Action::FocusDirection { dir: "down".to_string() },
		},
		Keybind {
			mods: vec!["super".to_string()],
			key: "k".to_string(),
			action: Action::FocusDirection { dir: "up".to_string() },
		},
		Keybind {
			mods: vec!["super".to_string()],
			key: "l".to_string(),
			action: Action::FocusDirection {
				dir: "right".to_string(),
			},
		},
		Keybind {
			mods: vec!["super".to_string()],
			key: "1".to_string(),
			action: Action::SwitchWorkspace { workspace: 1 },
		},
		Keybind {
			mods: vec!["super".to_string()],
			key: "2".to_string(),
			action: Action::SwitchWorkspace { workspace: 2 },
		},
		Keybind {
			mods: vec!["super".to_string()],
			key: "3".to_string(),
			action: Action::SwitchWorkspace { workspace: 3 },
		},
		Keybind {
			mods: vec!["super".to_string()],
			key: "f".to_string(),
			action: Action::ToggleFullscreen,
		},
		Keybind {
			mods: vec!["super".to_string()],
			key: "space".to_string(),
			action: Action::ToggleFloat,
		},
		Keybind {
			mods: vec!["super".to_string()],
			key: "p".to_string(),
			action: Action::TogglePin,
		},
		Keybind {
			mods: vec!["super".to_string()],
			key: "w".to_string(),
			action: Action::NextWorkspace,
		},
		Keybind {
			mods: vec!["super".to_string()],
			key: "q".to_string(),
			action: Action::PrevWorkspace,
		},
		Keybind {
			mods: vec!["super".to_string(), "shift".to_string()],
			key: "r".to_string(),
			action: Action::ReloadConfig,
		},
		Keybind {
			mods: vec!["super".to_string()],
			key: "l".to_string(),
			action: Action::Lock,
		},
	]
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowRule {
	#[serde(default)]
	pub match_class: Option<String>,
	#[serde(default)]
	pub match_title: Option<String>,
	#[serde(default)]
	pub workspace: Option<u32>,
	#[serde(default)]
	pub floating: Option<bool>,
	#[serde(default)]
	pub pin: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FontsConfig {
	#[serde(default = "default_font_ui")]
	pub ui: String,
	#[serde(default = "default_font_ui_size")]
	pub ui_size: u32,
	#[serde(default = "default_font_mono")]
	pub mono: String,
	#[serde(default = "default_font_mono_size")]
	pub mono_size: u32,
	#[serde(default = "default_font_document")]
	pub document: String,
	#[serde(default = "default_font_document_size")]
	pub document_size: u32,
	#[serde(default = "default_font_rendering")]
	pub rendering: String,
	#[serde(default = "default_font_hinting")]
	pub hinting: String,
	#[serde(default)]
	pub extra_dirs: Vec<String>,
}

impl Default for FontsConfig {
	fn default() -> Self {
		Self {
			ui: default_font_ui(),
			ui_size: default_font_ui_size(),
			mono: default_font_mono(),
			mono_size: default_font_mono_size(),
			document: default_font_document(),
			document_size: default_font_document_size(),
			rendering: default_font_rendering(),
			hinting: default_font_hinting(),
			extra_dirs: Vec::new(),
		}
	}
}

fn default_font_ui() -> String {
	"sans-serif".to_string()
}
fn default_font_ui_size() -> u32 {
	11
}
fn default_font_mono() -> String {
	"monospace".to_string()
}
fn default_font_mono_size() -> u32 {
	10
}
fn default_font_document() -> String {
	"serif".to_string()
}
fn default_font_document_size() -> u32 {
	11
}
fn default_font_rendering() -> String {
	"subpixel".to_string()
}
fn default_font_hinting() -> String {
	"slight".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocaleConfig {
	#[serde(default = "default_locale_lang")]
	pub lang: String,
	#[serde(default)]
	pub lc_time: Option<String>,
	#[serde(default)]
	pub lc_numeric: Option<String>,
	#[serde(default)]
	pub lc_monetary: Option<String>,
	#[serde(default)]
	pub lc_paper: Option<String>,
	#[serde(default)]
	pub lc_measurement: Option<String>,
	#[serde(default)]
	pub lc_collate: Option<String>,
}

impl Default for LocaleConfig {
	fn default() -> Self {
		Self {
			lang: default_locale_lang(),
			lc_time: None,
			lc_numeric: None,
			lc_monetary: None,
			lc_paper: None,
			lc_measurement: None,
			lc_collate: None,
		}
	}
}

fn default_locale_lang() -> String {
	std::env::var("LANG").unwrap_or_else(|_| "en_US.UTF-8".to_string())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WallpaperConfig {
	#[serde(default = "default_wallpaper_mode")]
	pub mode: String,
	#[serde(default)]
	pub path: Option<String>,
	#[serde(default = "default_wallpaper_fit")]
	pub fit: String,
	#[serde(default = "default_wallpaper_color")]
	pub color: String,
	#[serde(default)]
	pub dir: Option<String>,
	#[serde(default)]
	pub interval: Option<u32>,
}

impl Default for WallpaperConfig {
	fn default() -> Self {
		Self {
			mode: default_wallpaper_mode(),
			path: None,
			fit: default_wallpaper_fit(),
			color: default_wallpaper_color(),
			dir: None,
			interval: None,
		}
	}
}

fn default_wallpaper_mode() -> String {
	"solid".to_string()
}

fn default_wallpaper_fit() -> String {
	"fill".to_string()
}

fn default_wallpaper_color() -> String {
	"#1C1C2E".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutostartEntry {
	pub cmd: String,
	#[serde(default)]
	pub args: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyringConfig {
	#[serde(default = "default_keyring_prefer")]
	pub prefer: String,
	#[serde(default)]
	pub cmd: Option<String>,
	#[serde(default = "default_keyring_components")]
	pub components: Vec<String>,
}

impl Default for KeyringConfig {
	fn default() -> Self {
		Self {
			prefer: default_keyring_prefer(),
			cmd: None,
			components: default_keyring_components(),
		}
	}
}

fn default_keyring_prefer() -> String {
	"auto".to_string()
}

fn default_keyring_components() -> Vec<String> {
	vec!["pkcs11".to_string(), "secrets".to_string(), "ssh".to_string()]
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationsConfig {
	#[serde(default = "default_notifications_position")]
	pub position: String,
	#[serde(default = "default_notifications_margin")]
	pub margin: u32,
	#[serde(default = "default_notifications_timeout")]
	pub timeout: u32,
	#[serde(default = "default_notifications_width")]
	pub width: u32,
	#[serde(default = "default_notifications_slide")]
	pub slide: bool,
}

impl Default for NotificationsConfig {
	fn default() -> Self {
		Self {
			position: default_notifications_position(),
			margin: default_notifications_margin(),
			timeout: default_notifications_timeout(),
			width: default_notifications_width(),
			slide: default_notifications_slide(),
		}
	}
}

fn default_notifications_position() -> String {
	"top-right".to_string()
}

fn default_notifications_margin() -> u32 {
	20
}

fn default_notifications_timeout() -> u32 {
	5000
}

fn default_notifications_width() -> u32 {
	340
}

fn default_notifications_slide() -> bool {
	true
}

pub fn config_path() -> Result<PathBuf> {
	let config_dir = dirs::config_dir()
		.context("Could not determine config directory")?
		.join("vitrum");

	let vt_path = config_dir.join("vitrum.vt");
	if vt_path.exists() {
		return Ok(vt_path);
	}

	Ok(config_dir.join("vitrum.toml"))
}

pub fn init_default_config() -> Result<()> {
	let path = config_path()?;

	if !path.exists() {
		let config = Config::default();
		config.save()?;
		tracing::info!(path = %path.display(), "Created default config file");
	}

	Ok(())
}
