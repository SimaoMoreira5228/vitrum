use crate::ipc_handler::IpcEvent;

use smithay::desktop::Window;
use smithay::output::{Mode, Output, Scale};
use smithay::reexports::wayland_server::protocol::wl_surface::WlSurface;
use smithay::utils::{Logical, Physical, Point, Rectangle, Size};
use tracing::{debug, info, trace, warn};

use crate::window::{WindowData, WindowId};

use super::State;
use super::types::Backend;

impl State {
	pub fn mark_redraw(&mut self) {
		self.damage_tracker.add_full_damage();
		self.needs_redraw = true;
	}

	pub fn mark_redraw_with_damage(&mut self, region: Rectangle<i32, Logical>) {
		use smithay::utils::Transform;
		let scale = 1;
		let phys = region.to_physical(scale);
		self.damage_tracker.add_damage(phys);
		self.needs_redraw = true;
	}

	pub fn take_redraw_request(&mut self) -> bool {
		let redraw = self.needs_redraw;
		self.needs_redraw = false;
		redraw
	}

	pub fn emit_ipc_event(&self, event: IpcEvent) {
		if let Some(ref emitter) = self.ipc_event_emitter {
			emitter.emit(event);
		}
	}

	pub fn active_workspace_id(&self) -> u32 {
		let output_id = self
			.output_workspaces
			.focused_output()
			.or_else(|| self.output_workspaces.primary_output());
		output_id
			.and_then(|oid| self.output_workspaces.active_workspace_id(oid))
			.unwrap_or(1)
	}

	pub fn focused_workspace_set(&self) -> &crate::workspace::WorkspaceSet {
		&self.workspaces
	}

	pub fn add_window_to_active(&mut self, window_id: WindowId) {
		self.workspaces.active_mut().add_window(window_id);
	}

	pub fn remove_window_from(&mut self, workspace_id: u32, window_id: WindowId) {
		if let Some(ws) = self.workspaces.get_mut(workspace_id) {
			ws.remove_window(window_id);
		}
	}

	pub fn move_window_in_set(&mut self, window_id: WindowId, from_ws: u32, to_ws: u32) {
		self.workspaces.get_mut(from_ws).map(|ws| ws.remove_window(window_id));
		self.workspaces.get_mut(to_ws).map(|ws| ws.add_window(window_id));
	}

	pub fn _next_workspace_in_set(&mut self) {
		self.workspaces.next();
	}

	pub fn _prev_workspace_in_set(&mut self) {
		self.workspaces.prev();
	}

	pub fn set_keyboard_focus(&mut self, surface: Option<WlSurface>) {
		self.focused_surface = surface.clone();
		let _serial = smithay::utils::Serial::from(self.serial_counter);
		self.serial_counter = self.serial_counter.wrapping_add(1).max(1);
		let keyboard = self.keyboard.clone();
		keyboard.set_focus(self, surface, _serial);
	}

	pub fn add_window(
		&mut self,
		window: Window,
		surface: WlSurface,
		toplevel: smithay::wayland::shell::xdg::ToplevelSurface,
	) -> WindowId {
		let id = crate::window::new_window_id();
		let mut window_data = WindowData::new(window, surface.clone(), toplevel);
		window_data.workspace = self.active_workspace_id();

		self.windows.insert(id, window_data);
		self.surface_to_window.insert(surface.clone(), id);
		self.window_order.push(id);

		self.add_window_to_active(id);

		self.set_keyboard_focus(Some(surface.clone()));

		let foreign_toplevel = self._foreign_toplevel_list_state.new_toplevel::<State>("", "");
		self.foreign_toplevel_handles.insert(id, foreign_toplevel);

		info!(id = ?id, workspace = self.active_workspace_id(), "Window mapped into compositor state");

		if let Some(wd) = self.windows.get(&id) {
			self.emit_ipc_event(IpcEvent::WindowOpened {
				window: vitrum_ipc::WindowInfo {
					id: vitrum_ipc::WindowId(id.0),
					title: wd.title.clone(),
					app_id: wd.app_id.clone(),
					workspace: wd.workspace,
					floating: wd.flags.floating,
					fullscreen: wd.flags.fullscreen,
					opacity: wd.opacity,
					pinned: wd.flags.pinned,
					urgent: wd.flags.urgent,
				},
			});
		}

		self.apply_layout();
		id
	}

	pub fn add_window_from_x11(
		&mut self,
		window: smithay::desktop::Window,
		surface: WlSurface,
		geometry: smithay::utils::Rectangle<i32, smithay::utils::Logical>,
	) -> WindowId {
		let id = crate::window::new_window_id();
		let mut window_data = WindowData::new_x11(window, surface.clone());
		window_data.workspace = self.active_workspace_id();
		window_data.geometry = geometry;

		self.windows.insert(id, window_data);
		self.surface_to_window.insert(surface.clone(), id);
		self.window_order.push(id);

		self.add_window_to_active(id);

		self.set_keyboard_focus(Some(surface.clone()));

		let foreign_toplevel = self._foreign_toplevel_list_state.new_toplevel::<State>("", "");
		self.foreign_toplevel_handles.insert(id, foreign_toplevel);

		info!(id = ?id, workspace = self.active_workspace_id(), "X11 window mapped into compositor state");

		self.apply_layout();
		id
	}

	pub fn remove_window(&mut self, id: WindowId) {
		if let Some(window_data) = self.windows.remove(&id) {
			if let Some(handle) = self.foreign_toplevel_handles.remove(&id) {
				handle.send_closed();
			}

			self.surface_to_window.remove(&window_data.surface);
			self.window_order.retain(|&wid| wid != id);

			self.remove_window_from(window_data.workspace, id);

			if self.focused_surface.as_ref().is_some_and(|s| *s == window_data.surface) {
				self.focused_surface = None;
			}

			info!(id = ?id, workspace = window_data.workspace, "Window unmapped from compositor state");
			self.emit_ipc_event(IpcEvent::WindowClosed {
				id: vitrum_ipc::WindowId(id.0),
			});
			self.sync_focus_to_active_workspace();

			self.apply_layout();
		}
	}

	pub fn window_for_surface(&self, surface: &WlSurface) -> Option<WindowId> {
		self.surface_to_window.get(surface).copied()
	}

	pub fn prune_windows(&mut self) {
		let dead_windows: Vec<WindowId> = self
			.windows
			.iter()
			.filter(|(_, w)| !w.is_alive())
			.map(|(id, _)| *id)
			.collect();

		for id in dead_windows {
			self.remove_window(id);
		}
	}

	pub fn windows_for_render(&self) -> Vec<&WindowData> {
		self.window_order.iter().filter_map(|id| self.windows.get(id)).collect()
	}

	pub fn apply_layout(&mut self) {
		let workspace = self.focused_workspace_set().active();

		let tiled_count = workspace.window_count();

		let tiling_area = crate::layer_shell::compute_tiling_area(self, self.output_size);

		debug!(
			workspace = self.active_workspace_id(),
			tiled_count,
			output_w = self.output_size.w,
			output_h = self.output_size.h,
			tile_w = tiling_area.size.w,
			tile_h = tiling_area.size.h,
			tile_x = tiling_area.loc.x,
			tile_y = tiling_area.loc.y,
			"Applying layout"
		);

		let geometries = self
			.layout_engine
			.arrange_workspace(workspace, tiling_area.size, |id| self.windows.get(&id).cloned());

		let offset = tiling_area.loc;
		for (id, geo) in &geometries {
			if let Some(window_data) = self.windows.get_mut(id) {
				let adjusted = smithay::utils::Rectangle::new(
					smithay::utils::Point::from((geo.loc.x + offset.x, geo.loc.y + offset.y)),
					geo.size,
				);
				trace!(
					id = ?id,
					x = adjusted.loc.x,
					y = adjusted.loc.y,
					w = adjusted.size.w,
					h = adjusted.size.h,
					"Window geometry assigned"
				);
				window_data.set_geometry(adjusted);
			}
		}

		if geometries.is_empty() && tiled_count > 0 {
			warn!(
				workspace = self.active_workspace_id(),
				tiled_count, "Layout produced no geometries despite workspace having tiled windows"
			);
		}

		let active_id = self.active_workspace_id();
		for (_, window_data) in self.windows.iter_mut() {
			if window_data.workspace == active_id && window_data.flags.fullscreen {
				window_data.set_geometry(smithay::utils::Rectangle::new(
					smithay::utils::Point::from((0, 0)),
					self.output_size,
				));
			}
		}

		self.mark_redraw();
	}

	pub fn sync_foreign_toplevel_metadata(&mut self, window_id: WindowId) {
		let Some(window_data) = self.windows.get(&window_id) else {
			return;
		};

		let Some(handle) = self.foreign_toplevel_handles.get(&window_id) else {
			return;
		};

		handle.send_title(window_data.title.as_str());
		handle.send_app_id(window_data.app_id.as_str());
		handle.send_done();
	}

	pub fn focus_window(&mut self, id: WindowId) {
		let (surface, workspace) = match self.windows.get(&id) {
			Some(w) => (w.surface.clone(), w.workspace),
			None => {
				warn!(id = ?id, "Cannot focus: window not found");
				return;
			}
		};

		let _old_focus = self.focused_surface.as_ref().and_then(|s| self.window_for_surface(s));
		self.window_order.retain(|&wid| wid != id);
		self.window_order.push(id);
		self.set_keyboard_focus(Some(surface));
		info!(id = ?id, workspace = workspace, "Focused window changed");
		self.emit_ipc_event(IpcEvent::WindowFocused {
			id: vitrum_ipc::WindowId(id.0),
		});
		self.mark_redraw();
	}

	pub fn focus_direction(&mut self, dir: vitrum_ipc::Direction) -> bool {
		let focused_id = self
			.focused_surface
			.as_ref()
			.and_then(|surface| self.window_for_surface(surface));

		let focused_geo = match focused_id.and_then(|id| self.windows.get(&id)) {
			Some(w) => w.geometry,
			None => {
				let first = self
					.window_order
					.iter()
					.filter(|id| {
						self.windows
							.get(id)
							.is_some_and(|w| w.workspace == self.active_workspace_id() && w.is_alive())
					})
					.copied()
					.next();
				if let Some(id) = first {
					self.focus_window(id);
					return true;
				}
				return false;
			}
		};

		let focused_center = (
			focused_geo.loc.x + focused_geo.size.w / 2,
			focused_geo.loc.y + focused_geo.size.h / 2,
		);

		let mut best: Option<(WindowId, f64)> = None;

		for (id, window_data) in &self.windows {
			if Some(*id) == focused_id {
				continue;
			}
			if window_data.workspace != self.active_workspace_id() || !window_data.is_alive() {
				continue;
			}

			if window_data.flags.fullscreen {
				continue;
			}

			let other_center = (
				window_data.geometry.loc.x + window_data.geometry.size.w / 2,
				window_data.geometry.loc.y + window_data.geometry.size.h / 2,
			);

			let dx = (other_center.0 - focused_center.0) as f64;
			let dy = (other_center.1 - focused_center.1) as f64;

			let in_direction = match dir {
				vitrum_ipc::Direction::Left => dx < 0.0 && dy.abs() <= dx.abs(),
				vitrum_ipc::Direction::Right => dx > 0.0 && dy.abs() <= dx.abs(),
				vitrum_ipc::Direction::Up => dy < 0.0 && dx.abs() <= dy.abs(),
				vitrum_ipc::Direction::Down => dy > 0.0 && dx.abs() <= dy.abs(),
			};

			if !in_direction {
				continue;
			}

			let dist = (dx * dx + dy * dy).sqrt();
			match &best {
				Some((_, best_dist)) if dist < *best_dist => {
					best = Some((*id, dist));
				}
				None => {
					best = Some((*id, dist));
				}
				_ => {}
			}
		}

		if let Some((target_id, _)) = best {
			self.focus_window(target_id);
			true
		} else {
			false
		}
	}

	pub fn switch_workspace(&mut self, workspace_id: u32) {
		if workspace_id >= 1 && workspace_id <= 10 {
			let output_id = self
				.output_workspaces
				.focused_output()
				.or_else(|| self.output_workspaces.primary_output());

			self.workspaces.switch_to(workspace_id);
			if let Some(oid) = output_id {
				self.output_workspaces.switch_workspace(oid, workspace_id);
			}

			self.sync_focus_to_active_workspace();
			self.apply_layout();
			self.emit_ipc_event(IpcEvent::WorkspaceChanged { workspace: workspace_id });
		}
	}

	pub fn move_window_to_workspace(&mut self, window_id: WindowId, workspace_id: u32) {
		if !(1..=10).contains(&workspace_id) {
			warn!(workspace = workspace_id, "Ignoring move to invalid workspace");
			return;
		}

		let old_workspace = match self.windows.get(&window_id) {
			Some(wd) if wd.workspace != workspace_id => wd.workspace,
			Some(_) => return,
			None => {
				warn!(id = ?window_id, "Cannot move window: not found");
				return;
			}
		};

		self.move_window_in_set(window_id, old_workspace, workspace_id);

		if let Some(window_data) = self.windows.get_mut(&window_id) {
			window_data.workspace = workspace_id;
		}

		self.sync_focus_to_active_workspace();

		info!(
			id = ?window_id,
			from_workspace = old_workspace,
			to_workspace = workspace_id,
			"Moved window between workspaces"
		);

		self.emit_ipc_event(IpcEvent::WindowMoved {
			id: vitrum_ipc::WindowId(window_id.0),
			from_workspace: old_workspace,
			to_workspace: workspace_id,
		});

		self.apply_layout();
	}

	fn sync_focus_to_active_workspace(&mut self) {
		let focused_in_active_workspace = self
			.focused_surface
			.as_ref()
			.and_then(|surface| self.window_for_surface(surface))
			.and_then(|id| self.windows.get(&id))
			.is_some_and(|window| window.workspace == self.active_workspace_id());

		if focused_in_active_workspace {
			return;
		}

		let new_focus = self
			.window_order
			.iter()
			.rev()
			.find_map(|id| self.windows.get(id))
			.filter(|window| window.workspace == self.active_workspace_id())
			.map(|window| window.surface.clone());

		if let Some(surface) = new_focus.as_ref() {
			if let Some(id) = self.window_for_surface(surface) {
				debug!(id = ?id, workspace = self.active_workspace_id(), "Focus synchronized to active workspace");
			}
			self.set_keyboard_focus(Some(surface.clone()));
		} else {
			self.set_keyboard_focus(None);
			debug!(
				workspace = self.active_workspace_id(),
				"No focusable window in active workspace"
			);
		}
	}

	pub fn redraw(&mut self, frame_time: u32) -> anyhow::Result<()> {
		let backend_opt = self.backend.take();
		match backend_opt {
			Some(Backend::Winit(mut be)) => {
				let result = crate::backend::render::redraw(&mut be, self, frame_time);
				self.backend = Some(Backend::Winit(be));
				result?;
			}
			Some(Backend::Drm(mut be)) => {
				if let Err(e) = be.render(self, frame_time) {
					self.backend = Some(Backend::Drm(be));
					return Err(e);
				}
				self.backend = Some(Backend::Drm(be));
			}
			other => {
				self.backend = other;
			}
		}
		Ok(())
	}

	pub fn _stacked_toplevel_surfaces(&self) -> Vec<(WlSurface, Point<i32, Logical>)> {
		self.visible_toplevel_surfaces()
			.into_iter()
			.map(|(s, loc, _)| (s, loc))
			.collect()
	}

	pub fn visible_toplevel_surfaces(&self) -> Vec<(WlSurface, Point<i32, Logical>, bool)> {
		let mut selected = Vec::new();

		for &id in &self.window_order {
			if let Some(window_data) = self.windows.get(&id) {
				if !window_data.is_alive() {
					continue;
				}

				if !window_data.flags.pinned && window_data.workspace != self.active_workspace_id() {
					continue;
				}

				selected.push((
					window_data.surface.clone(),
					window_data.geometry.loc,
					window_data.flags.floating,
				));
			}
		}

		selected
	}

	pub fn tiled_surfaces(&self) -> Vec<(WlSurface, Point<i32, Logical>)> {
		self.visible_toplevel_surfaces()
			.into_iter()
			.filter(|(_, _, floating)| !*floating)
			.map(|(s, loc, _)| (s, loc))
			.collect()
	}

	pub fn floating_surfaces(&self) -> Vec<(WlSurface, Point<i32, Logical>)> {
		self.visible_toplevel_surfaces()
			.into_iter()
			.filter(|(_, _, floating)| *floating)
			.map(|(s, loc, _)| (s, loc))
			.collect()
	}

	pub fn stacked_surfaces_for_render(&self) -> Vec<(WlSurface, Point<i32, Logical>)> {
		if crate::session_lock::should_render_lock_surfaces_only(self) {
			return crate::session_lock::get_lock_surfaces(self)
				.into_iter()
				.map(|surface| (surface.wl_surface().clone(), Point::from((0, 0))))
				.collect();
		}

		let mut surfaces = Vec::new();

		let output_id = self.output_id.or_else(|| self.output_manager.map().primary().map(|o| o.id));
		let layer_surfaces = if let Some(oid) = output_id {
			crate::layer_shell::get_layer_surfaces_with_positions(self, oid, self.output_size)
		} else {
			crate::layer_shell::get_layer_surface_positions(self, self.output_size)
				.into_iter()
				.map(|(s, p)| (smithay::wayland::shell::wlr_layer::Layer::Top, s, p))
				.collect()
		};

		use smithay::wayland::shell::wlr_layer::Layer;

		for (_, surface, pos) in layer_surfaces.iter().filter(|(l, _, _)| *l == Layer::Background) {
			surfaces.push((surface.clone(), *pos));
		}

		for (_, surface, pos) in layer_surfaces.iter().filter(|(l, _, _)| *l == Layer::Bottom) {
			surfaces.push((surface.clone(), *pos));
		}

		surfaces.extend(self.tiled_surfaces());

		surfaces.extend(self.floating_surfaces());

		for (_, surface, pos) in layer_surfaces.iter().filter(|(l, _, _)| *l == Layer::Top) {
			surfaces.push((surface.clone(), *pos));
		}

		for (_, surface, pos) in layer_surfaces.iter().filter(|(l, _, _)| *l == Layer::Overlay) {
			surfaces.push((surface.clone(), *pos));
		}

		surfaces
	}

	pub fn kill_window(&mut self, window_id: WindowId) {
		if let Some(window_data) = self.windows.get(&window_id) {
			if let Some(ref toplevel) = window_data.toplevel {
				toplevel.send_close();
			} else if let Some(x11_surface) = window_data.window.x11_surface() {
				let _ = x11_surface.close();
			}
		}
	}

	pub fn ensure_output_registered(
		&mut self,
		output: Output,
		logical_size: Size<i32, Logical>,
		refresh_mhz: Option<i32>,
	) -> crate::output::OutputId {
		let output_name = output.name();

		let output_id = self
			.output_manager
			.map()
			.get_by_name(&output_name)
			.map(|existing| existing.id)
			.unwrap_or_else(|| {
				let output_state = crate::output::OutputState::new(output_name.clone(), logical_size);
				self.output_manager.add_output(output_state)
			});

		self.output_manager
			.update_property(output_id, crate::output::OutputProperty::Size(logical_size));

		let mode = Mode {
			size: smithay::utils::Size::<i32, Physical>::from((logical_size.w, logical_size.h)),
			refresh: refresh_mhz.unwrap_or(60_000),
		};
		output.change_current_state(Some(mode), None, Some(Scale::Fractional(1.0)), None);
		output.set_preferred(mode);

		if !self._wl_output_globals.contains_key(&output_id) {
			let global = output.create_global::<State>(&self.display_handle);
			self._wl_output_globals.insert(output_id, global);
		}

		self.smithay_outputs.insert(output_id, output);

		if self.output_id.is_none() {
			self.output_id = Some(output_id);
		}

		if !self.output_workspaces.has_output(output_id) {
			self.output_workspaces.add_output(output_id, Some(output_name.clone()));
			info!(output = %output_name, id = ?output_id, "Output registered with workspace manager");
		}

		output_id
	}

	pub fn sync_output_state(
		&mut self,
		output_id: crate::output::OutputId,
		logical_size: Size<i32, Logical>,
		scale_factor: f64,
		refresh_mhz: Option<i32>,
	) {
		self.output_manager
			.update_property(output_id, crate::output::OutputProperty::Size(logical_size));
		self.output_manager
			.update_property(output_id, crate::output::OutputProperty::Scale(scale_factor));

		if let Some(output) = self.smithay_outputs.get(&output_id) {
			let refresh = refresh_mhz.unwrap_or_else(|| output.current_mode().map(|m| m.refresh).unwrap_or(60_000));
			let mode = Mode {
				size: smithay::utils::Size::<i32, Physical>::from((logical_size.w, logical_size.h)),
				refresh,
			};
			output.change_current_state(Some(mode), None, Some(Scale::Fractional(scale_factor)), None);
			output.set_preferred(mode);
		}
	}

	pub fn toggle_floating(&mut self, window_id: WindowId) {
		if let Some(window_data) = self.windows.get_mut(&window_id) {
			window_data.flags.floating = !window_data.flags.floating;
			if window_data.flags.floating {
				let floating_geo = self.layout_engine.default_floating_geometry(self.output_size);
				window_data.set_geometry(floating_geo);
			}
			self.apply_layout();
		}
	}

	pub fn toggle_fullscreen(&mut self, window_id: WindowId) {
		if let Some(window_data) = self.windows.get_mut(&window_id) {
			window_data.flags.fullscreen = !window_data.flags.fullscreen;

			if window_data.flags.fullscreen {
				window_data.set_geometry(smithay::utils::Rectangle::new(
					smithay::utils::Point::from((0, 0)),
					self.output_size,
				));
				info!(id = ?window_id, "Window fullscreened");
			} else {
				info!(id = ?window_id, "Window un-fullscreened");
			}

			self.apply_layout();
		}
	}

	pub fn toggle_pinned(&mut self, window_id: WindowId) {
		if let Some(window_data) = self.windows.get_mut(&window_id) {
			window_data.flags.pinned = !window_data.flags.pinned;
			let pinned = window_data.flags.pinned;
			info!(id = ?window_id, pinned, "Window pin toggled");
			self.mark_redraw();
		}
	}

	pub fn process_output_changes(&mut self) {
		if !self.output_manager.has_changes() {
			return;
		}

		let changes = self.output_manager.take_changes();
		for change in changes {
			match change {
				crate::output::OutputChange::Added(id) => {
					info!(output = ?id, "Output added");
					self.apply_layout();
					self.mark_redraw();
				}
				crate::output::OutputChange::Removed(id) => {
					info!(output = ?id, "Output removed");
					self.apply_layout();
					self.mark_redraw();
				}
				crate::output::OutputChange::Changed(id, prop) => {
					info!(output = ?id, property = ?prop, "Output changed");
					self.apply_layout();
					self.mark_redraw();
				}
			}
		}
	}

	pub fn swap_windows(&mut self, a: WindowId, b: WindowId) {
		let ws_a = self.windows.get(&a).map(|w| w.workspace);
		let ws_b = self.windows.get(&b).map(|w| w.workspace);

		if let (Some(wa), Some(wb)) = (ws_a, ws_b) {
			if wa != wb {
				if let Some(wd) = self.windows.get_mut(&a) {
					wd.workspace = wb;
				}
				if let Some(wd) = self.windows.get_mut(&b) {
					wd.workspace = wa;
				}
				self.move_window_in_set(a, wa, wb);
				self.move_window_in_set(b, wb, wa);
			}
		}

		if let (Some(pos_a), Some(pos_b)) = (
			self.window_order.iter().position(|&id| id == a),
			self.window_order.iter().position(|&id| id == b),
		) {
			self.window_order.swap(pos_a, pos_b);
		}

		self.apply_layout();
		info!(a = ?a, b = ?b, "Windows swapped");
	}
}
