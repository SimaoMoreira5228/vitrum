#![allow(dead_code)]

use smithay::desktop::Window;
use smithay::reexports::wayland_server::Resource;
use smithay::reexports::wayland_server::protocol::wl_surface::WlSurface;
use smithay::utils::{IsAlive, Logical, Rectangle};
pub use vitrum_ipc::WindowId;

pub fn new_window_id() -> WindowId {
	use std::sync::atomic::{AtomicU64, Ordering};
	static COUNTER: AtomicU64 = AtomicU64::new(1);
	WindowId(COUNTER.fetch_add(1, Ordering::SeqCst))
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct WindowFlags {
	pub floating: bool,
	pub fullscreen: bool,
	pub pinned: bool,
	pub urgent: bool,
}

#[derive(Debug, Clone)]
pub struct WindowData {
	pub id: WindowId,
	pub window: Window,
	pub surface: WlSurface,
	pub toplevel: Option<smithay::wayland::shell::xdg::ToplevelSurface>,
	pub flags: WindowFlags,
	pub geometry: Rectangle<i32, Logical>,
	pub workspace: u32,
	pub title: String,
	pub app_id: String,
	pub opacity: f32,
	pub resize_delta: (i32, i32),
}

impl WindowData {
	pub fn new(window: Window, surface: WlSurface, toplevel: smithay::wayland::shell::xdg::ToplevelSurface) -> Self {
		Self {
			id: new_window_id(),
			window,
			surface,
			toplevel: Some(toplevel),
			flags: WindowFlags::default(),
			geometry: Rectangle::default(),
			workspace: 1,
			title: String::new(),
			app_id: String::new(),
			opacity: 1.0,
			resize_delta: (0, 0),
		}
	}

	pub fn new_x11(window: Window, surface: WlSurface) -> Self {
		Self {
			id: new_window_id(),
			window,
			surface,
			toplevel: None,
			flags: WindowFlags::default(),
			geometry: Rectangle::default(),
			workspace: 1,
			title: String::new(),
			app_id: String::new(),
			opacity: 1.0,
			resize_delta: (0, 0),
		}
	}

	pub fn is_alive(&self) -> bool {
		self.window.alive() && self.surface.is_alive()
	}

	pub fn set_geometry(&mut self, geometry: Rectangle<i32, Logical>) -> Rectangle<i32, Logical> {
		let old = self.geometry;
		self.geometry = geometry;
		old
	}
}
