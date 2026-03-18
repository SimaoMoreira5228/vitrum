#![allow(dead_code)]

use std::collections::HashMap;

use smithay::utils::{Logical, Point, Rectangle, Size};

use crate::window::{WindowData, WindowId};
use crate::workspace::Workspace;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LayoutMode {
	#[default]
	Dwindle,
	MasterStack,
	Floating,
}

#[derive(Debug, Clone)]
pub struct LayoutEngine {
	pub mode: LayoutMode,

	pub gaps_inner: i32,

	pub gaps_outer: i32,

	pub master_ratio: f32,

	workspace_layouts: HashMap<u32, LayoutMode>,
}

impl LayoutEngine {
	pub fn new() -> Self {
		Self {
			mode: LayoutMode::Dwindle,
			gaps_inner: 8,
			gaps_outer: 12,
			master_ratio: 0.5,
			workspace_layouts: HashMap::new(),
		}
	}

	pub fn set_workspace_layout(&mut self, workspace: u32, mode: LayoutMode) {
		self.workspace_layouts.insert(workspace, mode);
	}

	pub fn set_gaps(&mut self, inner: i32, outer: i32) {
		self.gaps_inner = inner;
		self.gaps_outer = outer;
	}

	pub fn set_master_ratio(&mut self, ratio: f32) {
		self.master_ratio = ratio.clamp(0.1, 0.9);
	}

	pub fn get_workspace_layout(&self, workspace: u32) -> LayoutMode {
		self.workspace_layouts.get(&workspace).copied().unwrap_or(self.mode)
	}

	pub fn default_floating_geometry(&self, output_size: Size<i32, Logical>) -> Rectangle<i32, Logical> {
		let width = (output_size.w as f32 * 0.6) as i32;
		let height = (output_size.h as f32 * 0.6) as i32;
		let x = (output_size.w - width) / 2;
		let y = (output_size.h - height) / 2;
		Rectangle::new(Point::from((x, y)), Size::from((width, height)))
	}

	pub fn arrange_workspace<F>(
		&self,
		workspace: &Workspace,
		output_size: Size<i32, Logical>,
		get_window_data: F,
	) -> Vec<(WindowId, Rectangle<i32, Logical>)>
	where
		F: Fn(WindowId) -> Option<WindowData>,
	{
		let layout = self.get_workspace_layout(workspace.id);

		let tiled_windows: Vec<(WindowId, WindowData)> = workspace
			.windows
			.iter()
			.filter_map(|&id| get_window_data(id).map(|w| (id, w)))
			.filter(|(_, w)| !w.flags.floating && !w.flags.fullscreen)
			.collect();

		if tiled_windows.is_empty() {
			return Vec::new();
		}

		let available = Rectangle::new(
			Point::from((self.gaps_outer, self.gaps_outer)),
			Size::from((output_size.w - self.gaps_outer * 2, output_size.h - self.gaps_outer * 2)),
		);

		match layout {
			LayoutMode::Dwindle => self.arrange_dwindle(&tiled_windows, available),
			LayoutMode::MasterStack => self.arrange_master_stack(&tiled_windows, available),
			LayoutMode::Floating => tiled_windows.iter().map(|(id, w)| (*id, w.geometry)).collect(),
		}
	}

	fn arrange_dwindle(
		&self,
		windows: &[(WindowId, WindowData)],
		available: Rectangle<i32, Logical>,
	) -> Vec<(WindowId, Rectangle<i32, Logical>)> {
		let mut geometries = Vec::new();

		if windows.is_empty() {
			return geometries;
		}

		if windows.len() == 1 {
			geometries.push((windows[0].0, available));
			return geometries;
		}

		let mid = windows.len() / 2;
		let (left, right) = windows.split_at(mid);

		let horizontal = windows.len() % 2 == 0;

		let (left_rect, right_rect) = if horizontal {
			let left_width = available.size.w / 2 - self.gaps_inner / 2;
			let right_width = available.size.w - left_width - self.gaps_inner;

			let left_rect = Rectangle::new(available.loc, Size::from((left_width, available.size.h)));
			let right_rect = Rectangle::new(
				Point::from((available.loc.x + left_width + self.gaps_inner, available.loc.y)),
				Size::from((right_width, available.size.h)),
			);
			(left_rect, right_rect)
		} else {
			let top_height = available.size.h / 2 - self.gaps_inner / 2;
			let bottom_height = available.size.h - top_height - self.gaps_inner;

			let left_rect = Rectangle::new(available.loc, Size::from((available.size.w, top_height)));
			let right_rect = Rectangle::new(
				Point::from((available.loc.x, available.loc.y + top_height + self.gaps_inner)),
				Size::from((available.size.w, bottom_height)),
			);
			(left_rect, right_rect)
		};

		geometries.extend(self.arrange_dwindle(left, left_rect));
		geometries.extend(self.arrange_dwindle(right, right_rect));

		geometries
	}

	fn arrange_master_stack(
		&self,
		windows: &[(WindowId, WindowData)],
		available: Rectangle<i32, Logical>,
	) -> Vec<(WindowId, Rectangle<i32, Logical>)> {
		let mut geometries = Vec::new();

		if windows.is_empty() {
			return geometries;
		}

		if windows.len() == 1 {
			geometries.push((windows[0].0, available));
			return geometries;
		}

		let master_width = (available.size.w as f32 * self.master_ratio) as i32 - self.gaps_inner / 2;
		let stack_width = available.size.w - master_width - self.gaps_inner;

		let master_rect = Rectangle::new(available.loc, Size::from((master_width, available.size.h)));
		geometries.push((windows[0].0, master_rect));

		let stack_count = windows.len() - 1;
		let stack_height = (available.size.h - (stack_count as i32 - 1) * self.gaps_inner) / stack_count as i32;

		for (i, (id, _)) in windows.iter().skip(1).enumerate() {
			let y = available.loc.y + i as i32 * (stack_height + self.gaps_inner);
			let rect = Rectangle::new(
				Point::from((available.loc.x + master_width + self.gaps_inner, y)),
				Size::from((stack_width, stack_height)),
			);
			geometries.push((*id, rect));
		}

		geometries
	}
}

impl Default for LayoutEngine {
	fn default() -> Self {
		Self::new()
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::window::WindowId;

	fn make_id() -> WindowId {
		crate::window::new_window_id()
	}

	fn available_1920x1080() -> Rectangle<i32, Logical> {
		Rectangle::new(Point::from((12, 12)), Size::from((1896, 1056)))
	}

	fn available_1000x800() -> Rectangle<i32, Logical> {
		Rectangle::new(Point::from((0, 0)), Size::from((1000, 800)))
	}

	fn arrange_dwindle_ids(
		engine: &LayoutEngine,
		count: usize,
		available: Rectangle<i32, Logical>,
	) -> Vec<(WindowId, Rectangle<i32, Logical>)> {
		let ids: Vec<WindowId> = (0..count).map(|_| make_id()).collect();
		let raw_tuples: Vec<(WindowId, *const ())> = ids.iter().map(|&id| (id, std::ptr::null())).collect();
		let tuples: &[(WindowId, WindowData)] =
			unsafe { std::mem::transmute::<&[(WindowId, *const ())], &[(WindowId, WindowData)]>(&raw_tuples) };
		engine.arrange_dwindle(tuples, available)
	}

	fn arrange_master_stack_ids(
		engine: &LayoutEngine,
		count: usize,
		available: Rectangle<i32, Logical>,
	) -> Vec<(WindowId, Rectangle<i32, Logical>)> {
		let ids: Vec<WindowId> = (0..count).map(|_| make_id()).collect();
		let raw_tuples: Vec<(WindowId, *const ())> = ids.iter().map(|&id| (id, std::ptr::null())).collect();
		let tuples: &[(WindowId, WindowData)] =
			unsafe { std::mem::transmute::<&[(WindowId, *const ())], &[(WindowId, WindowData)]>(&raw_tuples) };
		engine.arrange_master_stack(tuples, available)
	}

	#[test]
	fn test_dwindle_single_window_fills_available() {
		let engine = LayoutEngine::new();
		let available = available_1920x1080();
		let result = arrange_dwindle_ids(&engine, 1, available);
		assert_eq!(result.len(), 1);
		assert_eq!(result[0].1, available);
	}

	#[test]
	fn test_dwindle_two_windows_split_horizontal() {
		let engine = LayoutEngine::new();
		let available = available_1000x800();
		let result = arrange_dwindle_ids(&engine, 2, available);
		assert_eq!(result.len(), 2);
		assert_eq!(result[0].1.loc, available.loc);
		let total_width = result[0].1.size.w + result[1].1.size.w + engine.gaps_inner;
		assert_eq!(total_width, available.size.w);
	}

	#[test]
	fn test_dwindle_three_windows() {
		let engine = LayoutEngine::new();
		let available = available_1000x800();
		let result = arrange_dwindle_ids(&engine, 3, available);
		assert_eq!(result.len(), 3);
		for (_, geo) in &result {
			assert!(geo.size.w > 0);
			assert!(geo.size.h > 0);
		}
	}

	#[test]
	fn test_dwindle_empty_returns_empty() {
		let engine = LayoutEngine::new();
		let available = available_1000x800();
		let result = arrange_dwindle_ids(&engine, 0, available);
		assert!(result.is_empty());
	}

	#[test]
	fn test_dwindle_no_overlaps() {
		let engine = LayoutEngine::new();
		let available = available_1920x1080();
		let result = arrange_dwindle_ids(&engine, 5, available);
		for i in 0..result.len() {
			for j in (i + 1)..result.len() {
				let a = result[i].1;
				let b = result[j].1;
				let overlaps = a.loc.x < b.loc.x + b.size.w
					&& a.loc.x + a.size.w > b.loc.x
					&& a.loc.y < b.loc.y + b.size.h
					&& a.loc.y + a.size.h > b.loc.y;
				assert!(!overlaps, "Window {} overlaps with window {}: {:?} vs {:?}", i, j, a, b);
			}
		}
	}

	#[test]
	fn test_dwindle_all_within_bounds() {
		let engine = LayoutEngine::new();
		let available = available_1920x1080();
		let result = arrange_dwindle_ids(&engine, 4, available);
		for (_, geo) in &result {
			assert!(geo.loc.x >= available.loc.x);
			assert!(geo.loc.y >= available.loc.y);
			assert!(geo.loc.x + geo.size.w <= available.loc.x + available.size.w);
			assert!(geo.loc.y + geo.size.h <= available.loc.y + available.size.h);
		}
	}

	#[test]
	fn test_dwindle_geometry_positive_size() {
		let engine = LayoutEngine::new();
		let available = available_1920x1080();
		for count in 1..=8 {
			let result = arrange_dwindle_ids(&engine, count, available);
			for (_, geo) in &result {
				assert!(geo.size.w > 0, "Window width must be positive for {} windows", count);
				assert!(geo.size.h > 0, "Window height must be positive for {} windows", count);
			}
		}
	}

	#[test]
	fn test_master_stack_single_window_fills_available() {
		let engine = LayoutEngine::new();
		let available = available_1000x800();
		let result = arrange_master_stack_ids(&engine, 1, available);
		assert_eq!(result.len(), 1);
		assert_eq!(result[0].1, available);
	}

	#[test]
	fn test_master_stack_two_windows_master_left() {
		let engine = LayoutEngine::new();
		let available = available_1000x800();
		let result = arrange_master_stack_ids(&engine, 2, available);
		assert_eq!(result.len(), 2);
		assert_eq!(result[0].1.loc, available.loc);
		assert!(result[1].1.loc.x > result[0].1.loc.x);
	}

	#[test]
	fn test_master_stack_master_ratio() {
		let mut engine = LayoutEngine::new();
		engine.set_master_ratio(0.6);
		let available = available_1000x800();
		let result = arrange_master_stack_ids(&engine, 2, available);
		let master_ratio = result[0].1.size.w as f32 / available.size.w as f32;
		assert!((master_ratio - 0.6).abs() < 0.05, "Master ratio was {}", master_ratio);
	}

	#[test]
	fn test_master_stack_stack_splits_vertically() {
		let engine = LayoutEngine::new();
		let available = available_1000x800();
		let result = arrange_master_stack_ids(&engine, 4, available);
		assert_eq!(result.len(), 4);
		for i in 2..4 {
			assert!(result[i].1.loc.y > result[i - 1].1.loc.y);
		}
	}

	#[test]
	fn test_master_stack_no_overlaps() {
		let engine = LayoutEngine::new();
		let available = available_1920x1080();
		let result = arrange_master_stack_ids(&engine, 5, available);
		for i in 0..result.len() {
			for j in (i + 1)..result.len() {
				let a = result[i].1;
				let b = result[j].1;
				let overlaps = a.loc.x < b.loc.x + b.size.w
					&& a.loc.x + a.size.w > b.loc.x
					&& a.loc.y < b.loc.y + b.size.h
					&& a.loc.y + a.size.h > b.loc.y;
				assert!(!overlaps, "Window {} overlaps with window {}: {:?} vs {:?}", i, j, a, b);
			}
		}
	}

	#[test]
	fn test_floating_geometry_centered() {
		let engine = LayoutEngine::new();
		let output = Size::from((1920, 1080));
		let geo = engine.default_floating_geometry(output);
		assert_eq!(geo.size.w, (1920.0 * 0.6) as i32);
		assert_eq!(geo.size.h, (1080.0 * 0.6) as i32);
		assert_eq!(geo.loc.x, (1920 - geo.size.w) / 2);
		assert_eq!(geo.loc.y, (1080 - geo.size.h) / 2);
	}

	#[test]
	fn test_floating_geometry_within_bounds() {
		let engine = LayoutEngine::new();
		let output = Size::from((800, 600));
		let geo = engine.default_floating_geometry(output);
		assert!(geo.size.w > 0);
		assert!(geo.size.h > 0);
		assert!(geo.loc.x >= 0);
		assert!(geo.loc.y >= 0);
		assert!(geo.loc.x + geo.size.w <= output.w);
		assert!(geo.loc.y + geo.size.h <= output.h);
	}

	#[test]
	fn test_gaps_reduce_available_space() {
		let mut engine_no_gaps = LayoutEngine::new();
		engine_no_gaps.set_gaps(0, 0);

		let mut engine_with_gaps = LayoutEngine::new();
		engine_with_gaps.set_gaps(10, 20);

		let output = Size::from((1920, 1080));

		let available_no = Rectangle::new(Point::from((0, 0)), output);
		let available_gap = Rectangle::new(
			Point::from((engine_with_gaps.gaps_outer, engine_with_gaps.gaps_outer)),
			Size::from((
				output.w - engine_with_gaps.gaps_outer * 2,
				output.h - engine_with_gaps.gaps_outer * 2,
			)),
		);

		let result_no = arrange_dwindle_ids(&engine_no_gaps, 1, available_no);
		let result_gap = arrange_dwindle_ids(&engine_with_gaps, 1, available_gap);

		assert!(result_gap[0].1.size.w < result_no[0].1.size.w);
		assert!(result_gap[0].1.size.h < result_no[0].1.size.h);
	}

	#[test]
	fn test_workspace_specific_layout() {
		let mut engine = LayoutEngine::new();
		engine.set_workspace_layout(2, LayoutMode::MasterStack);
		assert_eq!(engine.get_workspace_layout(1), LayoutMode::Dwindle);
		assert_eq!(engine.get_workspace_layout(2), LayoutMode::MasterStack);
	}

	#[test]
	fn test_master_ratio_clamped() {
		let mut engine = LayoutEngine::new();
		engine.set_master_ratio(0.05);
		assert!(engine.master_ratio >= 0.1);
		engine.set_master_ratio(0.95);
		assert!(engine.master_ratio <= 0.9);
	}
}
