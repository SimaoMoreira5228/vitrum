#![allow(dead_code)]

use std::collections::{HashMap, HashSet};

use tracing::debug;

use crate::window::WindowId;

mod output;

pub use output::OutputWorkspaces;

#[derive(Debug, Clone)]
pub struct Workspace {
	pub id: u32,
	pub name: String,
	pub windows: HashSet<WindowId>,
	pub focused_window: Option<WindowId>,
}

impl Workspace {
	pub fn new(id: u32) -> Self {
		Self {
			id,
			name: format!("{}", id),
			windows: HashSet::new(),
			focused_window: None,
		}
	}

	pub fn add_window(&mut self, window_id: WindowId) {
		debug!(workspace = self.id, window = ?window_id, "Adding window to workspace");
		self.windows.insert(window_id);
	}

	pub fn remove_window(&mut self, id: WindowId) -> bool {
		debug!(workspace = self.id, window = ?id, "Removing window from workspace");
		if self.focused_window == Some(id) {
			self.focused_window = self.windows.iter().find(|&&k| k != id).copied();
		}
		self.windows.remove(&id)
	}

	pub fn focus_window(&mut self, id: WindowId) {
		if self.windows.contains(&id) {
			self.focused_window = Some(id);
			debug!(workspace = self.id, window = ?id, "Focused window");
		}
	}

	pub fn contains(&self, id: WindowId) -> bool {
		self.windows.contains(&id)
	}

	pub fn window_count(&self) -> usize {
		self.windows.len()
	}

	pub fn is_empty(&self) -> bool {
		self.windows.is_empty()
	}
}

#[derive(Debug, Clone)]
pub struct WorkspaceSet {
	workspaces: HashMap<u32, Workspace>,
	active: u32,
	output: Option<String>,
}

impl WorkspaceSet {
	pub fn new() -> Self {
		let mut workspaces = HashMap::new();

		for id in 1..=10 {
			workspaces.insert(id, Workspace::new(id));
		}

		Self {
			workspaces,
			active: 1,
			output: None,
		}
	}

	pub fn with_output(output: String) -> Self {
		let mut set = Self::new();
		set.output = Some(output);
		set
	}

	pub fn get(&self, id: u32) -> Option<&Workspace> {
		self.workspaces.get(&id)
	}

	pub fn get_mut(&mut self, id: u32) -> Option<&mut Workspace> {
		self.workspaces.get_mut(&id)
	}

	pub fn active(&self) -> &Workspace {
		self.workspaces.get(&self.active).expect("Active workspace must exist")
	}

	pub fn active_mut(&mut self) -> &mut Workspace {
		self.workspaces.get_mut(&self.active).expect("Active workspace must exist")
	}

	pub fn active_id(&self) -> u32 {
		self.active
	}

	pub fn switch_to(&mut self, id: u32) {
		if self.workspaces.contains_key(&id) {
			debug!(from = self.active, to = id, "Switching workspace");
			self.active = id;
		}
	}

	pub fn next(&mut self) {
		let next = if self.active >= 10 { 1 } else { self.active + 1 };
		self.switch_to(next);
	}

	pub fn prev(&mut self) {
		let prev = if self.active <= 1 { 10 } else { self.active - 1 };
		self.switch_to(prev);
	}

	pub fn all(&self) -> &HashMap<u32, Workspace> {
		&self.workspaces
	}

	pub fn move_window(&mut self, window_id: WindowId, target: u32) -> bool {
		if self.active_mut().remove_window(window_id) {
			if let Some(target_workspace) = self.workspaces.get_mut(&target) {
				target_workspace.add_window(window_id);
				return true;
			}
		}
		false
	}
}

impl Default for WorkspaceSet {
	fn default() -> Self {
		Self::new()
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::window::new_window_id;

	#[test]
	fn test_workspace_new_is_empty() {
		let ws = Workspace::new(1);
		assert!(ws.is_empty());
		assert_eq!(ws.window_count(), 0);
	}

	#[test]
	fn test_workspace_add_remove_windows() {
		let mut ws = Workspace::new(1);
		let id1 = new_window_id();
		let id2 = new_window_id();

		ws.add_window(id1);
		ws.add_window(id2);
		assert_eq!(ws.window_count(), 2);
		assert!(ws.contains(id1));
		assert!(ws.contains(id2));

		assert!(ws.remove_window(id1));
		assert_eq!(ws.window_count(), 1);
		assert!(!ws.contains(id1));
		assert!(ws.contains(id2));

		assert!(!ws.remove_window(new_window_id()));
	}

	#[test]
	fn test_workspace_focus_window() {
		let mut ws = Workspace::new(1);
		let id = new_window_id();
		ws.add_window(id);

		ws.focus_window(id);
		assert_eq!(ws.focused_window, Some(id));
	}

	#[test]
	fn test_workspace_focus_nonexistent_ignored() {
		let mut ws = Workspace::new(1);
		let id = new_window_id();

		ws.focus_window(id);
		assert_eq!(ws.focused_window, None);
	}

	#[test]
	fn test_workspace_remove_focused_updates_focus() {
		let mut ws = Workspace::new(1);
		let id1 = new_window_id();
		let id2 = new_window_id();
		ws.add_window(id1);
		ws.add_window(id2);
		ws.focus_window(id1);

		ws.remove_window(id1);

		assert_ne!(ws.focused_window, Some(id1));
	}

	#[test]
	fn test_workspace_set_defaults_to_workspace_1() {
		let set = WorkspaceSet::new();
		assert_eq!(set.active_id(), 1);
	}

	#[test]
	fn test_workspace_set_has_ten_workspaces() {
		let set = WorkspaceSet::new();
		for id in 1..=10 {
			assert!(set.get(id).is_some());
		}
		assert!(set.get(11).is_none());
		assert!(set.get(0).is_none());
	}

	#[test]
	fn test_workspace_set_switch_to() {
		let mut set = WorkspaceSet::new();
		set.switch_to(5);
		assert_eq!(set.active_id(), 5);
	}

	#[test]
	fn test_workspace_set_switch_invalid_ignored() {
		let mut set = WorkspaceSet::new();
		set.switch_to(0);
		assert_eq!(set.active_id(), 1);
		set.switch_to(11);
		assert_eq!(set.active_id(), 1);
	}

	#[test]
	fn test_workspace_set_next_wraps() {
		let mut set = WorkspaceSet::new();
		set.switch_to(10);
		set.next();
		assert_eq!(set.active_id(), 1);
	}

	#[test]
	fn test_workspace_set_prev_wraps() {
		let mut set = WorkspaceSet::new();
		set.switch_to(1);
		set.prev();
		assert_eq!(set.active_id(), 10);
	}

	#[test]
	fn test_workspace_set_move_window() {
		let mut set = WorkspaceSet::new();
		let id = new_window_id();

		set.active_mut().add_window(id);
		assert!(set.active().contains(id));

		assert!(set.move_window(id, 3));
		assert!(!set.active().contains(id));
		assert!(set.get(3).unwrap().contains(id));
	}

	#[test]
	fn test_workspace_set_move_nonexistent_returns_false() {
		let mut set = WorkspaceSet::new();
		let id = new_window_id();
		assert!(!set.move_window(id, 5));
	}

	#[test]
	fn test_workspace_set_move_to_invalid_workspace_returns_false() {
		let mut set = WorkspaceSet::new();
		let id = new_window_id();
		set.active_mut().add_window(id);
		assert!(!set.move_window(id, 11));
	}
}
