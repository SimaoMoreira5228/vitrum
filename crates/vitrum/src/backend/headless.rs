use std::time::Duration;

use anyhow::{Result, anyhow};
use smithay::reexports::calloop::generic::Generic;
use smithay::reexports::calloop::{EventLoop, Interest, LoopSignal, Mode as CalloopMode, PostAction};
use smithay::wayland::socket::ListeningSocketSource;
use tracing::{info, warn};

use crate::backend::{Backend, State};

pub fn run(max_fps: Option<u16>) -> Result<()> {
	info!(max_fps = ?max_fps, "starting headless backend");

	let mut event_loop: EventLoop<State> =
		EventLoop::try_new().map_err(|err| anyhow!("failed to create headless event loop: {err:?}"))?;
	let loop_handle = event_loop.handle();
	let loop_signal = event_loop.get_signal();

	let wayland = crate::wayland_runtime::WaylandRuntime::new()?;
	let mut state = State::new(crate::config::Config::load()?, loop_handle, wayland.display_handle.clone())?;

	let socket_source = ListeningSocketSource::new_auto().map_err(|err| anyhow!("failed to bind wayland socket: {err}"))?;
	let wayland_socket = socket_source.socket_name().to_string_lossy().to_string();
	crate::launcher::set_wayland_display_socket(wayland_socket.clone());
	state.session_env.set_wayland_socket(&wayland_socket);
	crate::launcher::set_session_env(state.session_env.child_env());
	info!(socket = %wayland_socket, "wayland socket ready (headless)");

	event_loop
		.handle()
		.insert_source(socket_source, |stream, _, state| {
			if let Err(err) = state
				.display_handle
				.insert_client(stream, std::sync::Arc::new(super::ClientState::default()))
			{
				warn!(error = %err, "failed to insert wayland client");
			}
		})
		.map_err(|err| anyhow!("failed to insert listening socket source: {err:?}"))?;

	let display_source = Generic::new(wayland.display, Interest::READ, CalloopMode::Level);
	event_loop
		.handle()
		.insert_source(display_source, |_, display, state| {
			if let Err(err) = unsafe { display.get_mut().dispatch_clients(state) } {
				warn!(error = %err, "wayland client dispatch failed");
			}
			Ok::<_, std::io::Error>(PostAction::Continue)
		})
		.map_err(|err| anyhow!("failed to insert display source: {err:?}"))?;

	state.backend = Some(Backend::Headless);
	state.mark_redraw();

	let frame_interval = max_fps
		.filter(|fps| *fps > 0)
		.map(|fps| Duration::from_secs_f64(1.0 / f64::from(fps)))
		.unwrap_or_else(|| Duration::from_millis(16));

	loop {
		event_loop
			.dispatch(frame_interval, &mut state)
			.map_err(|err| anyhow!("headless event loop dispatch failed: {err:?}"))?;

		if let Err(err) = state.display_handle.flush_clients() {
			warn!(error = %err, "failed to flush wayland clients in headless backend");
		}

		if state.should_stop {
			break;
		}

		if state.take_redraw_request() {
			let frame_time = state.start_time.elapsed().as_millis() as u32;
			if let Err(err) = state.redraw(frame_time) {
				warn!(error = %err, "headless redraw failed");
			}
		}

		if let Some(receiver) = state.ipc_receiver.take() {
			crate::ipc_handler::process_ipc_messages(&receiver, &mut state);
			state.ipc_receiver = Some(receiver);
		}

		loop {
			match state.config_receiver.try_recv() {
				Ok(crate::config::ConfigEvent::Reloaded) => {
					if let Err(err) = crate::config::reload_runtime_config(&mut state) {
						warn!(error = %err, "failed to reload config in headless backend");
					}
				}
				Ok(crate::config::ConfigEvent::Error(err)) => {
					warn!(error = %err, "config watcher error in headless backend");
				}
				Err(std::sync::mpsc::TryRecvError::Empty) => break,
				Err(std::sync::mpsc::TryRecvError::Disconnected) => break,
			}
		}
	}

	stop_loop(loop_signal);
	info!("headless backend stopped");
	Ok(())
}

fn stop_loop(signal: LoopSignal) {
	signal.stop();
	signal.wakeup();
}
