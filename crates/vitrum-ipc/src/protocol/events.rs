use serde::{Deserialize, Serialize};

use crate::{ThemeSnapshot, WindowId, WindowInfo};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowOpened {
	pub window: WindowInfo,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowClosed {
	pub id: WindowId,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowFocused {
	pub id: WindowId,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceChanged {
	pub workspace: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowMoved {
	pub id: WindowId,
	pub from_workspace: u32,
	pub to_workspace: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeChanged {
	pub theme: ThemeSnapshot,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IpcEventNotification {
	pub app_name: String,
	pub summary: String,
	pub body: String,
	pub icon: Option<String>,
	pub timeout: Option<u32>,
}
