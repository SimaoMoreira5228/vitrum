use smithay::reexports::wayland_protocols::xdg::decoration::zv1::server::zxdg_toplevel_decoration_v1::Mode;
use smithay::reexports::wayland_protocols::xdg::shell::server::xdg_toplevel::XdgToplevel;
use smithay::reexports::wayland_server::protocol::wl_surface::WlSurface;
use smithay::wayland::shell::xdg::ToplevelSurface;
use smithay::wayland::shell::xdg::decoration::XdgDecorationHandler;
use smithay::wayland::tablet_manager::TabletSeatHandler;
use smithay::wayland::xdg_system_bell::XdgSystemBellHandler;
use smithay::wayland::xdg_toplevel_icon::XdgToplevelIconHandler;
use tracing::debug;

use crate::backend::State;

impl XdgDecorationHandler for State {
	fn new_decoration(&mut self, toplevel: ToplevelSurface) {
		toplevel.with_pending_state(|state| {
			state.decoration_mode = Some(Mode::ServerSide);
		});
		toplevel.send_configure();
		self.mark_redraw();
	}

	fn request_mode(&mut self, toplevel: ToplevelSurface, _mode: Mode) {
		toplevel.with_pending_state(|state| {
			state.decoration_mode = Some(Mode::ServerSide);
		});
		toplevel.send_configure();
		self.mark_redraw();
	}

	fn unset_mode(&mut self, toplevel: ToplevelSurface) {
		toplevel.with_pending_state(|state| {
			state.decoration_mode = Some(Mode::ServerSide);
		});
		toplevel.send_configure();
		self.mark_redraw();
	}
}

impl XdgSystemBellHandler for State {
	fn ring(&mut self, _surface: Option<WlSurface>) {
		debug!("XDG system bell ring request received");
	}
}

impl XdgToplevelIconHandler for State {
	fn set_icon(&mut self, _toplevel: XdgToplevel, _wl_surface: WlSurface) {
		self.mark_redraw();
	}
}

impl TabletSeatHandler for State {}
