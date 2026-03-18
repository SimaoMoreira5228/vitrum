use std::collections::HashMap;
use std::path::PathBuf;
use std::time::{Duration, UNIX_EPOCH};

use anyhow::{Context, Result, anyhow, bail};
use smithay::backend::allocator::Fourcc as DrmFourcc;
use smithay::backend::allocator::gbm::{GbmAllocator, GbmBufferFlags, GbmDevice};
use smithay::backend::drm::compositor::{DrmCompositor, FrameFlags};
use smithay::backend::drm::exporter::gbm::GbmFramebufferExporter;
use smithay::backend::drm::{DrmDevice, DrmDeviceFd, DrmEvent, DrmEventMetadata, DrmEventTime, DrmNode, NodeType, Planes};
use smithay::backend::egl::{EGLContext, EGLDevice, EGLDisplay};
use smithay::backend::libinput::LibinputSessionInterface;
use smithay::backend::renderer::gles::GlesRenderer;
use smithay::backend::session::libseat::LibSeatSession;
use smithay::backend::session::{Event as SessionEvent, Session};
use smithay::backend::udev;
use smithay::backend::udev::UdevEvent;
use smithay::output::{Mode, Output, PhysicalProperties};
use smithay::reexports::calloop::{EventLoop, LoopHandle};
use smithay::reexports::drm::Device as DrmRawDeviceTrait;
use smithay::reexports::drm::control::{Device as DrmDeviceTrait, Mode as DrmMode, connector, crtc};
use smithay::reexports::input::Libinput;
use smithay::reexports::wayland_protocols::wp::presentation_time::server::wp_presentation_feedback;
use smithay::utils::DeviceFd;
use smithay::wayland::compositor::with_states;
use smithay::wayland::presentation::{PresentationFeedbackCachedState, PresentationFeedbackCallback, Refresh};
use tracing::{debug, info, warn};

use crate::backend::{Backend, State};

mod input;
mod render;
mod run;

pub use run::run;

type DrmGbmCompositor = DrmCompositor<GbmAllocator<DrmDeviceFd>, GbmFramebufferExporter<DrmDeviceFd>, (), DrmDeviceFd>;

pub struct DrmBackend {
	session: LibSeatSession,

	#[allow(dead_code)]
	libinput: Libinput,

	primary_node: DrmNode,

	drm_device: DrmDevice,

	#[allow(dead_code)]
	gbm_device: GbmDevice<DrmDeviceFd>,

	#[allow(dead_code)]
	egl_display: EGLDisplay,

	_egl_context: EGLContext,

	renderer: GlesRenderer,

	software_renderer: bool,
	driver_is_nvidia: bool,
	queued_frame_times: HashMap<crtc::Handle, u32>,
	pending_presentation_feedback: HashMap<crtc::Handle, Vec<PresentationFeedbackCallback>>,

	outputs: Vec<DrmOutput>,
}

pub struct DrmOutput {
	pub output: Output,
	pub crtc: crtc::Handle,
	pub connector: connector::Handle,
	pub mode: Mode,
	pub drm_mode: DrmMode,
	pub compositor: Option<DrmGbmCompositor>,
	pub disable_direct_scanout: bool,
	pub allow_overlay_scanout: bool,
	pub has_queued_initial_frame: bool,
}

impl DrmBackend {
	pub fn new(event_loop: &EventLoop<State>) -> Result<Self> {
		info!("Initializing DRM backend with GLES");

		let is_tty = unsafe { libc::isatty(0) } != 0;
		if !is_tty {
			bail!("DRM backend must be run from a TTY (Ctrl+Alt+F3)");
		}
		info!("Running from TTY");

		let (mut session, notifier) = LibSeatSession::new().context("Failed to create libseat session")?;
		let seat_name = session.seat();
		info!("LibSeat session on seat: {}", seat_name);

		let mut libinput = Libinput::new_with_udev(LibinputSessionInterface::from(session.clone()));
		libinput
			.udev_assign_seat(&seat_name)
			.map_err(|()| anyhow!("Failed to assign seat to libinput"))?;
		info!("Libinput ready");

		let loop_handle = event_loop.handle();

		let primary_node = Self::find_primary_gpu(&seat_name)?;
		info!("Primary GPU: {:?}", primary_node);

		let (drm_device, gbm_device, egl_display, egl_context, renderer, software_renderer) =
			Self::init_gpu(&mut session, primary_node, &loop_handle)?;
		let driver_is_nvidia = Self::is_nvidia_driver(&drm_device);
		if software_renderer {
			warn!("Software renderer detected; direct scanout optimizations will be disabled");
		}
		if driver_is_nvidia {
			warn!("NVIDIA DRM driver detected; overlay scanout optimizations will be disabled");
		}

		let outputs = Self::scan_outputs(&drm_device)?;
		info!("Found {} output(s)", outputs.len());

		let input_backend = smithay::backend::libinput::LibinputInputBackend::new(libinput.clone());

		if let Err(e) = loop_handle.insert_source(input_backend, |event, _, state| {
			Self::process_input(state, event);
		}) {
			warn!("Input setup warning: {:?}", e);
		}

		if let Err(e) = loop_handle.insert_source(notifier, |event, &mut (), state| match event {
			SessionEvent::PauseSession => {
				info!("Session paused");
				if let Some(Backend::Drm(backend)) = state.backend.as_mut() {
					backend.libinput.suspend();
					backend.drm_device.pause();
				}
			}
			SessionEvent::ActivateSession => {
				info!("Session resumed");
				if let Some(Backend::Drm(backend)) = state.backend.as_mut() {
					if let Err(err) = backend.libinput.resume() {
						warn!("Failed to resume libinput: {:?}", err);
					}
					if let Err(err) = backend.drm_device.activate(false) {
						warn!("Failed to reactivate DRM device: {:?}", err);
					}

					if let Err(err) = backend.reconfigure_outputs() {
						warn!("Failed to reconfigure outputs after session resume: {:?}", err);
					}
				}

				state.mark_redraw();
			}
		}) {
			warn!("Session notifier setup warning: {:?}", e);
		}

		info!("DRM backend initialized successfully");

		Ok(Self {
			session,
			libinput,
			primary_node,
			drm_device,
			gbm_device,
			egl_display,
			_egl_context: egl_context,
			renderer,
			software_renderer,
			driver_is_nvidia,
			queued_frame_times: HashMap::new(),
			pending_presentation_feedback: HashMap::new(),
			outputs,
		})
	}

	fn is_nvidia_driver(drm_device: &DrmDevice) -> bool {
		drm_device
			.get_driver()
			.map(|driver| {
				let name = driver.name().to_string_lossy().to_lowercase();
				let description = driver.description().to_string_lossy().to_lowercase();
				name.contains("nvidia") || description.contains("nvidia")
			})
			.unwrap_or(false)
	}

	fn find_primary_gpu(seat: &str) -> Result<DrmNode> {
		let mut candidates: Vec<PathBuf> = Vec::new();

		if let Ok(override_path) = std::env::var("VITRUM_DRM_DEVICE") {
			let override_path = PathBuf::from(override_path);
			info!("Using VITRUM_DRM_DEVICE override candidate: {:?}", override_path);
			candidates.push(override_path);
		}

		if let Some(primary_path) = udev::primary_gpu(seat).context("Error getting primary GPU")? {
			candidates.push(primary_path);
		}

		for path in udev::all_gpus(seat).context("Error enumerating GPUs")? {
			if !candidates.contains(&path) {
				candidates.push(path);
			}
		}

		for path in candidates {
			match DrmNode::from_path(&path) {
				Ok(node) => {
					info!("Selected GPU candidate: {:?}", path);
					return Ok(node);
				}
				Err(err) => {
					warn!("Failed to use GPU candidate {:?}: {:?}", path, err);
				}
			}
		}

		bail!("No usable DRM GPU node found for seat {}", seat)
	}

	fn refresh_for_output(output: &DrmOutput) -> Refresh {
		if output.mode.refresh <= 0 {
			Refresh::Unknown
		} else {
			let hz = output.mode.refresh as f64 / 1000.0;
			Refresh::fixed(Duration::from_secs_f64(1.0 / hz))
		}
	}

	fn metadata_to_presentation_time(metadata: Option<DrmEventMetadata>) -> Duration {
		match metadata.map(|m| m.time) {
			Some(DrmEventTime::Monotonic(duration)) => duration,
			Some(DrmEventTime::Realtime(system_time)) => system_time
				.duration_since(UNIX_EPOCH)
				.unwrap_or_else(|_| Duration::from_millis(0)),
			None => Duration::from_millis(0),
		}
	}

	fn publish_presentation_feedback(&mut self, crtc: crtc::Handle, metadata: Option<DrmEventMetadata>) {
		let Some(feedbacks) = self.pending_presentation_feedback.remove(&crtc) else {
			return;
		};

		let Some(output) = self.outputs.iter().find(|o| o.crtc == crtc) else {
			return;
		};

		let refresh = Self::refresh_for_output(output);
		let time = Self::metadata_to_presentation_time(metadata);
		let seq = metadata.map(|m| m.sequence as u64).unwrap_or(0);
		let flags = wp_presentation_feedback::Kind::Vsync
			| wp_presentation_feedback::Kind::HwClock
			| wp_presentation_feedback::Kind::HwCompletion;

		for feedback in feedbacks {
			feedback.presented(&output.output, time, refresh, seq, flags);
		}
	}

	fn collect_presentation_feedback(
		&mut self,
		crtc: crtc::Handle,
		surfaces: &[smithay::reexports::wayland_server::protocol::wl_surface::WlSurface],
	) {
		let feedbacks = self.pending_presentation_feedback.entry(crtc).or_default();

		for surface in surfaces {
			let mut surface_feedbacks = with_states(surface, |states| {
				std::mem::take(
					&mut states
						.cached_state
						.get::<PresentationFeedbackCachedState>()
						.current()
						.callbacks,
				)
			});
			feedbacks.append(&mut surface_feedbacks);
		}
	}

	pub fn handle_udev_event(&mut self, event: UdevEvent) -> Result<()> {
		match event {
			UdevEvent::Added { device_id, path } => {
				info!("Udev: device added id={} path={:?}", device_id, path);
				self.reconfigure_outputs()?;
			}
			UdevEvent::Changed { device_id } => {
				info!("Udev: device changed id={}", device_id);
				self.reconfigure_outputs()?;
			}
			UdevEvent::Removed { device_id } => {
				info!("Udev: device removed id={}", device_id);
				self.reconfigure_outputs()?;
			}
		}

		Ok(())
	}

	fn reconfigure_outputs(&mut self) -> Result<()> {
		info!("Reconfiguring DRM outputs after udev event");
		self.outputs = Self::scan_outputs(&self.drm_device)?;
		self.queued_frame_times.clear();
		self.pending_presentation_feedback.clear();
		self.create_surfaces()?;
		Ok(())
	}

	fn init_gpu(
		session: &mut LibSeatSession,
		node: DrmNode,
		loop_handle: &LoopHandle<'_, State>,
	) -> Result<(DrmDevice, GbmDevice<DrmDeviceFd>, EGLDisplay, EGLContext, GlesRenderer, bool)> {
		info!("Initializing GPU: {:?}", node);

		let drm_path = node.dev_path().ok_or_else(|| anyhow!("No device path for node"))?;
		info!("Opening DRM device: {:?}", drm_path);

		let fd = session
			.open(
				&drm_path,
				smithay::reexports::rustix::fs::OFlags::RDWR | smithay::reexports::rustix::fs::OFlags::CLOEXEC,
			)
			.map_err(|e| anyhow!("Failed to open DRM device: {:?}", e))?;
		let device_fd = DrmDeviceFd::new(DeviceFd::from(fd));
		info!("DRM device opened");

		let (drm_device, notifier) = DrmDevice::new(device_fd.clone(), true).context("Failed to create DRM device")?;
		if let Err(e) = loop_handle.insert_source(notifier, move |event, metadata, state| match event {
			DrmEvent::VBlank(crtc) => {
				debug!("DRM vblank event on {:?} ({:?})", node, crtc);
				let mut frame_time_to_send = None;
				let metadata_snapshot = *metadata;
				let mut should_publish_feedback = false;
				if let Some(Backend::Drm(backend)) = state.backend.as_mut() {
					for output in &mut backend.outputs {
						if output.crtc == crtc {
							if let Some(compositor) = output.compositor.as_mut() {
								if let Err(err) = compositor.frame_submitted() {
									warn!("Failed to mark frame submitted for {}: {:?}", output.output.name(), err);
								} else {
									frame_time_to_send = backend.queued_frame_times.remove(&crtc);
									should_publish_feedback = true;
								}
							}
						}
					}
					if should_publish_feedback {
						backend.publish_presentation_feedback(crtc, metadata_snapshot);
					}
				}

				if let Some(frame_time) = frame_time_to_send {
					Self::send_frame_callbacks(state, frame_time);
				}
			}
			DrmEvent::Error(err) => {
				warn!("DRM event error on {:?}: {:?}", node, err);
			}
		}) {
			warn!("DRM notifier setup warning: {:?}", e);
		}
		info!("DRM device created");

		let gbm_device = GbmDevice::new(device_fd).context("Failed to create GBM device")?;
		info!("GBM device created");

		let egl_display = unsafe { EGLDisplay::new(gbm_device.clone()) }.context("Failed to create EGL display")?;
		info!("EGL display created");

		let egl_context = EGLContext::new(&egl_display).context("Failed to create EGL context")?;
		info!("EGL context created");

		let renderer = unsafe {
			let ctx = EGLContext::new(&egl_display).context("Failed to create EGL context for renderer")?;
			GlesRenderer::new(ctx)
		}
		.context("Failed to create GLES renderer")?;
		info!("GLES renderer created");
		let software_renderer = EGLDevice::device_for_display(&egl_display)
			.map(|dev| dev.is_software())
			.unwrap_or(false);

		Ok((drm_device, gbm_device, egl_display, egl_context, renderer, software_renderer))
	}

	fn scan_outputs(drm_device: &DrmDevice) -> Result<Vec<DrmOutput>> {
		info!("Scanning displays...");

		let resources = drm_device.resource_handles().context("Failed to get DRM resources")?;

		let mut outputs = Vec::new();
		let mut claimed_crtcs = Vec::new();

		for connector_handle in resources.connectors() {
			let connector = match drm_device.get_connector(*connector_handle, true) {
				Ok(c) => c,
				Err(e) => {
					warn!("Failed to get connector {:?}: {:?}", connector_handle, e);
					continue;
				}
			};

			if connector.state() != connector::State::Connected {
				continue;
			}

			let name = format!("{:?}-{:?}", connector.interface(), connector.interface_id());

			let mut crtc = None;
			for encoder_handle in connector.encoders() {
				if let Ok(encoder) = drm_device.get_encoder(*encoder_handle) {
					for c in resources.filter_crtcs(encoder.possible_crtcs()) {
						if !claimed_crtcs.contains(&c) {
							crtc = Some(c);
							break;
						}
					}
				}
				if crtc.is_some() {
					break;
				}
			}

			let crtc = match crtc {
				Some(c) => {
					claimed_crtcs.push(c);
					c
				}
				None => {
					warn!("No available CRTC for {}", name);
					continue;
				}
			};

			let drm_mode = match connector.modes().first() {
				Some(m) => *m,
				None => {
					warn!("No modes for {}", name);
					continue;
				}
			};

			let mode = Mode::from(drm_mode);
			info!(
				"Display: {} - {}x{} @ {}Hz",
				name,
				mode.size.w,
				mode.size.h,
				mode.refresh as f32 / 1000.0
			);

			let size = connector.size().unwrap_or((0, 0));
			let physical = PhysicalProperties {
				size: (size.0 as i32, size.1 as i32).into(),
				subpixel: smithay::output::Subpixel::Unknown,
				make: "Unknown".into(),
				model: name.clone().into(),
				serial_number: format!("{}-{}", connector.interface().as_str(), connector.interface_id()),
			};

			let output = Output::new(name.clone(), physical);
			output.set_preferred(mode);
			output.change_current_state(Some(mode), None, None, None);

			outputs.push(DrmOutput {
				output,
				crtc,
				connector: *connector_handle,
				mode,
				drm_mode,
				compositor: None,
				disable_direct_scanout: false,
				allow_overlay_scanout: true,
				has_queued_initial_frame: false,
			});
		}

		Ok(outputs)
	}

	pub fn create_surfaces(&mut self) -> Result<()> {
		info!("Creating rendering surfaces...");

		let disable_scanout_env = std::env::var("VITRUM_DISABLE_DIRECT_SCANOUT").is_ok();
		let render_node = self.primary_node.node_with_type(NodeType::Render).and_then(Result::ok);
		let renderer_formats: Vec<_> = self.renderer.egl_context().dmabuf_render_formats().iter().copied().collect();
		let cursor_size = self.drm_device.cursor_size();
		let gbm_device = self.gbm_device.clone();
		let color_formats = [DrmFourcc::Argb8888, DrmFourcc::Xrgb8888];

		for output in &mut self.outputs {
			info!("Creating surface for {}", output.output.name());
			output.disable_direct_scanout = disable_scanout_env || self.software_renderer;
			output.allow_overlay_scanout = !self.driver_is_nvidia;

			let mut planes: Option<Planes> = match self.drm_device.planes(&output.crtc) {
				Ok(p) => Some(p),
				Err(err) => {
					warn!("Failed to query planes for {}: {:?}", output.output.name(), err);
					None
				}
			};

			if self.driver_is_nvidia {
				if let Some(planes) = planes.as_mut() {
					planes.overlay.clear();
				}
			}

			match self
				.drm_device
				.create_surface(output.crtc, output.drm_mode, &[output.connector])
			{
				Ok(surface) => {
					let allocator =
						GbmAllocator::new(gbm_device.clone(), GbmBufferFlags::RENDERING | GbmBufferFlags::SCANOUT);
					let framebuffer_exporter = GbmFramebufferExporter::new(gbm_device.clone(), render_node.into());

					match DrmCompositor::new(
						&output.output,
						surface,
						planes,
						allocator,
						framebuffer_exporter,
						color_formats,
						renderer_formats.iter().copied(),
						cursor_size,
						Some(gbm_device.clone()),
					) {
						Ok(compositor) => {
							info!("Surface and compositor created for {}", output.output.name());
							output.compositor = Some(compositor);
						}
						Err(e) => {
							warn!("Failed to create compositor for {}: {:?}", output.output.name(), e);
						}
					}
				}
				Err(e) => {
					warn!("Failed to create surface for {}: {:?}", output.output.name(), e);
				}
			}
		}

		let count = self.outputs.iter().filter(|o| o.compositor.is_some()).count();
		info!("Created {} rendering compositor surface(s)", count);

		Ok(())
	}

	fn frame_flags_for_output(output: &DrmOutput) -> FrameFlags {
		if output.disable_direct_scanout {
			return FrameFlags::empty();
		}

		let mut flags = FrameFlags::DEFAULT | FrameFlags::SKIP_CURSOR_ONLY_UPDATES;
		if !output.allow_overlay_scanout {
			flags.remove(FrameFlags::ALLOW_OVERLAY_PLANE_SCANOUT);
		}
		flags
	}

	pub fn outputs(&self) -> &[DrmOutput] {
		&self.outputs
	}

	pub fn renderer(&mut self) -> &mut GlesRenderer {
		&mut self.renderer
	}

	pub fn shutdown(&mut self) -> Result<()> {
		info!("Shutting down DRM backend");
		info!("DRM backend shutdown complete");
		Ok(())
	}
}

impl DrmOutput {
	pub fn output(&self) -> &Output {
		&self.output
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_drm_backend_creation() {
		if unsafe { libc::isatty(0) } != 0 {}
	}

	#[test]
	fn test_tty_detection() {
		let is_tty = unsafe { libc::isatty(0) } != 0;

		info!("Running on TTY: {}", is_tty);
	}
}
