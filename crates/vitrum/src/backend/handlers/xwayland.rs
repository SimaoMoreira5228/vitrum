use smithay::desktop::Window;
use smithay::reexports::wayland_server::protocol::wl_surface::WlSurface;
use smithay::utils::Rectangle;
use smithay::wayland::xwayland_shell::{XWaylandShellHandler, XWaylandShellState};
use smithay::xwayland::xwm::{X11Surface, X11Wm, XwmHandler, XwmId};
use tracing::{info, warn};

use crate::backend::State;

impl XWaylandShellHandler for State {
	fn xwayland_shell_state(&mut self) -> &mut XWaylandShellState {
		&mut self.xwayland_shell_state
	}

	fn surface_associated(&mut self, _xwm_id: XwmId, _surface: WlSurface, _window: X11Surface) {
		info!("X11 window associated with Wayland surface");
		self.mark_redraw();
	}
}

impl XwmHandler for State {
	fn xwm_state(&mut self, _xwm: XwmId) -> &mut X11Wm {
		self.xwm
			.as_mut()
			.expect("XWayland XWM not initialized - xwm_state called before start_wm")
	}

	fn new_window(&mut self, _xwm: XwmId, window: X11Surface) {
		info!("New X11 window created");

		let _ = window;
	}

	fn new_override_redirect_window(&mut self, _xwm: XwmId, window: X11Surface) {
		let geometry = window.geometry();

		if let Some(wl_surface) = window.wl_surface() {
			let x11_window = Window::new_x11_window(window.clone());
			let window_id = self.add_window_from_x11(x11_window, wl_surface, geometry);

			if let Some(wd) = self.windows.get_mut(&window_id) {
				wd.geometry = geometry;
				wd.flags.floating = true;
			}

			info!(id = ?window_id, "X11 override-redirect window tracked");
		} else {
			info!("X11 override-redirect window: no wl_surface yet, will track on map");
		}
		self.mark_redraw();
	}

	fn map_window_request(&mut self, _xwm: XwmId, window: X11Surface) {
		info!("X11 window map requested");

		if let Err(err) = window.set_mapped(true) {
			warn!(?err, "Failed to set X11 window as mapped");
			return;
		}

		let geometry = window.geometry();

		if let Err(err) = window.configure(geometry) {
			warn!(?err, "Failed to configure X11 window");
		}

		if let Some(wl_surface) = window.wl_surface() {
			let x11_window = Window::new_x11_window(window.clone());
			let window_id = self.add_window_from_x11(x11_window, wl_surface, geometry);

			if let Some(wd) = self.windows.get_mut(&window_id) {
				wd.geometry = geometry;
			}

			info!(id = ?window_id, "X11 window mapped");
		} else {
			info!("X11 window map deferred - no wl_surface yet");
		}

		self.mark_redraw();
	}

	fn map_window_notify(&mut self, _xwm: XwmId, _window: X11Surface) {
		self.mark_redraw();
	}

	fn mapped_override_redirect_window(&mut self, _xwm: XwmId, window: X11Surface) {
		let already_tracked = self.windows.iter().any(|(_, wd)| {
			wd.window
				.x11_surface()
				.map(|xs| xs.window_id() == window.window_id())
				.unwrap_or(false)
		});

		if !already_tracked {
			let geometry = window.geometry();
			if let Some(wl_surface) = window.wl_surface() {
				let x11_window = Window::new_x11_window(window.clone());
				let window_id = self.add_window_from_x11(x11_window, wl_surface, geometry);

				if let Some(wd) = self.windows.get_mut(&window_id) {
					wd.geometry = geometry;
					wd.flags.floating = true;
				}

				info!(id = ?window_id, "X11 override-redirect window tracked on map");
			}
		}
		self.mark_redraw();
	}

	fn unmapped_window(&mut self, _xwm: XwmId, window: X11Surface) {
		info!("X11 window unmapped");
		let window_id_to_remove = self
			.windows
			.iter()
			.find(|(_, wd)| {
				wd.window
					.x11_surface()
					.map(|xs| xs.window_id() == window.window_id())
					.unwrap_or(false)
			})
			.map(|(id, _)| *id);

		if let Some(id) = window_id_to_remove {
			self.remove_window(id);
		}
		self.mark_redraw();
	}

	fn destroyed_window(&mut self, _xwm: XwmId, window: X11Surface) {
		info!("X11 window destroyed");
		let window_id_to_remove = self
			.windows
			.iter()
			.find(|(_, wd)| {
				wd.window
					.x11_surface()
					.map(|xs| xs.window_id() == window.window_id())
					.unwrap_or(false)
			})
			.map(|(id, _)| *id);

		if let Some(id) = window_id_to_remove {
			self.remove_window(id);
		}
		self.mark_redraw();
	}

	fn configure_request(
		&mut self,
		_xwm: XwmId,
		window: X11Surface,
		x: Option<i32>,
		y: Option<i32>,
		w: Option<u32>,
		h: Option<u32>,
		_reorder: Option<smithay::xwayland::xwm::Reorder>,
	) {
		let mut geometry = window.geometry();
		if let Some(x) = x {
			geometry.loc.x = x;
		}
		if let Some(y) = y {
			geometry.loc.y = y;
		}
		if let Some(w) = w {
			geometry.size.w = w as i32;
		}
		if let Some(h) = h {
			geometry.size.h = h as i32;
		}

		if let Err(err) = window.configure(geometry) {
			warn!(?err, "Failed to configure X11 window on request");
		}
		self.mark_redraw();
	}

	fn configure_notify(
		&mut self,
		_xwm: XwmId,
		_window: X11Surface,
		_geometry: Rectangle<i32, smithay::utils::Logical>,
		_above: Option<u32>,
	) {
		self.mark_redraw();
	}

	fn resize_request(
		&mut self,
		_xwm: XwmId,
		_window: X11Surface,
		_button: u32,
		_resize_edge: smithay::xwayland::xwm::ResizeEdge,
	) {
		self.mark_redraw();
	}

	fn move_request(&mut self, _xwm: XwmId, _window: X11Surface, _button: u32) {
		self.mark_redraw();
	}
}
