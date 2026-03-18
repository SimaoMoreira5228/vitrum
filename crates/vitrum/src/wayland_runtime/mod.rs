use std::time::Instant;

use anyhow::{Result, anyhow};
use smithay::reexports::calloop::LoopHandle;
use smithay::reexports::wayland_server::{Display, DisplayHandle};
use tracing::info;

use crate::backend::State;

pub struct WaylandRuntime {
	pub display: Display<State>,
	pub display_handle: DisplayHandle,
	started_at: Instant,
}

impl WaylandRuntime {
	pub fn new() -> Result<Self> {
		let display: Display<State> =
			Display::new().map_err(|e| anyhow::anyhow!("Failed to create Wayland display: {e}"))?;
		let display_handle = display.handle();

		Ok(Self {
			display,
			display_handle,
			started_at: Instant::now(),
		})
	}

	pub fn create_state(&self, config: crate::config::Config, loop_handle: LoopHandle<'static, State>) -> Result<State> {
		State::new(config, loop_handle, self.display.handle())
	}

	pub fn display_handle(&self) -> DisplayHandle {
		self.display_handle.clone()
	}

	pub fn start_time(&self) -> Instant {
		self.started_at
	}
}
