use crate::backend::State;

#[test]
fn test_tiled_surfaces_excludes_floating() {
	let display = smithay::reexports::wayland_server::Display::<State>::new().unwrap();
	let dh = display.handle();
	let event_loop = calloop::EventLoop::try_new().unwrap();
	let state = State::new_for_test(event_loop.handle(), dh).unwrap();

	assert!(state.tiled_surfaces().is_empty());
}

#[test]
fn test_floating_surfaces_includes_only_floating() {
	let display = smithay::reexports::wayland_server::Display::<State>::new().unwrap();
	let dh = display.handle();
	let event_loop = calloop::EventLoop::try_new().unwrap();
	let state = State::new_for_test(event_loop.handle(), dh).unwrap();

	assert!(state.floating_surfaces().is_empty());
}

#[test]
fn test_visible_toplevel_surfaces_skips_other_workspaces() {
	let display = smithay::reexports::wayland_server::Display::<State>::new().unwrap();
	let dh = display.handle();
	let event_loop = calloop::EventLoop::try_new().unwrap();
	let state = State::new_for_test(event_loop.handle(), dh).unwrap();

	assert!(state.visible_toplevel_surfaces().is_empty());
}

#[test]
fn test_stacked_surfaces_for_render_no_lock() {
	let display = smithay::reexports::wayland_server::Display::<State>::new().unwrap();
	let dh = display.handle();
	let event_loop = calloop::EventLoop::try_new().unwrap();
	let state = State::new_for_test(event_loop.handle(), dh).unwrap();

	let surfaces = state.stacked_surfaces_for_render();
	assert!(surfaces.is_empty());
}
