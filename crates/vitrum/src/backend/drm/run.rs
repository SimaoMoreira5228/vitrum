use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result, anyhow};
use smithay::backend::session::Session;
use smithay::backend::udev::UdevBackend;
use smithay::reexports::calloop::generic::Generic;
use smithay::reexports::calloop::{EventLoop, Interest, Mode as CalloopMode, PostAction};
use smithay::wayland::socket::ListeningSocketSource;
use tracing::{error, info, warn};

use crate::backend::drm::DrmBackend;
use crate::backend::{Backend, ClientState, State};

pub fn run(max_fps: Option<u16>) -> Result<()> {
	info!("Starting DRM backend with max_fps: {:?}", max_fps);

	let event_loop: EventLoop<State> = EventLoop::try_new().context("Failed to create event loop")?;
	let loop_handle = event_loop.handle().clone();

	use crate::wayland_runtime::WaylandRuntime;
	let wayland = WaylandRuntime::new()?;
	info!("wayland runtime initialized");

	let mut state = State::new(
		crate::config::Config::load()?,
		event_loop.handle(),
		wayland.display_handle.clone(),
	)?;

	info!("Initializing XWayland...");
	match smithay::xwayland::XWayland::spawn(
		&state.display_handle,
		None,
		std::iter::empty::<(&str, &str)>(),
		true,
		std::process::Stdio::null(),
		std::process::Stdio::null(),
		|data| {
			data.insert_if_missing(ClientState::default);
		},
	) {
		Ok((xwayland, client)) => {
			info!("XWayland spawned successfully");
			state.xwayland_client = Some(client.clone());
			let dh = state.display_handle.clone();
			let wm_handle = loop_handle.clone();
			if let Err(err) = loop_handle.insert_source(xwayland, move |event, _, state: &mut State| match event {
				smithay::xwayland::XWaylandEvent::Ready {
					x11_socket,
					display_number,
				} => {
					info!(display = display_number, "XWayland ready, starting X11 WM");
					state.session_env.set_xwayland_display(display_number);
					crate::launcher::set_session_env(state.session_env.child_env());

					inject_xrdb(display_number, &state.config);

					match smithay::xwayland::xwm::X11Wm::start_wm(wm_handle.clone(), &dh, x11_socket, client.clone()) {
						Ok(wm) => {
							state.xwm = Some(wm);
							info!("X11 Window Manager started");
						}
						Err(err) => {
							warn!(?err, "Failed to start X11 WM");
						}
					}
				}
				smithay::xwayland::XWaylandEvent::Error => {
					warn!("XWayland crashed on startup");
				}
			}) {
				warn!(error = %err, "Failed to insert XWayland source into event loop");
			}
		}
		Err(err) => {
			warn!(error = %err, "Failed to spawn XWayland");
		}
	}

	run_drm_backend(event_loop, wayland, state, max_fps)
}

pub fn run_drm_backend(
	mut event_loop: EventLoop<State>,
	wayland: crate::wayland_runtime::WaylandRuntime,
	mut state: State,
	max_fps: Option<u16>,
) -> Result<()> {
	info!("Starting DRM backend");

	let mut backend = DrmBackend::new(&event_loop)?;

	let config = state.config.clone();
	crate::config::apply_config(&config, &mut state);

	backend.create_surfaces()?;

	info!("Outputs: {}", backend.outputs().len());
	for output in backend.outputs() {
		let logical_size = smithay::utils::Size::from((output.mode.size.w, output.mode.size.h));
		let output_id = state.ensure_output_registered(output.output.clone(), logical_size, Some(output.mode.refresh));
		if state.output_id.is_none() {
			state.output_id = Some(output_id);
			state.output_size = logical_size;
		}

		let surface_status = if output.compositor.is_some() {
			"surface ready"
		} else {
			"no surface"
		};
		info!(
			"  - {} ({}x{} @ {}Hz) - {}",
			output.output.name(),
			output.mode.size.w,
			output.mode.size.h,
			output.mode.refresh as f32 / 1000.0,
			surface_status
		);
	}

	state.backend = Some(Backend::Drm(backend));
	state.mark_redraw();
	if let Some(Backend::Drm(mut backend)) = state.backend.take() {
		let frame_time = state.start_time.elapsed().as_millis() as u32;
		if let Err(err) = backend.render(&mut state, frame_time) {
			warn!("Initial DRM render failed: {:?}", err);
		} else {
			info!("Initial DRM render submitted");
		}
		state.backend = Some(Backend::Drm(backend));
	}

	if let Some(Backend::Drm(ref backend)) = state.backend {
		let seat_name = backend.session.seat();
		match UdevBackend::new(&seat_name) {
			Ok(udev_backend) => {
				event_loop
					.handle()
					.insert_source(udev_backend, |event, _, state| {
						let backend_opt = state.backend.take();
						if let Some(Backend::Drm(mut backend)) = backend_opt {
							if let Err(err) = backend.handle_udev_event(event) {
								warn!("Failed to handle udev event: {:?}", err);
							}
							state.backend = Some(Backend::Drm(backend));
							state.mark_redraw();
						}
					})
					.map_err(|err| anyhow!("Failed to insert udev source: {:?}", err))?;
				info!("Udev backend source installed for live DRM hotplug handling");
			}
			Err(err) => {
				warn!("Failed to initialize udev backend source: {:?}", err);
			}
		}
	}

	let socket_source = ListeningSocketSource::new_auto().context("Failed to bind wayland socket")?;
	let wayland_socket = socket_source.socket_name().to_string_lossy().to_string();
	crate::launcher::set_wayland_display_socket(wayland_socket.clone());
	state.session_env.set_wayland_socket(&wayland_socket);
	crate::launcher::set_session_env(state.session_env.child_env());
	info!(socket = %wayland_socket, "wayland socket ready");

	info!("Initializing session services...");
	crate::session_services::start_session_services(&state.config);

	info!("Initializing autostart...");
	crate::autostart::init_autostart(state.config.autostart.clone(), state.config.disable_xdg_autostart);

	event_loop
		.handle()
		.insert_source(socket_source, |stream, _, state| {
			if let Err(err) = state.display_handle.insert_client(stream, Arc::new(ClientState::default())) {
				warn!(error = %err, "failed to insert wayland client");
			}
		})
		.context("Failed to insert listening socket source")?;

	let display_source = Generic::new(wayland.display, Interest::READ, CalloopMode::Level);
	event_loop
		.handle()
		.insert_source(display_source, |_, display, state| {
			if let Err(err) = unsafe { display.get_mut().dispatch_clients(state) } {
				warn!(error = %err, "wayland client dispatch failed");
			}
			Ok::<_, std::io::Error>(PostAction::Continue)
		})
		.context("Failed to insert display source")?;

	info!("Entering main loop...");
	let mut last_render = std::time::Instant::now();
	let frame_interval = max_fps
		.filter(|fps| *fps > 0)
		.map(|fps| Duration::from_secs_f64(1.0 / f64::from(fps)))
		.unwrap_or_else(|| Duration::from_millis(16));
	let mut config_channel_disconnected = false;
	let mut consecutive_render_errors: u32 = 0;
	let max_consecutive_render_errors: u32 = 3;

	loop {
		event_loop
			.dispatch(frame_interval, &mut state)
			.context("DRM event loop dispatch failed")?;

		if let Err(err) = state.display_handle.flush_clients() {
			warn!(error = %err, "failed to flush wayland clients");
		}

		let ipc_receiver = state.ipc_receiver.take();
		if let Some(receiver) = ipc_receiver {
			crate::ipc_handler::process_ipc_messages(&receiver, &mut state);
			state.ipc_receiver = Some(receiver);
		}

		if !config_channel_disconnected {
			loop {
				match state.config_receiver.try_recv() {
					Ok(crate::config::ConfigEvent::Reloaded) => match crate::config::Config::load() {
						Ok(new_config) => {
							info!("Config file changed, applying live reload");
							crate::config::apply_config(&new_config, &mut state);
							state.config = new_config;
							state.mark_redraw();
						}
						Err(err) => {
							warn!("Failed to reload config after change: {:?}", err);
						}
					},
					Ok(crate::config::ConfigEvent::Error(err)) => {
						warn!("Config watcher error: {}", err);
					}
					Err(std::sync::mpsc::TryRecvError::Empty) => break,
					Err(std::sync::mpsc::TryRecvError::Disconnected) => {
						warn!("Config watcher channel disconnected");
						config_channel_disconnected = true;
						break;
					}
				}
			}
		}

		if state.wallpaper.update_slideshow() {
			state.mark_redraw();
		}

		state.process_output_changes();

		let now = std::time::Instant::now();
		let redraw_requested = state.take_redraw_request();
		let frame_due = now.duration_since(last_render) >= frame_interval;
		let should_render = redraw_requested && frame_due;
		if redraw_requested && !frame_due {
			state.mark_redraw();
		}
		if should_render {
			let backend_opt = state.backend.take();
			if let Some(Backend::Drm(mut backend)) = backend_opt {
				let frame_time = state.start_time.elapsed().as_millis() as u32;
				if let Err(e) = backend.render(&mut state, frame_time) {
					consecutive_render_errors += 1;
					if consecutive_render_errors >= max_consecutive_render_errors {
						error!(
							errors = consecutive_render_errors,
							error = %e,
							"Too many consecutive render errors, reconfiguring outputs"
						);
						if let Err(err) = backend.reconfigure_outputs() {
							error!("Failed to reconfigure outputs during recovery: {:?}", err);
						}
						consecutive_render_errors = 0;
					} else {
						warn!(
							error = %e,
							count = consecutive_render_errors,
							"Render error (recovering)"
						);
					}
				} else {
					consecutive_render_errors = 0;
				}
				state.backend = Some(Backend::Drm(backend));
			}
			last_render = now;
		}
	}
}

fn inject_xrdb(display_number: u32, config: &vitrum_config::Config) {
	use std::io::Write;
	use std::process::{Command, Stdio};

	let resources = vitrum_theme::ThemeState::xrdb_resources(config);

	match Command::new("xrdb")
		.arg("-merge")
		.env("DISPLAY", format!(":{}", display_number))
		.stdin(Stdio::piped())
		.stdout(Stdio::null())
		.stderr(Stdio::null())
		.spawn()
	{
		Ok(mut child) => {
			if let Some(mut stdin) = child.stdin.take() {
				if let Err(e) = stdin.write_all(resources.as_bytes()) {
					warn!("Failed to write xrdb resources: {}", e);
				}
			}
			match child.wait() {
				Ok(status) if status.success() => {
					info!("xrdb resources injected to XWayland :{}", display_number);
				}
				Ok(status) => warn!("xrdb exited with status: {}", status),
				Err(e) => warn!("Failed to wait on xrdb: {}", e),
			}
		}
		Err(e) => {
			if e.kind() == std::io::ErrorKind::NotFound {
				info!("xrdb not found, skipping XWayland font/cursor config");
			} else {
				warn!("Failed to run xrdb: {}", e);
			}
		}
	}
}
