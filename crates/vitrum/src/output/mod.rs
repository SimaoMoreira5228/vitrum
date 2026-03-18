use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};

use smithay::output::Output;
use smithay::reexports::wayland_server::protocol::wl_output::WlOutput;
use smithay::utils::{Logical, Point, Rectangle, Size, Transform};
use tracing::{debug, info, warn};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct OutputId(pub u64);

static OUTPUT_ID_COUNTER: AtomicU64 = AtomicU64::new(0);

impl OutputId {
	pub fn next() -> Self {
		OutputId(OUTPUT_ID_COUNTER.fetch_add(1, Ordering::Relaxed) + 1)
	}
}

impl Default for OutputId {
	fn default() -> Self {
		Self::next()
	}
}

#[derive(Debug, Clone)]
pub struct OutputState {
	pub id: OutputId,
	pub name: String,
	pub make: String,
	pub model: String,
	pub size: Size<i32, Logical>,
	pub physical_size: Option<(i32, i32)>,
	pub position: Point<i32, Logical>,
	pub scale: f64,
	pub transform: Transform,
	pub refresh_rate_mhz: Option<u32>,
	pub enabled: bool,
	pub primary: bool,
	pub active_workspace: u32,
}

impl OutputState {
	pub fn new(name: impl Into<String>, size: Size<i32, Logical>) -> Self {
		Self {
			id: OutputId::next(),
			name: name.into(),
			make: String::new(),
			model: String::new(),
			size,
			physical_size: None,
			position: Point::from((0, 0)),
			scale: 1.0,
			transform: Transform::Normal,
			refresh_rate_mhz: None,
			enabled: true,
			primary: false,
			active_workspace: 1,
		}
	}

	pub fn geometry(&self) -> Rectangle<i32, Logical> {
		Rectangle {
			loc: self.position,
			size: self.size,
		}
	}

	pub fn physical_size_pixels(&self) -> Size<i32, smithay::utils::Physical> {
		Size::from((
			(self.size.w as f64 * self.scale) as i32,
			(self.size.h as f64 * self.scale) as i32,
		))
	}

	pub fn set_position(&mut self, x: i32, y: i32) {
		self.position = Point::from((x, y));
	}

	pub fn set_scale(&mut self, scale: f64) {
		self.scale = scale.max(0.1).min(10.0);
	}

	pub fn set_transform(&mut self, transform: Transform) {
		self.transform = transform;
	}

	pub fn contains(&self, point: Point<f64, Logical>) -> bool {
		let geo = self.geometry();
		point.x >= geo.loc.x as f64
			&& point.x < (geo.loc.x + geo.size.w) as f64
			&& point.y >= geo.loc.y as f64
			&& point.y < (geo.loc.y + geo.size.h) as f64
	}
}

#[derive(Debug, Default)]
pub struct OutputMap {
	outputs: HashMap<OutputId, OutputState>,
	name_to_id: HashMap<String, OutputId>,
	primary_output: Option<OutputId>,
}

impl OutputMap {
	pub fn new() -> Self {
		Self::default()
	}

	pub fn add_output(&mut self, output: OutputState) -> OutputId {
		let id = output.id;
		let name = output.name.clone();

		if self.outputs.is_empty() {
			self.primary_output = Some(id);
		}

		self.name_to_id.insert(name.clone(), id);
		self.outputs.insert(id, output);

		info!(
			id = ?id,
			name = %name,
			size = ?self.outputs[&id].size,
			"Output added"
		);

		id
	}

	pub fn remove_output(&mut self, id: OutputId) -> Option<OutputState> {
		if let Some(output) = self.outputs.remove(&id) {
			self.name_to_id.remove(&output.name);

			if self.primary_output == Some(id) {
				self.primary_output = self.outputs.keys().next().copied();
			}

			info!(id = ?id, name = %output.name, "Output removed");
			Some(output)
		} else {
			None
		}
	}

	pub fn get(&self, id: OutputId) -> Option<&OutputState> {
		self.outputs.get(&id)
	}

	pub fn get_mut(&mut self, id: OutputId) -> Option<&mut OutputState> {
		self.outputs.get_mut(&id)
	}

	pub fn get_by_name(&self, name: &str) -> Option<&OutputState> {
		self.name_to_id.get(name).and_then(|id| self.outputs.get(id))
	}

	pub fn get_by_name_mut(&mut self, name: &str) -> Option<&mut OutputState> {
		if let Some(&id) = self.name_to_id.get(name) {
			self.outputs.get_mut(&id)
		} else {
			None
		}
	}

	pub fn primary(&self) -> Option<&OutputState> {
		self.primary_output.and_then(|id| self.outputs.get(&id))
	}

	pub fn primary_mut(&mut self) -> Option<&mut OutputState> {
		if let Some(id) = self.primary_output {
			self.outputs.get_mut(&id)
		} else {
			None
		}
	}

	pub fn set_primary(&mut self, id: OutputId) {
		if self.outputs.contains_key(&id) {
			if self.primary_output != Some(id) {
				info!(id = ?id, "Primary output changed");
				self.primary_output = Some(id);
			}
		} else {
			warn!(id = ?id, "Cannot set primary: output not found");
		}
	}

	pub fn outputs(&self) -> &HashMap<OutputId, OutputState> {
		&self.outputs
	}

	pub fn outputs_mut(&mut self) -> &mut HashMap<OutputId, OutputState> {
		&mut self.outputs
	}

	pub fn sorted_outputs(&self) -> Vec<&OutputState> {
		let mut outputs: Vec<&OutputState> = self.outputs.values().collect();
		outputs.sort_by(|a, b| a.position.y.cmp(&b.position.y).then_with(|| a.position.x.cmp(&b.position.x)));
		outputs
	}

	pub fn len(&self) -> usize {
		self.outputs.len()
	}

	pub fn is_empty(&self) -> bool {
		self.outputs.is_empty()
	}

	pub fn output_at(&self, point: Point<f64, Logical>) -> Option<&OutputState> {
		self.outputs.values().find(|o| o.contains(point))
	}

	pub fn get_by_wl_output(&self, wl_output: &WlOutput) -> Option<&OutputState> {
		let output = Output::from_resource(wl_output)?;
		self.get_by_name(output.name().as_str())
	}

	pub fn output_id_for_wl_output(&self, wl_output: &WlOutput) -> Option<OutputId> {
		self.get_by_wl_output(wl_output).map(|o| o.id)
	}

	pub fn update_size(&mut self, id: OutputId, size: Size<i32, Logical>) {
		if let Some(output) = self.outputs.get_mut(&id) {
			if output.size != size {
				debug!(
					id = ?id,
					name = %output.name,
					old_size = ?output.size,
					new_size = ?size,
					"Output size changed"
				);
				output.size = size;
			}
		}
	}

	pub fn bounding_box(&self) -> Rectangle<i32, Logical> {
		if self.outputs.is_empty() {
			return Rectangle::from_size(Size::from((0, 0)));
		}

		let mut min_x = i32::MAX;
		let mut min_y = i32::MAX;
		let mut max_x = i32::MIN;
		let mut max_y = i32::MIN;

		for output in self.outputs.values() {
			let geo = output.geometry();
			min_x = min_x.min(geo.loc.x);
			min_y = min_y.min(geo.loc.y);
			max_x = max_x.max(geo.loc.x + geo.size.w);
			max_y = max_y.max(geo.loc.y + geo.size.h);
		}

		Rectangle {
			loc: Point::from((min_x, min_y)),
			size: Size::from((max_x - min_x, max_y - min_y)),
		}
	}

	pub fn auto_arrange_horizontal(&mut self) {
		let output_info: Vec<(OutputId, i32)> = self.sorted_outputs().into_iter().map(|o| (o.id, o.size.w)).collect();

		let mut x = 0;
		for (id, width) in output_info {
			if let Some(o) = self.outputs.get_mut(&id) {
				o.set_position(x, 0);
			}
			x += width;
		}
		info!("Outputs auto-arranged horizontally");
	}

	pub fn auto_arrange_vertical(&mut self) {
		let output_info: Vec<(OutputId, i32)> = self.sorted_outputs().into_iter().map(|o| (o.id, o.size.h)).collect();

		let mut y = 0;
		for (id, height) in output_info {
			if let Some(o) = self.outputs.get_mut(&id) {
				o.set_position(0, y);
			}
			y += height;
		}
		info!("Outputs auto-arranged vertically");
	}

	pub fn workspace_range(&self, id: OutputId, total_workspaces: u32) -> Option<(u32, u32)> {
		let sorted = self.sorted_outputs();
		let position = sorted.iter().position(|o| o.id == id)?;

		let num_outputs = sorted.len() as u32;
		let workspaces_per_output = total_workspaces / num_outputs;
		let remainder = total_workspaces % num_outputs;

		let start = (0..position as u32)
			.map(|i| workspaces_per_output + if i < remainder { 1 } else { 0 })
			.sum::<u32>()
			+ 1;

		let count = workspaces_per_output + if (position as u32) < remainder { 1 } else { 0 };

		Some((start, start + count - 1))
	}
}

#[derive(Debug, Clone)]
pub enum OutputChange {
	Added(OutputId),
	Removed(OutputId),
	Changed(OutputId, OutputProperty),
}

#[derive(Debug, Clone)]
pub enum OutputProperty {
	Size(Size<i32, Logical>),
	Position(Point<i32, Logical>),
	Scale(f64),
	Transform(Transform),
	Enabled(bool),
}

pub struct OutputManager {
	map: OutputMap,
	changes: Vec<OutputChange>,
}

impl OutputManager {
	pub fn new() -> Self {
		Self {
			map: OutputMap::new(),
			changes: Vec::new(),
		}
	}

	pub fn map(&self) -> &OutputMap {
		&self.map
	}

	pub fn map_mut(&mut self) -> &mut OutputMap {
		&mut self.map
	}

	pub fn add_output(&mut self, output: OutputState) -> OutputId {
		let id = self.map.add_output(output);
		self.changes.push(OutputChange::Added(id));
		id
	}

	pub fn remove_output(&mut self, id: OutputId) -> Option<OutputState> {
		let output = self.map.remove_output(id);
		if output.is_some() {
			self.changes.push(OutputChange::Removed(id));
		}
		output
	}

	pub fn update_property(&mut self, id: OutputId, property: OutputProperty) {
		if let Some(output) = self.map.get_mut(id) {
			match &property {
				OutputProperty::Size(size) => output.size = *size,
				OutputProperty::Position(pos) => output.position = *pos,
				OutputProperty::Scale(scale) => output.scale = *scale,
				OutputProperty::Transform(transform) => output.transform = *transform,
				OutputProperty::Enabled(enabled) => output.enabled = *enabled,
			}
			self.changes.push(OutputChange::Changed(id, property));
		}
	}

	pub fn take_changes(&mut self) -> Vec<OutputChange> {
		std::mem::take(&mut self.changes)
	}

	pub fn has_changes(&self) -> bool {
		!self.changes.is_empty()
	}

	pub fn primary_size(&self) -> Size<i32, Logical> {
		self.map.primary().map(|o| o.size).unwrap_or_else(|| Size::from((1920, 1080)))
	}

	pub fn get_by_wl_output(&self, wl_output: &WlOutput) -> Option<&OutputState> {
		self.map.get_by_wl_output(wl_output)
	}

	pub fn output_id_for_wl_output(&self, wl_output: &WlOutput) -> Option<OutputId> {
		self.map.output_id_for_wl_output(wl_output)
	}
}

impl Default for OutputManager {
	fn default() -> Self {
		Self::new()
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_output_state_contains() {
		let output = OutputState {
			id: OutputId::next(),
			name: "DP-1".to_string(),
			make: "Test".to_string(),
			model: "Monitor".to_string(),
			size: Size::from((1920, 1080)),
			physical_size: None,
			position: Point::from((0, 0)),
			scale: 1.0,
			transform: Transform::Normal,
			refresh_rate_mhz: Some(60000),
			enabled: true,
			primary: false,
			active_workspace: 1,
		};

		assert!(output.contains(Point::from((0.0, 0.0))));
		assert!(output.contains(Point::from((1919.0, 1079.0))));
		assert!(!output.contains(Point::from((1920.0, 1080.0))));
		assert!(!output.contains(Point::from((-1.0, -1.0))));
	}

	#[test]
	fn test_output_map() {
		let mut map = OutputMap::new();

		let output1 = OutputState::new("DP-1", Size::from((1920, 1080)));
		let id1 = map.add_output(output1);

		let output2 = OutputState::new("DP-2", Size::from((2560, 1440)));
		let id2 = map.add_output(output2);

		assert_eq!(map.len(), 2);
		assert!(map.get(id1).is_some());
		assert!(map.get(id2).is_some());

		assert_eq!(map.primary().map(|o| o.id), Some(id1));

		assert!(map.get_by_name("DP-1").is_some());
		assert!(map.get_by_name("DP-2").is_some());
		assert!(map.get_by_name("HDMI-1").is_none());
	}

	#[test]
	fn test_workspace_range() {
		let mut map = OutputMap::new();

		let mut output1 = OutputState::new("DP-1", Size::from((1920, 1080)));
		output1.position = Point::from((0, 0));
		let id1 = map.add_output(output1);

		let mut output2 = OutputState::new("DP-2", Size::from((2560, 1440)));
		output2.position = Point::from((1920, 0));
		let id2 = map.add_output(output2);

		assert_eq!(map.workspace_range(id1, 10), Some((1, 5)));
		assert_eq!(map.workspace_range(id2, 10), Some((6, 10)));
	}

	#[test]
	fn test_multi_output_auto_arrange_and_bounding_box() {
		let mut map = OutputMap::new();
		let mut dp1 = OutputState::new("DP-1", Size::from((1920, 1080)));
		dp1.position = Point::from((500, 500));
		map.add_output(dp1);

		let mut dp2 = OutputState::new("DP-2", Size::from((2560, 1440)));
		dp2.position = Point::from((-200, 10));
		map.add_output(dp2);

		let mut hdmi = OutputState::new("HDMI-1", Size::from((1280, 1024)));
		hdmi.position = Point::from((2000, -100));
		map.add_output(hdmi);

		map.auto_arrange_horizontal();

		let mut arranged: Vec<_> = map.outputs().values().collect();
		arranged.sort_by_key(|output| output.position.x);

		assert_eq!(arranged.len(), 3);
		assert!(arranged.iter().all(|output| output.position.y == 0));
		assert_eq!(arranged[0].position.x, 0);
		assert_eq!(arranged[1].position.x, arranged[0].size.w);
		assert_eq!(arranged[2].position.x, arranged[0].size.w + arranged[1].size.w);

		let bbox = map.bounding_box();
		assert_eq!(bbox.loc, Point::from((0, 0)));
		assert_eq!(bbox.size, Size::from((5760, 1440)));
	}

	#[test]
	fn test_workspace_range_three_outputs_with_remainder() {
		let mut map = OutputMap::new();

		let mut left = OutputState::new("LEFT", Size::from((1920, 1080)));
		left.position = Point::from((0, 0));
		let left_id = map.add_output(left);

		let mut center = OutputState::new("CENTER", Size::from((1920, 1080)));
		center.position = Point::from((1920, 0));
		let center_id = map.add_output(center);

		let mut right = OutputState::new("RIGHT", Size::from((1920, 1080)));
		right.position = Point::from((3840, 0));
		let right_id = map.add_output(right);

		assert_eq!(map.workspace_range(left_id, 10), Some((1, 4)));
		assert_eq!(map.workspace_range(center_id, 10), Some((5, 7)));
		assert_eq!(map.workspace_range(right_id, 10), Some((8, 10)));
	}

	#[test]
	fn test_primary_output_falls_back_when_removed() {
		let mut map = OutputMap::new();

		let id1 = map.add_output(OutputState::new("DP-1", Size::from((1920, 1080))));
		let id2 = map.add_output(OutputState::new("DP-2", Size::from((2560, 1440))));

		assert_eq!(map.primary().map(|o| o.id), Some(id1));

		map.remove_output(id1);

		assert_eq!(map.primary().map(|o| o.id), Some(id2));
	}
}
