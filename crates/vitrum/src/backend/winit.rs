use std::sync::Arc;
use std::time::Duration;

use anyhow::{Result, anyhow};
use calloop::timer::{TimeoutAction, Timer};
use smithay::backend::input::{
	AbsolutePositionEvent, ButtonState, Event, InputEvent, KeyState, KeyboardKeyEvent, PointerAxisEvent, PointerButtonEvent,
	PointerMotionAbsoluteEvent,
};
use smithay::backend::renderer::gles::GlesRenderer;
use smithay::backend::winit::{self, WinitEvent};
use smithay::input::pointer::{AxisFrame, ButtonEvent, MotionEvent};
use smithay::output::{Output, PhysicalProperties, Subpixel};
use smithay::reexports::calloop::generic::Generic;
use smithay::reexports::calloop::{Interest, Mode, PostAction};
use smithay::utils::Serial;
use smithay::wayland::socket::ListeningSocketSource;
use tracing::{debug, info, warn};

use crate::backend::{ClientState, State};
use crate::wayland_runtime::WaylandRuntime;

pub fn run(max_fps: Option<u16>) -> Result<()> {
	info!("initializing smithay winit backend");

	let (_config_manager, config, _config_sender, _config_receiver) =
		crate::config::ConfigManager::new().map_err(|e| anyhow!("failed to load config: {e}"))?;
	info!("configuration loaded");

	let (backend, winit_source) =
		winit::init::<GlesRenderer>().map_err(|err| anyhow!("failed to initialize smithay winit backend: {:?}", err))?;

	let initial_size = backend.window_size();
	info!(size = ?initial_size, "smithay winit backend initialized");

	let initial_output_size: smithay::utils::Size<i32, smithay::utils::Logical> =
		smithay::utils::Size::from((initial_size.w, initial_size.h));

	let mut event_loop: calloop::EventLoop<State> =
		calloop::EventLoop::try_new().map_err(|err| anyhow!("failed to create calloop event loop: {:?}", err))?;

	let handle = event_loop.handle();

	let wayland = WaylandRuntime::new()?;
	info!("wayland runtime initialized");

	handle
		.insert_source(winit_source, |event, _, state| {
			handle_winit_event(event, state);
		})
		.map_err(|err| anyhow!("failed to insert winit source: {:?}", err))?;

	let socket_source =
		ListeningSocketSource::new_auto().map_err(|err| anyhow!("failed to bind wayland socket: {:?}", err))?;
	let wayland_socket = socket_source.socket_name().to_string_lossy().to_string();
	crate::launcher::set_wayland_display_socket(wayland_socket.clone());
	info!(socket = %wayland_socket, "wayland socket ready");

	handle
		.insert_source(socket_source, |stream, _, state| {
			if let Err(err) = state.display_handle.insert_client(stream, Arc::new(ClientState::default())) {
				warn!(error = %err, "failed to insert wayland client");
			}
		})
		.map_err(|err| anyhow!("failed to insert socket source: {:?}", err))?;

	let loop_handle = handle.clone();
	let mut state = wayland.create_state(config.clone(), handle)?;

	state.session_env.set_wayland_socket(&wayland_socket);
	crate::launcher::set_session_env(state.session_env.child_env());

	let display_source = Generic::new(wayland.display, Interest::READ, Mode::Level);

	let _display_token = loop_handle
		.insert_source(display_source, |_, display, state| {
			if let Err(err) = unsafe { display.get_mut().dispatch_clients(state) } {
				warn!(error = %err, "wayland client dispatch failed");
			}
			state.prune_windows();
			Ok::<_, std::io::Error>(PostAction::Continue)
		})
		.map_err(|err| anyhow!("failed to insert display source: {:?}", err))?;

	let config = state.config.clone();
	crate::config::apply_config(&config, &mut state);

	let smithay_output = Output::new(
		"winit-0".to_string(),
		PhysicalProperties {
			size: (0, 0).into(),
			subpixel: Subpixel::Unknown,
			make: "Vitrum".into(),
			model: "Winit".into(),
			serial_number: "virtual-winit-0".into(),
		},
	);

	let output_id = state.ensure_output_registered(smithay_output, initial_output_size, None);
	state.output_id = Some(output_id);
	state.output_size = initial_output_size;
	info!(output_id = ?output_id, "Created initial output for winit window");

	info!("Initializing session services...");
	crate::session_services::start_session_services(&state.config);

	info!("Initializing autostart...");
	crate::autostart::init_autostart(state.config.autostart.clone(), state.config.disable_xdg_autostart);

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

	state.backend = Some(crate::backend::Backend::Winit(backend));

	if let Some(crate::backend::Backend::Winit(ref mut be)) = state.backend {
		be.window().request_redraw();
	}

	let event_loop_handle = event_loop.handle();

	let frame_interval = max_fps
		.filter(|fps| *fps > 0)
		.map(|fps| Duration::from_secs_f64(1.0 / f64::from(fps)));
	state.frame_interval = frame_interval;

	if let Some(fps) = max_fps {
		info!(max_fps = fps, "winit frame cap configured");
	} else {
		info!("winit frame cap disabled");
	}

	let mut config_channel_disconnected = false;

	event_loop
		.run(None, &mut state, move |state| {
			if state.should_stop {
				return;
			}

			if let Err(err) = state.display_handle.flush_clients() {
				warn!(error = %err, "failed to flush wayland clients");
			}

			let ipc_receiver = state.ipc_receiver.take();
			if let Some(receiver) = ipc_receiver {
				crate::ipc_handler::process_ipc_messages(&receiver, state);
				state.ipc_receiver = Some(receiver);
			}

			if !config_channel_disconnected {
				loop {
					match state.config_receiver.try_recv() {
						Ok(crate::config::ConfigEvent::Reloaded) => {
							if let Err(err) = crate::config::reload_runtime_config(state) {
								warn!(error = %err, "failed to reload config after change");
							} else {
								info!("config file changed, applied live reload");
								state.mark_redraw();
							}
						}
						Ok(crate::config::ConfigEvent::Error(err)) => {
							warn!(error = %err, "config watcher error");
						}
						Err(std::sync::mpsc::TryRecvError::Empty) => break,
						Err(std::sync::mpsc::TryRecvError::Disconnected) => {
							warn!("config watcher channel disconnected");
							config_channel_disconnected = true;
							break;
						}
					}
				}
			}

			state.process_output_changes();

			if state.take_redraw_request() && state.frame_timer_token.is_none() {
				if let Some(interval) = state.frame_interval {
					let timer = Timer::from_duration(interval);
					let token = event_loop_handle
						.insert_source(timer, |_, _, state| {
							let frame_time = state.start_time.elapsed().as_millis() as u32;
							if let Err(err) = state.redraw(frame_time) {
								warn!(error = %err, "redraw failed in frame timer");
							}
							state.frame_timer_token = None;
							if let Err(err) = state.display_handle.flush_clients() {
								warn!(error = %err, "failed to flush clients in frame timer");
							}
							if let Some(crate::backend::Backend::Winit(ref mut be)) = state.backend {
								be.window().request_redraw();
							}
							TimeoutAction::Drop
						})
						.expect("failed to insert frame timer");
					state.frame_timer_token = Some(token);
				} else {
					if let Some(crate::backend::Backend::Winit(ref mut be)) = state.backend {
						be.window().request_redraw();
					}
				}
			}
		})
		.map_err(|err| anyhow!("event loop error: {:?}", err))?;

	warn!("winit backend loop exited");
	Ok(())
}

fn handle_winit_event(event: WinitEvent, state: &mut State) {
	match event {
		WinitEvent::Resized { size, scale_factor } => {
			debug!(?size, scale_factor, "winit surface resized");
			state.window_size = Some(size);

			let logical_size = smithay::utils::Size::from((size.w, size.h));
			state.output_size = logical_size;

			if let Some(output_id) = state.output_id {
				state.sync_output_state(output_id, logical_size, scale_factor, None);
			}

			state.apply_layout();

			let frame_time = state.start_time.elapsed().as_millis() as u32;
			if let Err(err) = state.redraw(frame_time) {
				warn!(error = %err, "redraw failed after resize");
			}
		}
		WinitEvent::Redraw => {
			let frame_time = state.start_time.elapsed().as_millis() as u32;
			if let Err(err) = state.redraw(frame_time) {
				warn!(error = %err, "redraw failed");
			}
		}
		WinitEvent::Input(input_event) => {
			process_input(state, input_event);
		}
		WinitEvent::CloseRequested => {
			info!("close requested from winit host window");
			state.should_stop = true;
		}
		WinitEvent::Focus(_) => {}
	}
}

fn process_input(state: &mut State, event: InputEvent<smithay::backend::winit::WinitInput>) {
	let mut redraw = false;

	state.notify_input_activity();

	match event {
		InputEvent::Keyboard { event } => {
			let serial = state.serial_counter;
			state.serial_counter = state.serial_counter.wrapping_add(1).max(1);
			let serial: Serial = serial.into();
			let time = event.time_msec();

			let key_code = event.key_code();
			let key_state = event.state();

			let keyboard = state.keyboard.clone();
			keyboard.input(state, key_code, key_state, serial, time, |state, mods, keysym| {
				if key_state != KeyState::Pressed {
					return smithay::input::keyboard::FilterResult::Forward;
				}

				let keysym = keysym.modified_sym();

				let modifiers = crate::keybind::KeyModifiers::from_state(*mods);

				if let Some(action) = state.keybind_manager.match_keybind(keysym, modifiers, key_state) {
					debug!(
						keysym = ?keysym,
						modifiers = ?modifiers,
						action = ?action,
						"Matched keybind"
					);

					crate::config::execute_action(&action, state);
					return smithay::input::keyboard::FilterResult::Intercept(());
				}

				smithay::input::keyboard::FilterResult::Forward
			});

			redraw = true;
		}
		InputEvent::PointerMotionAbsolute { event } => {
			let pointer = state.pointer.clone();
			let serial = state.serial_counter;
			state.serial_counter = state.serial_counter.wrapping_add(1).max(1);
			let serial: Serial = serial.into();

			state.pointer_location = smithay::utils::Point::from((
				event.x_transformed(state.output_size.w),
				event.y_transformed(state.output_size.h),
			));
			let focus = pointer_focus(state, state.pointer_location);
			pointer.motion(
				state,
				focus,
				&MotionEvent {
					location: state.pointer_location,
					serial,
					time: event.time_msec(),
				},
			);
			pointer.frame(state);
			redraw = true;
		}
		InputEvent::PointerButton { event } => {
			let pointer = state.pointer.clone();
			let serial = state.serial_counter;
			state.serial_counter = state.serial_counter.wrapping_add(1).max(1);
			let serial: Serial = serial.into();

			pointer.button(
				state,
				&ButtonEvent {
					button: event.button_code(),
					state: event.state(),
					serial,
					time: event.time_msec(),
				},
			);
			pointer.frame(state);

			if event.state() == ButtonState::Pressed {
				if let Some((surface, _)) = pointer_focus(state, state.pointer_location) {
					if let Some(window_id) = state.window_for_surface(&surface) {
						state.focus_window(window_id);
					}
				}
			}

			redraw = true;
		}
		InputEvent::PointerAxis { event } => {
			let pointer = state.pointer.clone();
			let _serial = state.serial_counter;
			state.serial_counter = state.serial_counter.wrapping_add(1).max(1);

			let mut frame = AxisFrame::new(event.time_msec()).source(event.source());
			if let Some(amount) = event.amount(smithay::backend::input::Axis::Horizontal) {
				frame = frame.value(smithay::backend::input::Axis::Horizontal, amount);
			}
			if let Some(amount) = event.amount(smithay::backend::input::Axis::Vertical) {
				frame = frame.value(smithay::backend::input::Axis::Vertical, amount);
			}
			if let Some(discrete) = event.amount_v120(smithay::backend::input::Axis::Horizontal) {
				frame = frame.v120(smithay::backend::input::Axis::Horizontal, discrete as i32);
			}
			if let Some(discrete) = event.amount_v120(smithay::backend::input::Axis::Vertical) {
				frame = frame.v120(smithay::backend::input::Axis::Vertical, discrete as i32);
			}
			pointer.axis(state, frame);
			pointer.frame(state);
			redraw = true;
		}
		_ => {}
	}

	if redraw {
		state.mark_redraw();
	}
}

fn pointer_focus(
	state: &State,
	pointer_location: smithay::utils::Point<f64, smithay::utils::Logical>,
) -> Option<(
	smithay::reexports::wayland_server::protocol::wl_surface::WlSurface,
	smithay::utils::Point<f64, smithay::utils::Logical>,
)> {
	if let Some(focus) = crate::layer_shell::layer_pointer_focus(state, pointer_location) {
		return Some(focus);
	}

	state.windows_for_render().iter().rev().find_map(|window_data| {
		smithay::desktop::utils::under_from_surface_tree(
			&window_data.surface,
			pointer_location,
			(0, 0),
			smithay::desktop::WindowSurfaceType::ALL,
		)
		.map(|(wl_surface, surface_loc)| (wl_surface, surface_loc.to_f64()))
	})
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
					tracing::warn!("Failed to write xrdb resources: {}", e);
				}
			}
			match child.wait() {
				Ok(status) if status.success() => {
					tracing::info!("xrdb resources injected to XWayland :{}", display_number);
				}
				Ok(status) => tracing::warn!("xrdb exited with status: {}", status),
				Err(e) => tracing::warn!("Failed to wait on xrdb: {}", e),
			}
		}
		Err(e) => {
			if e.kind() == std::io::ErrorKind::NotFound {
				tracing::info!("xrdb not found, skipping XWayland font/cursor config");
			} else {
				tracing::warn!("Failed to run xrdb: {}", e);
			}
		}
	}
}
