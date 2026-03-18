use anyhow::{Result, anyhow};
use smithay::backend::allocator::Fourcc;
use smithay::backend::renderer::element::Kind;
use smithay::backend::renderer::element::surface::{WaylandSurfaceRenderElement, render_elements_from_surface_tree};
use smithay::backend::renderer::gles::{GlesFrame, GlesRenderer};
use smithay::backend::renderer::utils::draw_render_elements;
use smithay::backend::renderer::{Color32F, ExportMem, Frame, Renderer, Texture, TextureMapping};
use smithay::utils::{Buffer, Point, Rectangle, Size, Transform};
use smithay::wayland::compositor::{SurfaceAttributes, TraversalAction, with_surface_tree_downward};

use crate::backend::State;

fn render_wallpaper(
	frame: &mut GlesFrame<'_, '_>,
	wayland_state: &mut State,
	damage: &[Rectangle<i32, smithay::utils::Physical>],
) -> Result<()> {
	if wayland_state.wallpaper.update_slideshow() {
		wayland_state.mark_redraw();
	}

	let size = wayland_state.output_size;
	wayland_state.wallpaper.set_output_size(size.w, size.h);

	if wayland_state.wallpaper.is_solid() {
		let color = wayland_state.wallpaper.background_color();
		frame
			.clear(Color32F::new(color[0], color[1], color[2], color[3]), damage)
			.map_err(|err| anyhow!("failed to clear frame: {err}"))?;
		return Ok(());
	}

	let texture_rect = match wayland_state.wallpaper.calculate_rects() {
		Some(rect) => rect,
		None => {
			let color = wayland_state.wallpaper.background_color();
			frame
				.clear(Color32F::new(color[0], color[1], color[2], color[3]), damage)
				.map_err(|err| anyhow!("failed to clear frame: {err}"))?;
			return Ok(());
		}
	};

	let texture = wayland_state
		.wallpaper
		.current_texture()
		.ok_or_else(|| anyhow!("Wallpaper texture not found"))?;

	let color = wayland_state.wallpaper.background_color();
	frame
		.clear(Color32F::new(color[0], color[1], color[2], color[3]), damage)
		.map_err(|err| anyhow!("failed to clear frame: {err}"))?;

	render_wallpaper_texture(frame, texture, &texture_rect, damage)?;

	Ok(())
}

fn render_wallpaper_texture(
	frame: &mut GlesFrame<'_, '_>,
	texture: &smithay::backend::renderer::gles::GlesTexture,
	rect: &crate::wallpaper::TextureRect,
	damage: &[Rectangle<i32, smithay::utils::Physical>],
) -> Result<()> {
	let dst_x = rect.dst[0];
	let dst_y = rect.dst[1];
	let dst_w = rect.dst[2];
	let dst_h = rect.dst[3];

	let src_rect: Rectangle<f64, Buffer> = Rectangle {
		loc: Point::from((rect.src[0] as f64, rect.src[1] as f64)),
		size: Size::from((rect.src[2] as f64, rect.src[3] as f64)),
	};

	let dst_rect = Rectangle {
		loc: Point::from((dst_x as i32, dst_y as i32)),
		size: Size::from((dst_w as i32, dst_h as i32)),
	};

	frame
		.render_texture_from_to(
			texture,
			src_rect,
			dst_rect,
			damage,
			&[],
			smithay::utils::Transform::Normal,
			1.0,
			None,
			&[],
		)
		.map_err(|e| anyhow!("Failed to render wallpaper: {e}"))?;

	Ok(())
}

pub fn redraw(
	backend: &mut smithay::backend::winit::WinitGraphicsBackend<GlesRenderer>,
	wayland_state: &mut State,
	frame_time_millis: u32,
) -> Result<()> {
	let size = backend.window_size();

	wayland_state.damage_tracker.set_output_size(size.w, size.h);

	if !wayland_state.damage_tracker.has_damage() {
		return Ok(());
	}

	let damage_regions = wayland_state.damage_tracker.damage_regions();
	let damage: Vec<_> = damage_regions.iter().copied().collect();

	let output_size = wayland_state.output_size;
	wayland_state.wallpaper.set_output_size(output_size.w, output_size.h);
	wayland_state.wallpaper.update_slideshow();

	let surfaces = wayland_state.stacked_surfaces_for_render();
	let active_workspace = wayland_state.active_workspace_id();
	let active_window_count = wayland_state
		.windows
		.values()
		.filter(|window| window.workspace == active_workspace && window.is_alive())
		.count();

	tracing::debug!(
		active_workspace,
		active_window_count,
		render_surface_count = surfaces.len(),
		"Prepared stacked toplevel render list"
	);

	if active_window_count > 0 && surfaces.is_empty() {
		tracing::warn!(
			active_workspace,
			active_window_count,
			"Active workspace has windows but no render surfaces were selected"
		);
	}

	{
		let (renderer, mut framebuffer) = backend
			.bind()
			.map_err(|err| anyhow!("failed to bind winit framebuffer: {err}"))?;

		if !wayland_state.wallpaper.is_solid() {
			if let Err(e) = wayland_state.wallpaper.ensure_texture_loaded(renderer) {
				tracing::warn!(error = %e, "Failed to load wallpaper texture");
			}
		}

		let mut elements: Vec<WaylandSurfaceRenderElement<GlesRenderer>> = Vec::new();
		for (surface, location) in &surfaces {
			let window_id = wayland_state.window_for_surface(surface);
			let loc = (location.x, location.y);
			let mut surface_elements =
				render_elements_from_surface_tree(renderer, surface, loc, 1.0, 1.0, Kind::Unspecified);

			if surface_elements.is_empty() {
				tracing::warn!(
					window_id = ?window_id,
					active_workspace,
					"Surface produced no render elements (possibly unmapped/no buffer yet)"
				);
			} else {
				tracing::debug!(
					window_id = ?window_id,
					element_count = surface_elements.len(),
					"Surface produced render elements"
				);
			}

			elements.append(&mut surface_elements);
		}

		let mut frame = renderer
			.render(&mut framebuffer, size, Transform::Flipped180)
			.map_err(|err| anyhow!("failed to begin render pass: {err}"))?;

		render_wallpaper(&mut frame, wayland_state, &damage[..])
			.map_err(|err| anyhow!("failed to render wallpaper: {err}"))?;

		draw_render_elements(&mut frame, 1.0, &elements, &damage[..])
			.map_err(|err| anyhow!("failed to draw wayland surfaces: {err}"))?;

		let _sync = frame.finish().map_err(|err| anyhow!("failed to finish frame: {err}"))?;

		let region = Rectangle::<i32, Buffer>::from_size(Size::from((size.w, size.h)));
		if let Ok(mapping) = renderer.copy_framebuffer(&framebuffer, region, Fourcc::Argb8888) {
			if let Ok(bytes) = renderer.map_texture(&mapping) {
				wayland_state.last_winit_capture = Some(crate::backend::CapturedFrame {
					width: mapping.width(),
					height: mapping.height(),
					format: TextureMapping::format(&mapping),
					flipped: mapping.flipped(),
					data: bytes.to_vec(),
				});
			}
		}
	}

	for (surface, _loc) in wayland_state.stacked_surfaces_for_render() {
		with_surface_tree_downward(
			&surface,
			(),
			|_, _, &()| TraversalAction::DoChildren(()),
			|_surface, states, &()| {
				for callback in states
					.cached_state
					.get::<SurfaceAttributes>()
					.current()
					.frame_callbacks
					.drain(..)
				{
					callback.done(frame_time_millis);
				}
			},
			|_, _, &()| true,
		);
	}

	backend
		.submit(Some(&damage[..]))
		.map_err(|err| anyhow!("failed to submit winit frame: {err}"))?;

	wayland_state.damage_tracker.clear_damage();

	Ok(())
}
