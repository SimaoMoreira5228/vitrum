use std::collections::VecDeque;

use smithay::utils::{Physical, Rectangle};
use tracing::trace;

const MAX_DAMAGE_RECTANGLES: usize = 10;

#[derive(Debug, Clone)]
pub struct DamageTracker {
	damage: VecDeque<Rectangle<i32, Physical>>,

	output_size: (i32, i32),

	full_damage: bool,

	frame_count: u64,
}

impl DamageTracker {
	pub fn new(width: i32, height: i32) -> Self {
		Self {
			damage: VecDeque::new(),
			output_size: (width, height),
			full_damage: true,
			frame_count: 0,
		}
	}

	pub fn set_output_size(&mut self, width: i32, height: i32) {
		if self.output_size != (width, height) {
			self.output_size = (width, height);
			self.full_damage = true;
			self.damage.clear();
		}
	}

	pub fn add_damage(&mut self, rect: Rectangle<i32, Physical>) {
		let rect = Rectangle {
			loc: rect.loc,
			size: rect.size,
		};

		if rect.size.w <= 0 || rect.size.h <= 0 {
			return;
		}

		trace!(damage = ?rect, "Adding damage rectangle");

		for existing in &self.damage {
			if existing.contains_rect(rect) {
				trace!("Damage already covered by existing rectangle");
				return;
			}
		}

		self.damage.retain(|existing| !rect.contains_rect(*existing));

		self.damage.push_back(rect);

		while self.damage.len() > MAX_DAMAGE_RECTANGLES {
			self.full_damage = true;
			self.damage.pop_front();
		}
	}

	pub fn add_full_damage(&mut self) {
		trace!("Adding full damage");
		self.full_damage = true;
		self.damage.clear();
	}

	pub fn damage(&self) -> Option<&VecDeque<Rectangle<i32, Physical>>> {
		if self.full_damage { None } else { Some(&self.damage) }
	}

	pub fn damage_regions(&self) -> Vec<Rectangle<i32, Physical>> {
		if self.full_damage {
			vec![Rectangle::from_size((self.output_size.0, self.output_size.1).into())]
		} else {
			self.damage.iter().copied().collect()
		}
	}

	pub fn clear_damage(&mut self) {
		self.frame_count += 1;
		trace!(frame = self.frame_count, "Clearing damage");
		self.full_damage = false;
		self.damage.clear();
	}

	pub fn has_damage(&self) -> bool {
		self.full_damage || !self.damage.is_empty()
	}

	pub fn output_size(&self) -> (i32, i32) {
		self.output_size
	}

	pub fn frame_count(&self) -> u64 {
		self.frame_count
	}
}

impl Default for DamageTracker {
	fn default() -> Self {
		Self::new(1920, 1080)
	}
}

trait RectangleExt {
	fn contains_rect(&self, other: Self) -> bool;
}

impl RectangleExt for Rectangle<i32, Physical> {
	fn contains_rect(&self, other: Self) -> bool {
		let self_right = self.loc.x + self.size.w;
		let self_bottom = self.loc.y + self.size.h;
		let other_right = other.loc.x + other.size.w;
		let other_bottom = other.loc.y + other.size.h;

		self.loc.x <= other.loc.x && self.loc.y <= other.loc.y && self_right >= other_right && self_bottom >= other_bottom
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_damage_tracker_new() {
		let tracker = DamageTracker::new(1920, 1080);
		assert!(tracker.has_damage());
		assert!(tracker.damage().is_none());
	}

	#[test]
	fn test_damage_tracker_add_damage() {
		let mut tracker = DamageTracker::new(1920, 1080);
		tracker.clear_damage();

		let rect = Rectangle::from_loc_and_size((0, 0), (100, 100));
		tracker.add_damage(rect);

		assert!(tracker.has_damage());
		assert_eq!(tracker.damage().unwrap().len(), 1);
	}

	#[test]
	fn test_damage_tracker_full_damage() {
		let mut tracker = DamageTracker::new(1920, 1080);
		tracker.clear_damage();

		tracker.add_full_damage();
		assert!(tracker.damage().is_none());
	}

	#[test]
	fn test_damage_tracker_merge_covered() {
		let mut tracker = DamageTracker::new(1920, 1080);
		tracker.clear_damage();

		let small = Rectangle::from_loc_and_size((10, 10), (50, 50));
		let large = Rectangle::from_loc_and_size((0, 0), (100, 100));

		tracker.add_damage(small);
		tracker.add_damage(large);

		assert_eq!(tracker.damage().unwrap().len(), 1);
	}
}
