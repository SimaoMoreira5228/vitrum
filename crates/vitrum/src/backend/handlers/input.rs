use smithay::reexports::wayland_server::protocol::wl_surface::WlSurface;

use crate::backend::State;

impl smithay::input::SeatHandler for State {
	type KeyboardFocus = WlSurface;
	type PointerFocus = WlSurface;
	type TouchFocus = WlSurface;

	fn seat_state(&mut self) -> &mut smithay::input::SeatState<Self> {
		&mut self.seat_state
	}

	fn focus_changed(&mut self, _seat: &smithay::input::Seat<Self>, _focused: Option<&WlSurface>) {
		if let Some(focused) = _focused {
			if let Some(window_id) = self.window_for_surface(focused) {
				self.sync_foreign_toplevel_metadata(window_id);
			}
		}

		self.mark_redraw();
	}

	fn cursor_image(&mut self, _seat: &smithay::input::Seat<Self>, _image: smithay::input::pointer::CursorImageStatus) {
		self.mark_redraw();
	}
}

impl smithay::wayland::shm::ShmHandler for State {
	fn shm_state(&self) -> &smithay::wayland::shm::ShmState {
		&self.shm_state
	}
}

impl smithay::wayland::buffer::BufferHandler for State {
	fn buffer_destroyed(&mut self, _buffer: &smithay::reexports::wayland_server::protocol::wl_buffer::WlBuffer) {
		self.mark_redraw();
	}
}

impl smithay::wayland::idle_notify::IdleNotifierHandler for State {
	fn idle_notifier_state(&mut self) -> &mut smithay::wayland::idle_notify::IdleNotifierState<Self> {
		&mut self.idle_notifier_state
	}
}

impl State {
	pub fn notify_input_activity(&mut self) {
		if self.is_idle {
			self.is_idle = false;
		}
		self.idle_notifier_state.notify_activity(&self._seat);
	}

	pub fn _check_idle_timeout(&mut self) {
		if self.idle_timeout_secs == 0 {
			return;
		}
	}
}

impl smithay::wayland::fractional_scale::FractionalScaleHandler for State {}

impl smithay::wayland::input_method::InputMethodHandler for State {
	fn new_popup(&mut self, _popup: smithay::wayland::input_method::PopupSurface) {
		self.mark_redraw();
	}

	fn dismiss_popup(&mut self, _popup: smithay::wayland::input_method::PopupSurface) {
		self.mark_redraw();
	}

	fn popup_repositioned(&mut self, _popup: smithay::wayland::input_method::PopupSurface) {
		self.mark_redraw();
	}

	fn parent_geometry(&self, _surface: &WlSurface) -> smithay::utils::Rectangle<i32, smithay::utils::Logical> {
		self.focused_surface
			.as_ref()
			.and_then(|focused| self.window_for_surface(focused))
			.and_then(|id| self.windows.get(&id).map(|window| window.geometry))
			.unwrap_or_else(|| smithay::utils::Rectangle::new((0, 0).into(), self.output_size))
	}
}

impl smithay::wayland::selection::data_device::DataDeviceHandler for State {
	fn data_device_state(&mut self) -> &mut smithay::wayland::selection::data_device::DataDeviceState {
		&mut self.data_device_state
	}
}

impl smithay::wayland::selection::primary_selection::PrimarySelectionHandler for State {
	fn primary_selection_state(&mut self) -> &mut smithay::wayland::selection::primary_selection::PrimarySelectionState {
		self.primary_selection_state
			.as_mut()
			.expect("primary_selection_state must be initialized")
	}
}

impl smithay::wayland::selection::SelectionHandler for State {
	type SelectionUserData = ();
}

impl smithay::wayland::selection::data_device::WaylandDndGrabHandler for State {}

impl smithay::input::dnd::DndGrabHandler for State {}

impl smithay::wayland::xdg_activation::XdgActivationHandler for State {
	fn activation_state(&mut self) -> &mut smithay::wayland::xdg_activation::XdgActivationState {
		&mut self.xdg_activation_state
	}

	fn request_activation(
		&mut self,
		_token: smithay::wayland::xdg_activation::XdgActivationToken,
		token_data: smithay::wayland::xdg_activation::XdgActivationTokenData,
		surface: WlSurface,
	) {
		if token_data.timestamp.elapsed().as_secs() > 10 {
			return;
		}

		if let Some(window_id) = self.window_for_surface(&surface) {
			let is_focused = self.focused_surface.as_ref().is_some_and(|s| *s == surface);
			if !is_focused {
				if let Some(window_data) = self.windows.get_mut(&window_id) {
					window_data.flags.urgent = true;
					tracing::info!(id = ?window_id, "Window marked urgent (activation request)");
					self.mark_redraw();
				}
			}
		}
	}
}

smithay::delegate_xdg_activation!(State);
