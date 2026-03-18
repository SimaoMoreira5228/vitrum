use smithay::desktop::WindowSurfaceType;
use smithay::reexports::wayland_server::protocol::wl_output::WlOutput;
use smithay::reexports::wayland_server::protocol::wl_surface::WlSurface;
use smithay::utils::{Logical, Point, Rectangle, Size};
use smithay::wayland::compositor::with_states;
use smithay::wayland::shell::wlr_layer::{Layer, LayerSurface as WlrLayerSurface, WlrLayerShellHandler, WlrLayerShellState};
use smithay::wayland::shell::xdg::PopupSurface;
use tracing::{debug, info, warn};

use crate::backend::State;
use crate::output::OutputId;

#[derive(Debug, Clone)]
pub struct LayerSurfaceInfo {
	pub surface: WlrLayerSurface,
	pub layer: Layer,
	pub namespace: String,
	pub output_id: Option<OutputId>,
	pub mapped: bool,
}

impl LayerSurfaceInfo {
	pub fn new(surface: WlrLayerSurface, layer: Layer, namespace: String, output_id: Option<OutputId>) -> Self {
		Self {
			surface,
			layer,
			namespace,
			output_id,
			mapped: false,
		}
	}
}

#[derive(Debug)]
pub struct LayerShellManager {
	pub state: WlrLayerShellState,

	surfaces: Vec<LayerSurfaceInfo>,

	pending_surfaces: Vec<WlSurface>,
}

impl LayerShellManager {
	pub fn new(display_handle: &smithay::reexports::wayland_server::DisplayHandle) -> Self {
		Self {
			state: WlrLayerShellState::new::<State>(display_handle),
			surfaces: Vec::new(),
			pending_surfaces: Vec::new(),
		}
	}

	pub fn add_surface(&mut self, info: LayerSurfaceInfo) {
		debug!(
			namespace = %info.namespace,
			layer = ?info.layer,
			"Adding layer surface"
		);
		self.surfaces.push(info);
	}

	pub fn remove_surface(&mut self, surface: &WlrLayerSurface) {
		let wl_surface = surface.wl_surface();
		if let Some(pos) = self.surfaces.iter().position(|info| info.surface.wl_surface() == wl_surface) {
			debug!(
				namespace = %self.surfaces[pos].namespace,
				"Removing layer surface"
			);
			self.surfaces.remove(pos);
		}
		self.pending_surfaces.retain(|s| s != wl_surface);
	}

	pub fn surfaces(&self) -> &[LayerSurfaceInfo] {
		&self.surfaces
	}

	pub fn surfaces_for_layer(&self, layer: Layer) -> impl Iterator<Item = &LayerSurfaceInfo> {
		self.surfaces.iter().filter(move |info| info.layer == layer)
	}

	pub fn is_layer_surface(&self, surface: &WlSurface) -> bool {
		self.surfaces.iter().any(|info| info.surface.wl_surface() == surface)
	}

	pub fn mark_mapped(&mut self, surface: &WlSurface) {
		if let Some(info) = self.surfaces.iter_mut().find(|info| info.surface.wl_surface() == surface) {
			if !info.mapped {
				debug!(namespace = %info.namespace, "Layer surface mapped");
				info.mapped = true;
			}
		}
	}

	pub fn mark_unmapped(&mut self, surface: &WlSurface) {
		if let Some(info) = self.surfaces.iter_mut().find(|info| info.surface.wl_surface() == surface) {
			if info.mapped {
				debug!(namespace = %info.namespace, "Layer surface unmapped");
				info.mapped = false;
			}
		}
	}

	pub fn add_pending(&mut self, surface: WlSurface) {
		if !self.pending_surfaces.contains(&surface) {
			self.pending_surfaces.push(surface);
		}
	}

	pub fn configure_pending(&mut self) {
		for surface in &self.pending_surfaces {
			if let Some(info) = self.surfaces.iter().find(|info| info.surface.wl_surface() == surface) {
				debug!(namespace = %info.namespace, "Sending layer configure");
				info.surface.send_configure();
			}
		}
		self.pending_surfaces.clear();
	}

	pub fn get_layer(&self, surface: &WlSurface) -> Option<Layer> {
		self.surfaces
			.iter()
			.find(|info| info.surface.wl_surface() == surface)
			.map(|info| info.layer)
	}

	pub fn mapped_surfaces_ordered(&self) -> Vec<&LayerSurfaceInfo> {
		let mut mapped: Vec<_> = self.surfaces.iter().filter(|s| s.mapped).collect();

		mapped.sort_by_key(|info| match info.layer {
			Layer::Background => 0,
			Layer::Bottom => 1,
			Layer::Top => 2,
			Layer::Overlay => 3,
		});

		mapped
	}
}

impl WlrLayerShellHandler for State {
	fn shell_state(&mut self) -> &mut WlrLayerShellState {
		&mut self.layer_shell_manager.state
	}

	fn new_layer_surface(&mut self, surface: WlrLayerSurface, wl_output: Option<WlOutput>, layer: Layer, namespace: String) {
		info!(
			namespace = %namespace,
			layer = ?layer,
			"New layer surface requested"
		);

		let wl_surface = surface.wl_surface().clone();

		let requested_output_id = wl_output
			.as_ref()
			.and_then(|resource| self.output_manager.output_id_for_wl_output(resource));

		if wl_output.is_some() && requested_output_id.is_none() {
			warn!("Could not resolve requested wl_output to known OutputId; using fallback output selection");
		}

		let output_id = requested_output_id
			.or(self.output_id)
			.or_else(|| self.output_manager.map().primary().map(|o| o.id));

		let info = LayerSurfaceInfo::new(surface.clone(), layer, namespace.clone(), output_id);

		self.layer_shell_manager.add_surface(info);
		self.layer_shell_manager.add_pending(wl_surface.clone());

		self.layer_shell_manager.configure_pending();

		info!(namespace = %namespace, "Layer surface created");
	}

	fn layer_destroyed(&mut self, surface: WlrLayerSurface) {
		let wl_surface = surface.wl_surface();

		if let Some(pos) = self
			.layer_shell_manager
			.surfaces()
			.iter()
			.position(|info| info.surface.wl_surface() == wl_surface)
		{
			let info = &self.layer_shell_manager.surfaces()[pos];
			info!(
				namespace = %info.namespace,
				"Layer surface destroyed"
			);
		}

		self.layer_shell_manager.remove_surface(&surface);

		self.mark_redraw();
	}

	fn new_popup(&mut self, _parent: WlrLayerSurface, popup: PopupSurface) {
		debug!("Layer shell popup created");

		if let Err(err) = popup.send_configure() {
			warn!(error = ?err, "Failed to send layer-shell popup configure");
		}

		self.mark_redraw();
	}
}

smithay::delegate_layer_shell!(State);

pub fn handle_layer_surface_commit(state: &mut State, surface: &WlSurface) -> bool {
	if !state.layer_shell_manager.is_layer_surface(surface) {
		return false;
	}

	let is_mapped = with_states(surface, |surface_states| {
		surface_states
			.cached_state
			.get::<smithay::wayland::compositor::SurfaceAttributes>()
			.current()
			.buffer
			.is_some()
	});

	if is_mapped {
		state.layer_shell_manager.mark_mapped(surface);
	} else {
		state.layer_shell_manager.mark_unmapped(surface);
	}

	state.mark_redraw();

	true
}

fn layer_matches_output(surface_output: Option<OutputId>, output_id: OutputId) -> bool {
	surface_output.is_none() || surface_output == Some(output_id)
}

pub fn get_layer_surfaces_for_output(state: &State, output_id: OutputId) -> Vec<(Layer, WlrLayerSurface)> {
	state
		.layer_shell_manager
		.mapped_surfaces_ordered()
		.into_iter()
		.filter(|info| layer_matches_output(info.output_id, output_id))
		.map(|info| (info.layer, info.surface.clone()))
		.collect()
}

pub fn layer_surface_at_position(state: &State, position: smithay::utils::Point<f64, Logical>) -> Option<&LayerSurfaceInfo> {
	state
		.layer_shell_manager
		.mapped_surfaces_ordered()
		.into_iter()
		.rev()
		.find(|info| {
			smithay::desktop::utils::under_from_surface_tree(
				info.surface.wl_surface(),
				position,
				(0, 0),
				WindowSurfaceType::ALL,
			)
			.is_some()
		})
}

pub fn layer_pointer_focus(
	state: &State,
	pointer_location: smithay::utils::Point<f64, Logical>,
) -> Option<(
	smithay::reexports::wayland_server::protocol::wl_surface::WlSurface,
	smithay::utils::Point<f64, smithay::utils::Logical>,
)> {
	state
		.layer_shell_manager
		.mapped_surfaces_ordered()
		.into_iter()
		.rev()
		.find_map(|info| {
			smithay::desktop::utils::under_from_surface_tree(
				info.surface.wl_surface(),
				pointer_location,
				(0, 0),
				WindowSurfaceType::ALL,
			)
			.map(|(wl_surface, surface_loc)| (wl_surface, surface_loc.to_f64()))
		})
}

pub fn compute_layer_surface_position(info: &LayerSurfaceInfo, output_size: Size<i32, Logical>) -> Point<i32, Logical> {
	let cached_state = with_states(info.surface.wl_surface(), |states| {
		states
			.cached_state
			.get::<smithay::wayland::shell::wlr_layer::LayerSurfaceCachedState>()
			.current()
			.clone()
	});

	let anchor = cached_state.anchor;
	let size = cached_state.size;
	let margin = cached_state.margin;

	let w = if size.w > 0 { size.w } else { output_size.w };
	let h = if size.h > 0 { size.h } else { output_size.h };

	let (x, y) = if anchor.contains(smithay::wayland::shell::wlr_layer::Anchor::TOP) {
		let y = margin.top;
		if anchor.contains(smithay::wayland::shell::wlr_layer::Anchor::LEFT) {
			(margin.left, y)
		} else if anchor.contains(smithay::wayland::shell::wlr_layer::Anchor::RIGHT) {
			(output_size.w - w - margin.right, y)
		} else {
			((output_size.w - w) / 2, y)
		}
	} else if anchor.contains(smithay::wayland::shell::wlr_layer::Anchor::BOTTOM) {
		let y = output_size.h - h - margin.bottom;
		if anchor.contains(smithay::wayland::shell::wlr_layer::Anchor::LEFT) {
			(margin.left, y)
		} else if anchor.contains(smithay::wayland::shell::wlr_layer::Anchor::RIGHT) {
			(output_size.w - w - margin.right, y)
		} else {
			((output_size.w - w) / 2, y)
		}
	} else if anchor.contains(smithay::wayland::shell::wlr_layer::Anchor::LEFT) {
		let x = margin.left;
		if anchor.contains(smithay::wayland::shell::wlr_layer::Anchor::TOP) {
			(x, margin.top)
		} else if anchor.contains(smithay::wayland::shell::wlr_layer::Anchor::BOTTOM) {
			(x, output_size.h - h - margin.bottom)
		} else {
			(x, (output_size.h - h) / 2)
		}
	} else if anchor.contains(smithay::wayland::shell::wlr_layer::Anchor::RIGHT) {
		let x = output_size.w - w - margin.right;
		if anchor.contains(smithay::wayland::shell::wlr_layer::Anchor::TOP) {
			(x, margin.top)
		} else if anchor.contains(smithay::wayland::shell::wlr_layer::Anchor::BOTTOM) {
			(x, output_size.h - h - margin.bottom)
		} else {
			(x, (output_size.h - h) / 2)
		}
	} else {
		((output_size.w - w) / 2, (output_size.h - h) / 2)
	};

	Point::from((x, y))
}

pub fn get_layer_surface_positions(state: &State, output_size: Size<i32, Logical>) -> Vec<(WlSurface, Point<i32, Logical>)> {
	let mut positions = Vec::new();

	for info in state.layer_shell_manager.mapped_surfaces_ordered() {
		if !info.mapped {
			continue;
		}
		let pos = compute_layer_surface_position(info, output_size);
		positions.push((info.surface.wl_surface().clone(), pos));
	}

	positions
}

pub fn get_layer_surfaces_with_positions(
	state: &State,
	output_id: OutputId,
	output_size: Size<i32, Logical>,
) -> Vec<(Layer, WlSurface, Point<i32, Logical>)> {
	let mut results = Vec::new();

	for info in state.layer_shell_manager.mapped_surfaces_ordered() {
		if !info.mapped {
			continue;
		}
		if !layer_matches_output(info.output_id, output_id) {
			continue;
		}
		let pos = compute_layer_surface_position(info, output_size);
		results.push((info.layer, info.surface.wl_surface().clone(), pos));
	}

	results
}

pub fn compute_tiling_area(state: &State, output_size: Size<i32, Logical>) -> Rectangle<i32, Logical> {
	let mut area = Rectangle::from_size(output_size);

	for info in state.layer_shell_manager.mapped_surfaces_ordered() {
		if !info.mapped {
			continue;
		}

		let cached_state = with_states(info.surface.wl_surface(), |states| {
			states
				.cached_state
				.get::<smithay::wayland::shell::wlr_layer::LayerSurfaceCachedState>()
				.current()
				.clone()
		});

		let exclusive = cached_state.exclusive_zone;
		let anchor = cached_state.anchor;
		let size = cached_state.size;

		let exclusive_pixels = match exclusive {
			smithay::wayland::shell::wlr_layer::ExclusiveZone::Exclusive(val) => val,
			_ => continue,
		};

		if exclusive_pixels <= 0 {
			continue;
		}

		if anchor.contains(smithay::wayland::shell::wlr_layer::Anchor::TOP) {
			let h = if size.h > 0 { size.h } else { exclusive_pixels as i32 };
			area.loc.y += h;
			area.size.h -= h;
		} else if anchor.contains(smithay::wayland::shell::wlr_layer::Anchor::BOTTOM) {
			let h = if size.h > 0 { size.h } else { exclusive_pixels as i32 };
			area.size.h -= h;
		} else if anchor.contains(smithay::wayland::shell::wlr_layer::Anchor::LEFT) {
			let w = if size.w > 0 { size.w } else { exclusive_pixels as i32 };
			area.loc.x += w;
			area.size.w -= w;
		} else if anchor.contains(smithay::wayland::shell::wlr_layer::Anchor::RIGHT) {
			let w = if size.w > 0 { size.w } else { exclusive_pixels as i32 };
			area.size.w -= w;
		}
	}

	if area.size.w < 0 {
		area.size.w = 0;
	}
	if area.size.h < 0 {
		area.size.h = 0;
	}

	area
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_layer_ordering() {
		let _ = Layer::Background;
		let _ = Layer::Bottom;
		let _ = Layer::Top;
		let _ = Layer::Overlay;
	}

	#[test]
	fn test_layer_output_matcher() {
		let output = OutputId(7);

		assert!(layer_matches_output(None, output));
		assert!(layer_matches_output(Some(output), output));
		assert!(!layer_matches_output(Some(OutputId(8)), output));
	}
}
