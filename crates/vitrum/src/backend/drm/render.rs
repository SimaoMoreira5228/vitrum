use anyhow::{Result, anyhow};
use smithay::backend::allocator::Fourcc;
use smithay::backend::renderer::element::Kind;
use smithay::backend::renderer::element::surface::{WaylandSurfaceRenderElement, render_elements_from_surface_tree};
use smithay::backend::renderer::element::texture::{TextureBuffer, TextureRenderElement};
use smithay::backend::renderer::gles::GlesRenderer;
use smithay::backend::renderer::utils::draw_render_elements;
use smithay::backend::renderer::{
	Bind, Color32F, ExportMem, Frame, ImportAll, ImportMem, Offscreen, Renderer, Texture, TextureMapping,
};
use smithay::reexports::wayland_server::protocol::wl_surface::WlSurface;
use smithay::render_elements;
use smithay::utils::{Buffer, Physical, Point, Rectangle, Size, Transform};
use smithay::wayland::compositor::{SurfaceAttributes, TraversalAction, with_surface_tree_downward};
use tracing::{debug, info};

use crate::backend::State;
use crate::backend::drm::DrmBackend;

impl DrmBackend {
	pub fn render(&mut self, state: &mut State, frame_time_millis: u32) -> Result<()> {
		let surfaces = state.stacked_surfaces_for_render();
		let mut feedback_crtcs = Vec::new();
		let mut captured_any_output = false;
		for output in &mut self.outputs {
			let frame_flags = Self::frame_flags_for_output(output);
			let mut queued_on_crtc = None;
			if let Some(ref mut compositor) = output.compositor {
				let logical_size: smithay::utils::Size<i32, smithay::utils::Logical> =
					smithay::utils::Size::from((output.mode.size.w, output.mode.size.h));
				state.wallpaper.set_output_size(logical_size.w, logical_size.h);

				if !state.wallpaper.is_solid() {
					if let Err(e) = state.wallpaper.ensure_texture_loaded(&mut self.renderer) {
						tracing::warn!(error = %e, "Failed to load wallpaper texture for DRM render");
					}
				}

				let mut elements: Vec<CustomRenderElement<GlesRenderer>> = Vec::new();

				if !state.wallpaper.is_solid() {
					if let (Some(texture), Some(rect)) =
						(state.wallpaper.current_texture(), state.wallpaper.calculate_rects())
					{
						let dst_rect = Rectangle {
							loc: Point::from((rect.dst[0] as i32, rect.dst[1] as i32)),
							size: Size::from((rect.dst[2] as i32, rect.dst[3] as i32)),
						};
						let buffer =
							TextureBuffer::from_texture(&self.renderer, texture.clone(), 1, Transform::Normal, None);
						let element = TextureRenderElement::from_texture_buffer(
							dst_rect.loc.to_f64().to_physical(1.0),
							&buffer,
							None,
							None,
							None,
							Kind::Unspecified,
						);
						elements.push(CustomRenderElement::Wallpaper(element));
					}
				}

				for (surface, location) in &surfaces {
					let loc = (location.x, location.y);
					let surface_elements =
						render_elements_from_surface_tree(&mut self.renderer, surface, loc, 1.0, 1.0, Kind::Unspecified);
					elements.extend(surface_elements.into_iter().map(CustomRenderElement::Surface));
				}

				let frame_result = compositor
					.render_frame(&mut self.renderer, &elements, state.wallpaper.background_color(), frame_flags)
					.map_err(|e| anyhow!("Failed to render frame for {}: {:?}", output.output.name(), e))?;

				if !captured_any_output && !state.screencopy_sessions.is_empty() {
					Self::capture_output_for_screencopy(&mut self.renderer, state, &elements, output.mode.size);
					captured_any_output = true;
				}

				let should_queue = !frame_result.is_empty || !output.has_queued_initial_frame;
				if should_queue {
					let was_initial_queue = !output.has_queued_initial_frame;
					match compositor.queue_frame(()) {
						Ok(()) => {
							queued_on_crtc = Some(output.crtc);
							output.has_queued_initial_frame = true;
							self.queued_frame_times.insert(output.crtc, frame_time_millis);
							if was_initial_queue {
								info!("Queued initial frame for {}", output.output.name());
							}
						}
						Err(smithay::backend::drm::compositor::FrameError::EmptyFrame) => {
							debug!("Queue frame returned EmptyFrame for {}", output.output.name());
						}
						Err(e) => {
							return Err(anyhow!("Failed to queue frame for {}: {:?}", output.output.name(), e));
						}
					}
				}
			}

			if let Some(crtc) = queued_on_crtc {
				feedback_crtcs.push(crtc);
			}
		}

		let wl_surfaces = surfaces
			.iter()
			.map(|(surface, _)| surface.clone())
			.collect::<Vec<WlSurface>>();
		for crtc in feedback_crtcs {
			self.collect_presentation_feedback(crtc, &wl_surfaces);
		}
		Ok(())
	}

	fn capture_output_for_screencopy(
		renderer: &mut GlesRenderer,
		state: &mut State,
		elements: &[CustomRenderElement<GlesRenderer>],
		size: Size<i32, Physical>,
	) {
		let mut offscreen = match renderer.create_buffer(Fourcc::Argb8888, Size::from((size.w, size.h))) {
			Ok(buffer) => buffer,
			Err(err) => {
				debug!(?err, "Unable to create offscreen capture buffer");
				return;
			}
		};

		let mut fb = match renderer.bind(&mut offscreen) {
			Ok(fb) => fb,
			Err(err) => {
				debug!(?err, "Unable to bind offscreen capture buffer");
				return;
			}
		};

		let mut frame = match renderer.render(&mut fb, size, Transform::Normal) {
			Ok(frame) => frame,
			Err(err) => {
				debug!(?err, "Unable to begin offscreen capture render");
				return;
			}
		};

		let full_damage = vec![Rectangle::from_size(size)];
		let bg = state.wallpaper.background_color();
		if frame.clear(Color32F::new(bg[0], bg[1], bg[2], bg[3]), &full_damage).is_err() {
			return;
		}

		if draw_render_elements(&mut frame, 1.0, elements, &full_damage).is_err() {
			return;
		}

		if frame.finish().is_err() {
			return;
		}
		drop(fb);

		let region = Rectangle::<i32, Buffer>::from_size(Size::from((size.w, size.h)));
		if let Ok(mapping) = renderer.copy_texture(&offscreen, region, Fourcc::Argb8888) {
			if let Ok(bytes) = renderer.map_texture(&mapping) {
				state.last_winit_capture = Some(crate::backend::CapturedFrame {
					width: mapping.width(),
					height: mapping.height(),
					format: TextureMapping::format(&mapping),
					flipped: mapping.flipped(),
					data: bytes.to_vec(),
				});
			}
		}
	}

	pub(super) fn send_frame_callbacks(state: &mut State, frame_time_millis: u32) {
		for (surface, _loc) in state.stacked_surfaces_for_render() {
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
	}
}

render_elements! {
	pub CustomRenderElement<R> where R: ImportAll + ImportMem, R::TextureId: 'static;
	Surface=WaylandSurfaceRenderElement<R>,
	Wallpaper=TextureRenderElement<R::TextureId>,
}
