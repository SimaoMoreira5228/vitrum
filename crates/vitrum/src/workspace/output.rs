use std::collections::HashMap;

use tracing::{debug, info, warn};

use crate::output::OutputId;
use crate::window::WindowId;
use crate::workspace::{Workspace, WorkspaceSet};

#[derive(Debug)]
pub struct OutputWorkspaces {
	output_sets: HashMap<OutputId, WorkspaceSet>,

	primary_output: Option<OutputId>,

	focused_output: Option<OutputId>,
}

impl OutputWorkspaces {
	pub fn new() -> Self {
		Self {
			output_sets: HashMap::new(),
			primary_output: None,
			focused_output: None,
		}
	}

	pub fn add_output(&mut self, output_id: OutputId, output_name: Option<String>) {
		debug!(output_id = ?output_id, "Adding output to workspace manager");

		let mut set = WorkspaceSet::new();
		if let Some(name) = output_name {
			set = WorkspaceSet::with_output(name);
		}

		self.output_sets.insert(output_id, set);

		if self.primary_output.is_none() {
			self.primary_output = Some(output_id);
			self.focused_output = Some(output_id);
			info!(output_id = ?output_id, "Set as primary output");
		}
	}

	pub fn remove_output(&mut self, output_id: OutputId) {
		debug!(output_id = ?output_id, "Removing output from workspace manager");

		self.output_sets.remove(&output_id);

		if self.primary_output == Some(output_id) {
			self.primary_output = self.output_sets.keys().next().copied();
			info!(new_primary = ?self.primary_output, "Updated primary output");
		}

		if self.focused_output == Some(output_id) {
			self.focused_output = self.primary_output;
		}
	}

	pub fn set_focused_output(&mut self, output_id: OutputId) {
		if self.output_sets.contains_key(&output_id) {
			debug!(output_id = ?output_id, "Setting focused output");
			self.focused_output = Some(output_id);
		} else {
			warn!(output_id = ?output_id, "Cannot focus unknown output");
		}
	}

	pub fn focused_output(&self) -> Option<OutputId> {
		self.focused_output
	}

	pub fn primary_output(&self) -> Option<OutputId> {
		self.primary_output
	}

	pub fn get(&self, output_id: OutputId) -> Option<&WorkspaceSet> {
		self.output_sets.get(&output_id)
	}

	pub fn get_mut(&mut self, output_id: OutputId) -> Option<&mut WorkspaceSet> {
		self.output_sets.get_mut(&output_id)
	}

	pub fn active_workspace(&self, output_id: OutputId) -> Option<&Workspace> {
		self.output_sets.get(&output_id).map(|set| set.active())
	}

	pub fn active_workspace_id(&self, output_id: OutputId) -> Option<u32> {
		self.output_sets.get(&output_id).map(|set| set.active_id())
	}

	pub fn switch_workspace(&mut self, output_id: OutputId, workspace_id: u32) -> bool {
		if let Some(set) = self.output_sets.get_mut(&output_id) {
			set.switch_to(workspace_id);
			debug!(output_id = ?output_id, workspace = workspace_id, "Switched workspace");
			true
		} else {
			warn!(output_id = ?output_id, "Cannot switch workspace on unknown output");
			false
		}
	}

	pub fn switch_workspace_on_focused(&mut self, workspace_id: u32) -> bool {
		if let Some(output_id) = self.focused_output {
			self.switch_workspace(output_id, workspace_id)
		} else {
			warn!("No focused output to switch workspace on");
			false
		}
	}

	pub fn next_workspace(&mut self, output_id: OutputId) {
		if let Some(set) = self.output_sets.get_mut(&output_id) {
			set.next();
		}
	}

	pub fn prev_workspace(&mut self, output_id: OutputId) {
		if let Some(set) = self.output_sets.get_mut(&output_id) {
			set.prev();
		}
	}

	pub fn move_window_to_workspace(
		&mut self,
		window_id: WindowId,
		target_workspace: u32,
		target_output: Option<OutputId>,
	) -> bool {
		let mut source_output = None;
		let mut source_workspace = None;

		for (output_id, set) in &self.output_sets {
			for (ws_id, ws) in set.all() {
				if ws.contains(window_id) {
					source_output = Some(*output_id);
					source_workspace = Some(*ws_id);
					break;
				}
			}
			if source_output.is_some() {
				break;
			}
		}

		if let (Some(src_output), Some(src_ws)) = (source_output, source_workspace) {
			let target_output = target_output.unwrap_or(src_output);

			if let Some(src_set) = self.output_sets.get_mut(&src_output) {
				if let Some(src_ws) = src_set.get_mut(src_ws) {
					src_ws.remove_window(window_id);
				}
			}

			if let Some(target_set) = self.output_sets.get_mut(&target_output) {
				if let Some(target_ws) = target_set.get_mut(target_workspace) {
					target_ws.add_window(window_id);
					debug!(
						window = ?window_id,
						from_output = ?src_output,
						to_output = ?target_output,
						to_workspace = target_workspace,
						"Moved window to workspace"
					);
					return true;
				}
			}
		}

		false
	}

	pub fn get_workspace_windows(&self, output_id: OutputId, workspace_id: u32) -> Option<Vec<WindowId>> {
		self.output_sets
			.get(&output_id)
			.and_then(|set| set.get(workspace_id))
			.map(|ws| ws.windows.iter().copied().collect())
	}

	pub fn all_outputs(&self) -> &HashMap<OutputId, WorkspaceSet> {
		&self.output_sets
	}

	pub fn has_output(&self, output_id: OutputId) -> bool {
		self.output_sets.contains_key(&output_id)
	}

	pub fn get_visible_windows(&self, output_id: OutputId) -> Vec<WindowId> {
		self.active_workspace(output_id)
			.map(|ws| ws.windows.iter().copied().collect())
			.unwrap_or_default()
	}

	pub fn get_all_visible_windows(&self) -> Vec<(OutputId, Vec<WindowId>)> {
		self.output_sets
			.iter()
			.map(|(&output_id, set)| {
				let windows: Vec<WindowId> = set.active().windows.iter().copied().collect();
				(output_id, windows)
			})
			.collect()
	}
}

impl Default for OutputWorkspaces {
	fn default() -> Self {
		Self::new()
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_output_workspaces() {
		let mut manager = OutputWorkspaces::new();

		let output1 = OutputId::next();
		let output2 = OutputId::next();

		manager.add_output(output1, Some("DP-1".to_string()));
		manager.add_output(output2, Some("DP-2".to_string()));

		assert_eq!(manager.active_workspace_id(output1), Some(1));
		assert_eq!(manager.active_workspace_id(output2), Some(1));

		manager.switch_workspace(output1, 3);
		assert_eq!(manager.active_workspace_id(output1), Some(3));
		assert_eq!(manager.active_workspace_id(output2), Some(1));
	}

	#[test]
	fn test_focused_output() {
		let mut manager = OutputWorkspaces::new();
		let output1 = OutputId::next();

		manager.add_output(output1, None);

		assert_eq!(manager.focused_output(), Some(output1));

		manager.switch_workspace_on_focused(5);
		assert_eq!(manager.active_workspace_id(output1), Some(5));
	}
}
