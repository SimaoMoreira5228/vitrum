use serde::{Deserialize, Serialize};

use crate::{
	CompositorState, EnvironmentSnapshot, OutputInfo, ThemeSnapshot, WindowInfo, WorkspaceInfo, WorkspaceLayoutInfo,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorResponse {
	pub msg: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowsResponse {
	pub windows: Vec<WindowInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspacesResponse {
	pub workspaces: Vec<WorkspaceInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputsResponse {
	pub outputs: Vec<OutputInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayoutsResponse {
	pub layouts: Vec<WorkspaceLayoutInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateResponse {
	pub state: CompositorState,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeResponse {
	pub theme: ThemeSnapshot,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvironmentResponse {
	pub env: EnvironmentSnapshot,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActiveWindowResponse {
	pub window: Option<WindowInfo>,
}
