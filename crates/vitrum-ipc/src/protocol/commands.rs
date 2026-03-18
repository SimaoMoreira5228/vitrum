use serde::{Deserialize, Serialize};

use crate::{Direction, LayoutMode, ThemePatch, WallpaperFit, WindowId};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FocusWindow {
	pub id: WindowId,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MoveToWorkspace {
	pub window: WindowId,
	pub workspace: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetLayout {
	pub workspace: u32,
	pub mode: LayoutMode,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FloatWindow {
	pub id: WindowId,
	pub floating: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KillWindow {
	pub id: WindowId,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwitchWorkspace {
	pub id: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToggleFullscreen {
	pub id: WindowId,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FocusDirection {
	pub dir: Direction,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Spawn {
	pub cmd: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwapWindow {
	pub a: WindowId,
	pub b: WindowId,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetOpacity {
	pub id: WindowId,
	pub value: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResizeDelta {
	pub id: WindowId,
	pub dw: i32,
	pub dh: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WallpaperSet {
	pub path: String,
	pub fit: WallpaperFit,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WallpaperColor {
	pub color: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WallpaperSlideshow {
	pub dir: String,
	pub interval_secs: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetTheme {
	pub patch: ThemePatch,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Osd {
	pub icon: crate::OsdIcon,
	pub value: Option<u8>,
	pub text: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IpcNotify {
	pub app_name: String,
	pub summary: String,
	pub body: String,
	pub icon: Option<String>,
	pub timeout: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShowClipboard;
