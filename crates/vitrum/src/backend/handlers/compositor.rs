use smithay::backend::renderer::utils::on_commit_buffer_handler;
use smithay::reexports::wayland_server::Client;
use smithay::reexports::wayland_server::protocol::wl_surface::WlSurface;
use smithay::wayland::compositor::{CompositorClientState, get_parent, is_sync_subsurface};
use smithay::wayland::shell::xdg::XdgToplevelSurfaceRoleAttributes;
use std::sync::OnceLock;
use tracing::{debug, trace, warn};

use crate::backend::State;

impl smithay::wayland::compositor::CompositorHandler for State {
	fn compositor_state(&mut self) -> &mut smithay::wayland::compositor::CompositorState {
		&mut self.compositor_state
	}

	fn commit(&mut self, surface: &WlSurface) {
		on_commit_buffer_handler::<Self>(surface);

		if crate::session_lock::handle_lock_surface_commit(self, surface) {
			trace!("Commit handled by session lock surface");
			return;
		}

		if crate::layer_shell::handle_layer_surface_commit(self, surface) {
			trace!("Commit handled by layer-shell surface");
			return;
		}

		if !is_sync_subsurface(surface) {
			let mut root = surface.clone();
			while let Some(parent) = get_parent(&root) {
				root = parent;
			}

			if let Some(window_id) = self.window_for_surface(&root) {
				if let Some(window_data) = self.windows.get(&window_id) {
					window_data.window.on_commit();
				}
			}
		}

		if let Some(window_id) = self.window_for_surface(surface) {
			if let Some(window) = self.windows.get_mut(&window_id) {
				let (title, app_id) = smithay::wayland::compositor::with_states(surface, |states| {
					states
						.data_map
						.get::<XdgToplevelSurfaceRoleAttributes>()
						.map(|role| (role.title.clone(), role.app_id.clone()))
						.unwrap_or((None, None))
				});

				let mut metadata_changed = false;
				if let Some(ref new_title) = title {
					if window.title != *new_title {
						debug!(id = ?window_id, old = %window.title, new = %new_title, "Window title changed");
						window.title = new_title.clone();
						metadata_changed = true;
					}
				}

				if let Some(ref new_app_id) = app_id {
					if window.app_id != *new_app_id {
						debug!(id = ?window_id, old = %window.app_id, new = %new_app_id, "Window app_id changed");
						window.app_id = new_app_id.clone();
						metadata_changed = true;
					}
				}

				trace!(
					id = ?window_id,
					workspace = window.workspace,
					title = %window.title,
					app_id = %window.app_id,
					"Window surface committed"
				);

				if metadata_changed {
					self.sync_foreign_toplevel_metadata(window_id);
				}
			} else {
				trace!("Commit on surface with no window data");
			}
		} else {
			trace!("Commit on unmapped/unknown surface");
		}
		self.mark_redraw();
	}

	fn client_compositor_state<'a>(&self, client: &'a Client) -> &'a CompositorClientState {
		if let Some(client_state) = client.get_data::<super::ClientState>() {
			return &client_state.compositor_state;
		}

		static FALLBACK_CLIENT_STATE: OnceLock<CompositorClientState> = OnceLock::new();
		static MISSING_CLIENT_STATE_WARNED: OnceLock<()> = OnceLock::new();
		if MISSING_CLIENT_STATE_WARNED.set(()).is_ok() {
			warn!("ClientState not found on Wayland client; using fallback compositor client state");
		}
		FALLBACK_CLIENT_STATE.get_or_init(CompositorClientState::default)
	}
}
