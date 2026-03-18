use serde::{Deserialize, Serialize};

pub mod client;
pub mod framing;
pub mod protocol;
pub mod server;

pub use client::{IpcClient, IpcEventSubscriber};
pub use protocol::Opcode;
pub use protocol::commands::*;
pub use protocol::events::*;
pub use protocol::responses::*;
pub use server::{CommandConnection, IpcServer};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WindowId(pub u64);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowInfo {
	pub id: WindowId,
	pub title: String,
	pub app_id: String,
	pub workspace: u32,
	pub floating: bool,
	pub fullscreen: bool,
	pub opacity: f32,
	pub pinned: bool,
	pub urgent: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputInfo {
	pub id: u64,
	pub name: String,
	pub size: (i32, i32),
	pub position: (i32, i32),
	pub scale: f64,
	pub enabled: bool,
	pub primary: bool,
	pub active_workspace: u32,
	pub transform: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceLayoutInfo {
	pub workspace: u32,
	pub active: bool,
	pub mode: LayoutMode,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum OsdIcon {
	Volume,
	Brightness,
	Mute,
	MicMute,
	CapsLock,
	Custom,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LayoutMode {
	Dwindle,
	MasterStack,
	Floating,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Direction {
	Up,
	Down,
	Left,
	Right,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceInfo {
	pub id: u32,
	pub name: String,
	pub window_count: usize,
	pub active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeSnapshot {
	pub accent: String,
	pub background: String,
	pub surface: String,
	pub surface_raised: String,
	pub text: String,
	pub text_muted: String,
	pub border: String,
	pub error: String,
	pub warning: String,
	pub success: String,

	pub border_width: u32,
	pub gaps_inner: u32,
	pub gaps_outer: u32,
	pub corner_radius: u32,

	pub cursor_theme: String,
	pub cursor_size: u32,

	pub icon_theme: String,

	pub color_scheme: String,

	pub font_ui: String,
	pub font_ui_size: u32,
	pub font_mono: String,
	pub font_mono_size: u32,

	pub dpi: u32,
	pub gdk_scale: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompositorState {
	pub focused_window: Option<WindowId>,
	pub active_workspace: u32,
	pub windows: Vec<WindowInfo>,
	pub workspaces: Vec<WorkspaceInfo>,
	pub theme: ThemeSnapshot,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvironmentSnapshot {
	pub wayland_display: Option<String>,
	pub xwayland_display: Option<String>,
	pub env_vars: Vec<(String, String)>,
}

pub fn command_socket_path() -> std::path::PathBuf {
	dirs::runtime_dir()
		.unwrap_or_else(|| std::path::PathBuf::from("/tmp"))
		.join("vitrum")
		.join("vitrum.sock")
}

pub fn event_socket_path() -> std::path::PathBuf {
	dirs::runtime_dir()
		.unwrap_or_else(|| std::path::PathBuf::from("/tmp"))
		.join("vitrum")
		.join("vitrum-events.sock")
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WallpaperFit {
	Fill,
	Fit,
	Center,
	Tile,
}

impl std::fmt::Display for WallpaperFit {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			WallpaperFit::Fill => write!(f, "fill"),
			WallpaperFit::Fit => write!(f, "fit"),
			WallpaperFit::Center => write!(f, "center"),
			WallpaperFit::Tile => write!(f, "tile"),
		}
	}
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ThemePatch {
	pub accent: Option<String>,
	pub background: Option<String>,
	pub surface: Option<String>,
	pub surface_raised: Option<String>,
	pub text: Option<String>,
	pub text_muted: Option<String>,
	pub border: Option<String>,
	pub error: Option<String>,
	pub warning: Option<String>,
	pub success: Option<String>,
	pub border_width: Option<u32>,
	pub gaps_inner: Option<u32>,
	pub gaps_outer: Option<u32>,
	pub corner_radius: Option<u32>,
	pub cursor_theme: Option<String>,
	pub cursor_size: Option<u32>,
	pub icon_theme: Option<String>,
	pub color_scheme: Option<String>,
	pub sound_theme: Option<String>,
	pub font_ui: Option<String>,
	pub font_ui_size: Option<u32>,
	pub font_mono: Option<String>,
	pub font_mono_size: Option<u32>,
	pub font_document: Option<String>,
	pub font_document_size: Option<u32>,
	pub dpi: Option<u32>,
	pub gdk_scale: Option<u32>,
}
