use serde::{Deserialize, Serialize};

pub mod commands;
pub mod events;
pub mod responses;

pub const IPC_MAGIC: [u8; 4] = *b"VITR";
pub const FRAME_HEADER_LEN: usize = 10;
pub const IPC_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u16)]
pub enum Opcode {
	Handshake = 0x0001,
	Subscribe = 0x0002,

	FocusWindow = 0x1001,
	MoveToWorkspace = 0x1002,
	SetLayout = 0x1003,
	FloatWindow = 0x1004,
	KillWindow = 0x1005,
	GetWindows = 0x1006,
	GetWorkspaces = 0x1007,
	GetOutputs = 0x1008,
	GetLayouts = 0x1009,
	GetState = 0x100A,
	ReloadConfig = 0x100B,
	SwitchWorkspace = 0x100C,
	NextWorkspace = 0x100D,
	PrevWorkspace = 0x100E,
	ToggleFullscreen = 0x100F,
	FocusDirection = 0x1010,
	Spawn = 0x1011,
	SwapWindow = 0x1012,
	SetOpacity = 0x1013,
	ResizeDelta = 0x1014,
	Lock = 0x1015,
	WallpaperSet = 0x1016,
	WallpaperColor = 0x1017,
	WallpaperSlideshow = 0x1018,
	SetTheme = 0x1019,
	Osd = 0x101A,
	GetTheme = 0x101B,
	GetEnvironment = 0x101C,
	GetActiveWindow = 0x101D,
	Notify = 0x101E,
	ShowClipboard = 0x101F,

	EventWindowOpened = 0x2001,
	EventWindowClosed = 0x2002,
	EventWindowFocused = 0x2003,
	EventWorkspaceChanged = 0x2004,
	EventWindowMoved = 0x2005,
	EventThemeChanged = 0x2006,
	EventWallpaperChanged = 0x2007,
	EventNotification = 0x2008,
	EventShowClipboard = 0x2009,

	ResponseOk = 0x3001,
	ResponseError = 0x3002,
	ResponseWindows = 0x3003,
	ResponseWorkspaces = 0x3004,
	ResponseOutputs = 0x3005,
	ResponseLayouts = 0x3006,
	ResponseState = 0x3007,
	ResponseTheme = 0x3008,
	ResponseEnvironment = 0x3009,
	ResponseActiveWindow = 0x300A,

	Unknown = 0xFFFF,
}

impl From<u16> for Opcode {
	fn from(val: u16) -> Self {
		match val {
			0x0001 => Opcode::Handshake,
			0x0002 => Opcode::Subscribe,

			0x1001 => Opcode::FocusWindow,
			0x1002 => Opcode::MoveToWorkspace,
			0x1003 => Opcode::SetLayout,
			0x1004 => Opcode::FloatWindow,
			0x1005 => Opcode::KillWindow,
			0x1006 => Opcode::GetWindows,
			0x1007 => Opcode::GetWorkspaces,
			0x1008 => Opcode::GetOutputs,
			0x1009 => Opcode::GetLayouts,
			0x100A => Opcode::GetState,
			0x100B => Opcode::ReloadConfig,
			0x100C => Opcode::SwitchWorkspace,
			0x100D => Opcode::NextWorkspace,
			0x100E => Opcode::PrevWorkspace,
			0x100F => Opcode::ToggleFullscreen,
			0x1010 => Opcode::FocusDirection,
			0x1011 => Opcode::Spawn,
			0x1012 => Opcode::SwapWindow,
			0x1013 => Opcode::SetOpacity,
			0x1014 => Opcode::ResizeDelta,
			0x1015 => Opcode::Lock,
			0x1016 => Opcode::WallpaperSet,
			0x1017 => Opcode::WallpaperColor,
			0x1018 => Opcode::WallpaperSlideshow,
			0x1019 => Opcode::SetTheme,
			0x101A => Opcode::Osd,
			0x101B => Opcode::GetTheme,
			0x101C => Opcode::GetEnvironment,
			0x101D => Opcode::GetActiveWindow,
			0x101E => Opcode::Notify,
			0x101F => Opcode::ShowClipboard,

			0x2001 => Opcode::EventWindowOpened,
			0x2002 => Opcode::EventWindowClosed,
			0x2003 => Opcode::EventWindowFocused,
			0x2004 => Opcode::EventWorkspaceChanged,
			0x2005 => Opcode::EventWindowMoved,
			0x2006 => Opcode::EventThemeChanged,
			0x2007 => Opcode::EventWallpaperChanged,
			0x2008 => Opcode::EventNotification,
			0x2009 => Opcode::EventShowClipboard,

			0x3001 => Opcode::ResponseOk,
			0x3002 => Opcode::ResponseError,
			0x3003 => Opcode::ResponseWindows,
			0x3004 => Opcode::ResponseWorkspaces,
			0x3005 => Opcode::ResponseOutputs,
			0x3006 => Opcode::ResponseLayouts,
			0x3007 => Opcode::ResponseState,
			0x3008 => Opcode::ResponseTheme,
			0x3009 => Opcode::ResponseEnvironment,
			0x300A => Opcode::ResponseActiveWindow,

			_ => Opcode::Unknown,
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_opcode_conversion() {
		assert_eq!(Opcode::from(0x0001), Opcode::Handshake);
		assert_eq!(Opcode::from(0x1001), Opcode::FocusWindow);
		assert_eq!(Opcode::from(0x2001), Opcode::EventWindowOpened);
		assert_eq!(Opcode::from(0x3001), Opcode::ResponseOk);
		assert_eq!(Opcode::from(0xFFFF), Opcode::Unknown);
		assert_eq!(Opcode::from(0x1234), Opcode::Unknown);
	}
}

bitflags::bitflags! {
	#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
	pub struct IpcEventMask: u32 {
		const WINDOW    = 1 << 0;
		const WORKSPACE = 1 << 1;
		const THEME     = 1 << 2;
		const WALLPAPER = 1 << 3;
		const NOTIFICATION = 1 << 4;
		const CLIPBOARD = 1 << 5;

		const ALL       = Self::WINDOW.bits() | Self::WORKSPACE.bits() | Self::THEME.bits() | Self::WALLPAPER.bits() | Self::NOTIFICATION.bits() | Self::CLIPBOARD.bits();
	}
}

pub const CAP_FD_PASSING: u64 = 1 << 0;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IpcHandshake {
	pub compositor_version: String,
	pub protocol_version: u32,
	pub capabilities: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IpcSubscribe {
	pub mask: IpcEventMask,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IpcError {
	pub msg: String,
}
