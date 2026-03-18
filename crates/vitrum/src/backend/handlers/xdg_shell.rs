use smithay::desktop::Window;
use smithay::utils::Serial;
use smithay::wayland::compositor::with_states;
use smithay::wayland::shell::xdg::XdgToplevelSurfaceRoleAttributes;
use smithay::wayland::shell::xdg::{ToplevelSurface, XdgShellState};
use tracing::{debug, info, warn};

use crate::backend::State;

impl smithay::wayland::shell::xdg::XdgShellHandler for State {
	fn xdg_shell_state(&mut self) -> &mut XdgShellState {
		&mut self.xdg_shell_state
	}

	fn new_toplevel(&mut self, surface: ToplevelSurface) {
		let window = Window::new_wayland_window(surface.clone());
		let wl_surface = surface.wl_surface().clone();

		let (title, app_id): (Option<String>, Option<String>) = with_states(&wl_surface, |states| {
			let role = states.data_map.get::<XdgToplevelSurfaceRoleAttributes>();
			if let Some(role) = role {
				(role.title.clone(), role.app_id.clone())
			} else {
				(None, None)
			}
		});

		let app_id_str = app_id.clone().unwrap_or_default();
		let title_str = title.clone().unwrap_or_default();

		let matched_rules = crate::config::match_window_rules(&self.config, &app_id_str, &title_str);

		let window_id = self.add_window(window, wl_surface, surface.clone());

		if let Some(window_data) = self.windows.get_mut(&window_id) {
			window_data.title = title.unwrap_or_default();
			window_data.app_id = app_id.unwrap_or_default();
		}
		self.sync_foreign_toplevel_metadata(window_id);

		if let Some((workspace, floating, pin)) = matched_rules {
			if let Some(ws) = workspace {
				if ws >= 1 && ws <= 10 {
					debug!(id = ?window_id, target_ws = ws, "Window rule: move to workspace");
					self.move_window_to_workspace(window_id, ws);
				}
			}
			if let Some(is_floating) = floating {
				if is_floating {
					if let Some(window_data) = self.windows.get_mut(&window_id) {
						window_data.flags.floating = true;
						let floating_geo = self.layout_engine.default_floating_geometry(self.output_size);
						window_data.set_geometry(floating_geo);
						debug!(id = ?window_id, "Window rule: set floating");
					}
				}
			}
			if let Some(is_pinned) = pin {
				if is_pinned {
					if let Some(window_data) = self.windows.get_mut(&window_id) {
						window_data.flags.pinned = true;
						debug!(id = ?window_id, "Window rule: set pinned");
					}
				}
			}
		}

		let configure_serial = if let Some(window_data) = self.windows.get(&window_id) {
			let size = window_data.geometry.size;
			surface.with_pending_state(|state| {
				state.size = Some(size);
			});
			let serial = surface.send_configure();
			debug!(?serial, w = size.w, h = size.h, "Initial configure sent");
			serial
		} else {
			let serial = surface.send_configure();
			debug!(?serial, "Initial configure sent (no geometry)");
			serial
		};

		info!(
			id = ?window_id,
			app_id = %app_id_str,
			title = %title_str,
			workspace = self.active_workspace_id(),
			configure_serial = ?configure_serial,
			"XDG toplevel created"
		);
		self.mark_redraw();
	}

	fn new_popup(
		&mut self,
		surface: smithay::wayland::shell::xdg::PopupSurface,
		positioner: smithay::wayland::shell::xdg::PositionerState,
	) {
		let geometry = positioner.get_geometry();

		surface.with_pending_state(|state| {
			state.geometry = geometry;
			state.positioner = positioner;
		});

		if let Err(err) = surface.send_configure() {
			warn!(?err, "Failed to configure popup");
		}

		debug!(
			x = geometry.loc.x,
			y = geometry.loc.y,
			w = geometry.size.w,
			h = geometry.size.h,
			"Popup configured"
		);
		self.mark_redraw();
	}

	fn grab(
		&mut self,
		_surface: smithay::wayland::shell::xdg::PopupSurface,
		_seat: smithay::reexports::wayland_server::protocol::wl_seat::WlSeat,
		_serial: Serial,
	) {
		debug!("Popup grab requested");
		self.mark_redraw();
	}

	fn reposition_request(
		&mut self,
		surface: smithay::wayland::shell::xdg::PopupSurface,
		positioner: smithay::wayland::shell::xdg::PositionerState,
		_token: u32,
	) {
		let geometry = positioner.get_geometry();

		surface.with_pending_state(|state| {
			state.geometry = geometry;
			state.positioner = positioner;
		});

		if let Err(err) = surface.send_configure() {
			warn!(?err, "Failed to configure popup on reposition");
		}

		debug!(x = geometry.loc.x, y = geometry.loc.y, "Popup repositioned");
		self.mark_redraw();
	}
}
