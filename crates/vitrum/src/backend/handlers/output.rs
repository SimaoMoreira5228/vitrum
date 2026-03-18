use smithay::reexports::wayland_server::protocol::wl_output::WlOutput;
use smithay::wayland::foreign_toplevel_list::{ForeignToplevelListHandler, ForeignToplevelListState};
use smithay::wayland::output::OutputHandler;
use tracing::info;

use crate::backend::State;

impl ForeignToplevelListHandler for State {
	fn foreign_toplevel_list_state(&mut self) -> &mut ForeignToplevelListState {
		&mut self._foreign_toplevel_list_state
	}
}

impl OutputHandler for State {
	fn output_bound(&mut self, output: smithay::output::Output, _wl_output: WlOutput) {
		let output_name = output.name();

		if self.output_manager.map().get_by_name(&output_name).is_some() {
			return;
		}

		let output_state = crate::output::OutputState::new(output_name.clone(), self.output_size);
		let output_id = self.output_manager.add_output(output_state);

		if self.output_id.is_none() {
			self.output_id = Some(output_id);
		}

		info!(name = %output_name, output_id = ?output_id, "Output bound via wl_output");
	}
}
