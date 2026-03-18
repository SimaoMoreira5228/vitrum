use std::time::Duration;

use calloop::{EventLoop, LoopHandle};
use smithay::utils::{Logical, Point, Size};

use crate::backend::State;
use crate::window::WindowId;

pub struct Fixture {
	event_loop: EventLoop<'static, State>,
	_handle: LoopHandle<'static, State>,
	state: State,
}

impl Fixture {
	pub fn new() -> Self {
		let event_loop = EventLoop::try_new().unwrap();
		let handle = event_loop.handle();

		let display = smithay::reexports::wayland_server::Display::<State>::new().unwrap();
		let display_handle = display.handle();

		let state = State::new_for_test(handle.clone(), display_handle).unwrap();

		Self {
			event_loop,
			_handle: handle,
			state,
		}
	}

	pub fn dispatch_all(&mut self) {
		for _ in 0..5 {
			self.event_loop.dispatch(Duration::ZERO, &mut self.state).unwrap();
		}
	}

	pub fn add_client(&mut self) -> usize {
		0
	}

	pub fn has_client(&self, _id: usize) -> bool {
		true
	}

	pub fn create_xdg_window(&mut self, _client_id: usize, _app_id: &str) -> Option<WindowId> {
		let window_id = crate::window::new_window_id();
		self.state.add_window_to_active(window_id);
		self.state.mark_redraw();
		Some(window_id)
	}

	pub fn window_count(&self) -> usize {
		self.state.windows.len()
	}

	pub fn visible_window_count(&self) -> usize {
		let active = self.state.active_workspace_id();
		self.state
			.focused_workspace_set()
			.get(active)
			.map(|ws| ws.window_count())
			.unwrap_or(0)
	}

	pub fn active_workspace(&self) -> u32 {
		self.state.active_workspace_id()
	}

	pub fn switch_workspace(&mut self, id: u32) {
		self.state.switch_workspace(id);
	}

	pub fn switch_workspace_next(&mut self) {
		self.state._next_workspace_in_set();
		self.state.switch_workspace(self.state.focused_workspace_set().active_id());
	}

	pub fn switch_workspace_prev(&mut self) {
		self.state._prev_workspace_in_set();
		self.state.switch_workspace(self.state.focused_workspace_set().active_id());
	}

	pub fn move_window_to_workspace(&mut self, window_id: WindowId, workspace: u32) {
		let old_ws = self
			.state
			.focused_workspace_set()
			.all()
			.iter()
			.find(|(_, ws)| ws.contains(window_id))
			.map(|(id, _)| *id);

		if let Some(old_id) = old_ws {
			if old_id != workspace {
				self.state.move_window_in_set(window_id, old_id, workspace);
			}
		}
	}

	pub fn focused_window_id(&self) -> Option<WindowId> {
		self.state
			.focused_surface
			.as_ref()
			.and_then(|s| self.state.window_for_surface(s))
	}

	pub fn focus_direction(&mut self, dir: vitrum_ipc::Direction) {
		self.state.focus_direction(dir);
	}

	pub fn set_layout(&mut self, mode: vitrum_ipc::LayoutMode) {
		let layout_mode = match mode {
			vitrum_ipc::LayoutMode::Dwindle => crate::layout::LayoutMode::Dwindle,
			vitrum_ipc::LayoutMode::MasterStack => crate::layout::LayoutMode::MasterStack,
			vitrum_ipc::LayoutMode::Floating => crate::layout::LayoutMode::Floating,
		};
		let active = self.state.active_workspace_id();
		self.state.layout_engine.set_workspace_layout(active, layout_mode);
		self.state.apply_layout();
	}

	pub fn kill_window(&mut self, window_id: WindowId) {
		self.state.remove_window(window_id);
	}

	pub fn kill_workspace_window(&mut self, window_id: WindowId) {
		let active = self.state.active_workspace_id();
		self.state.remove_window_from(active, window_id);
	}

	pub fn toggle_floating(&mut self, window_id: WindowId) {
		self.state.toggle_floating(window_id);
	}

	pub fn toggle_fullscreen(&mut self, window_id: WindowId) {
		self.state.toggle_fullscreen(window_id);
	}
}
