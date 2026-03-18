use smithay::backend::input::{
	AbsolutePositionEvent, ButtonState, Event, InputEvent, KeyState, KeyboardKeyEvent, PointerAxisEvent, PointerButtonEvent,
	PointerMotionEvent,
};
use smithay::input::pointer::{AxisFrame, ButtonEvent, MotionEvent};
use smithay::utils::{Logical, Point, Serial};
use tracing::debug;

use super::DrmBackend;
use crate::backend::State;

impl DrmBackend {
	pub(super) fn process_input(state: &mut State, event: InputEvent<smithay::backend::libinput::LibinputInputBackend>) {
		let mut redraw = false;

		state.notify_input_activity();

		let serial = state.serial_counter;
		state.serial_counter = state.serial_counter.wrapping_add(1).max(1);
		let serial: Serial = serial.into();
		let pointer = state.pointer.clone();

		match event {
			InputEvent::Keyboard { event } => {
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
			InputEvent::PointerMotion { event } => {
				let delta = event.delta();
				state.pointer_location += delta;
				let focus = Self::pointer_focus(state, state.pointer_location);
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
			InputEvent::PointerMotionAbsolute { event } => {
				state.pointer_location = Point::from((
					event.x_transformed(state.output_size.w),
					event.y_transformed(state.output_size.h),
				));
				let focus = Self::pointer_focus(state, state.pointer_location);
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
					if let Some((surface, _)) = Self::pointer_focus(state, state.pointer_location) {
						if let Some(window_id) = state.window_for_surface(&surface) {
							state.focus_window(window_id);
						}
					}
				}

				redraw = true;
			}
			InputEvent::PointerAxis { event } => {
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
		pointer_location: Point<f64, Logical>,
	) -> Option<(
		smithay::reexports::wayland_server::protocol::wl_surface::WlSurface,
		Point<f64, Logical>,
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
}
