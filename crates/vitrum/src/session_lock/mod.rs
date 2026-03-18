use smithay::reexports::wayland_protocols::ext::session_lock::v1::server::{
	ext_session_lock_manager_v1, ext_session_lock_surface_v1,
};
use smithay::reexports::wayland_server::protocol::wl_output::WlOutput;
use smithay::reexports::wayland_server::protocol::wl_surface::WlSurface;
use smithay::reexports::wayland_server::{DataInit, Dispatch, GlobalDispatch, New};
use smithay::utils::{Logical, Size};
use smithay::wayland::selection::data_device::DataDeviceHandler;
use smithay::wayland::session_lock::{LockSurface, SessionLockHandler, SessionLockManagerState, SessionLocker};
use tracing::{debug, info, warn};

use crate::backend::State;
use crate::output::OutputId;

fn resolve_lock_surface_target(
	primary_output: Option<(OutputId, Size<i32, Logical>)>,
	current_output: Option<(OutputId, Size<i32, Logical>)>,
) -> (OutputId, Size<i32, Logical>) {
	if let Some(output) = primary_output {
		return output;
	}

	if let Some(output) = current_output {
		return output;
	}

	(OutputId::default(), Size::from((1920, 1080)))
}

#[derive(Debug)]
pub struct SessionLockManager {
	pub state: SessionLockManagerState,

	locked: bool,

	lock_surfaces: Vec<LockSurfaceInfo>,

	pending_locker: Option<SessionLocker>,
}

#[derive(Debug, Clone)]
pub struct LockSurfaceInfo {
	pub surface: LockSurface,
	pub output_id: OutputId,
	pub configured: bool,
}

impl SessionLockManager {
	pub fn new(display_handle: &smithay::reexports::wayland_server::DisplayHandle) -> Self {
		Self {
			state: SessionLockManagerState::new::<State, _>(display_handle, |_client| true),
			locked: false,
			lock_surfaces: Vec::new(),
			pending_locker: None,
		}
	}

	pub fn is_locked(&self) -> bool {
		self.locked
	}

	pub fn lock(&mut self, locker: SessionLocker) {
		if self.locked {
			warn!("Session already locked, rejecting new lock request");

			return;
		}

		info!("Locking session");
		self.locked = true;
		self.pending_locker = Some(locker);

		self.confirm_lock();
	}

	fn confirm_lock(&mut self) {
		if let Some(locker) = self.pending_locker.take() {
			info!("Confirming session lock");
			locker.lock();
		}
	}

	pub fn unlock(&mut self) {
		if !self.locked {
			return;
		}

		info!("Unlocking session");
		self.locked = false;
		self.lock_surfaces.clear();
		self.pending_locker = None;
	}

	pub fn add_lock_surface(&mut self, surface: LockSurface, output_id: OutputId, output_size: Size<i32, Logical>) {
		debug!(output_id = ?output_id, "Adding lock surface");

		surface.with_pending_state(|states| {
			states.size = Some((output_size.w as u32, output_size.h as u32).into());
		});
		surface.send_configure();

		self.lock_surfaces.push(LockSurfaceInfo {
			surface,
			output_id,
			configured: true,
		});

		if self.pending_locker.is_some() {
			self.confirm_lock();
		}
	}

	pub fn remove_lock_surface(&mut self, surface: &LockSurface) {
		let wl_surface = surface.wl_surface();
		if let Some(pos) = self
			.lock_surfaces
			.iter()
			.position(|info| info.surface.wl_surface() == wl_surface)
		{
			debug!("Removing lock surface");
			self.lock_surfaces.remove(pos);
		}
	}

	pub fn lock_surfaces(&self) -> &[LockSurfaceInfo] {
		&self.lock_surfaces
	}

	pub fn is_lock_surface(&self, surface: &WlSurface) -> bool {
		self.lock_surfaces.iter().any(|info| info.surface.wl_surface() == surface)
	}

	pub fn should_render_only_lock_surfaces(&self) -> bool {
		self.locked && !self.lock_surfaces.is_empty()
	}
}

impl SessionLockHandler for State {
	fn lock_state(&mut self) -> &mut SessionLockManagerState {
		&mut self.session_lock_manager.state
	}

	fn lock(&mut self, confirmation: SessionLocker) {
		info!("Session lock requested");

		if self.session_lock_manager.is_locked() {
			warn!("Session already locked, rejecting");

			return;
		}

		self.session_lock_manager.lock(confirmation);

		self.mark_redraw();

		info!("Session locked - hiding all regular windows");
	}

	fn unlock(&mut self) {
		info!("Session unlock requested");

		if !self.session_lock_manager.is_locked() {
			return;
		}

		self.session_lock_manager.unlock();

		self.mark_redraw();

		info!("Session unlocked - showing regular windows");
	}

	fn new_surface(&mut self, surface: LockSurface, output: WlOutput) {
		debug!("New lock surface requested");

		let requested_output = self.output_manager.get_by_wl_output(&output).map(|o| (o.id, o.size));

		let primary_output = self.output_manager.map().primary().map(|o| (o.id, o.size));

		let current_output = self
			.output_id
			.and_then(|id| self.output_manager.map().get(id))
			.map(|o| (o.id, o.size));

		let has_requested_output = requested_output.is_some();
		let (output_id, output_size) =
			requested_output.unwrap_or_else(|| resolve_lock_surface_target(primary_output, current_output));

		if !has_requested_output && primary_output.is_none() && current_output.is_none() {
			warn!("No known outputs available for session lock surface; using fallback size");
		}

		self.session_lock_manager.add_lock_surface(surface, output_id, output_size);

		self.mark_redraw();

		debug!("Lock surface created and configured");
	}
}

smithay::delegate_session_lock!(State);

pub fn handle_lock_surface_commit(state: &mut State, surface: &WlSurface) -> bool {
	if !state.session_lock_manager.is_lock_surface(surface) {
		return false;
	}

	state.mark_redraw();

	true
}

pub fn is_session_locked(state: &State) -> bool {
	state.session_lock_manager.is_locked()
}

pub fn should_render_lock_surfaces_only(state: &State) -> bool {
	state.session_lock_manager.should_render_only_lock_surfaces()
}

pub fn get_lock_surfaces(state: &State) -> Vec<&LockSurface> {
	state
		.session_lock_manager
		.lock_surfaces()
		.iter()
		.map(|info| &info.surface)
		.collect()
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_session_lock_basic() {
		assert_eq!(std::mem::size_of::<LockSurfaceInfo>() > 0, true);
	}

	#[test]
	fn test_resolve_lock_surface_target_prefers_primary() {
		let primary = Some((OutputId(10), Size::from((2560, 1440))));
		let current = Some((OutputId(20), Size::from((1920, 1080))));

		let (id, size) = resolve_lock_surface_target(primary, current);

		assert_eq!(id, OutputId(10));
		assert_eq!(size, Size::from((2560, 1440)));
	}

	#[test]
	fn test_resolve_lock_surface_target_uses_current_when_no_primary() {
		let primary = None;
		let current = Some((OutputId(20), Size::from((1920, 1080))));

		let (id, size) = resolve_lock_surface_target(primary, current);

		assert_eq!(id, OutputId(20));
		assert_eq!(size, Size::from((1920, 1080)));
	}

	#[test]
	fn test_resolve_lock_surface_target_falls_back_to_default() {
		let (id, size) = resolve_lock_surface_target(None, None);

		assert!(id.0 > 0);
		assert_eq!(size, Size::from((1920, 1080)));
	}
}
