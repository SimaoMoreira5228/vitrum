use std::collections::HashMap;
use std::time::Instant;

use smithay::input::SeatState;
use smithay::input::keyboard::KeyboardHandle;
use smithay::input::pointer::PointerHandle;
use smithay::output::Output;
use smithay::reexports::calloop::LoopHandle;
use smithay::reexports::wayland_server::DisplayHandle;
use smithay::reexports::wayland_server::backend::GlobalId;
use smithay::reexports::wayland_server::protocol::wl_surface::WlSurface;
use smithay::utils::{Logical, Physical, Point, Size};
use smithay::wayland::compositor::CompositorState;
use smithay::wayland::cursor_shape::CursorShapeManagerState;
use smithay::wayland::foreign_toplevel_list::{ForeignToplevelHandle, ForeignToplevelListState};
use smithay::wayland::fractional_scale::FractionalScaleManagerState;
use smithay::wayland::idle_notify::IdleNotifierState;
use smithay::wayland::image_capture_source::{ImageCaptureSourceState, OutputCaptureSourceState};
use smithay::wayland::image_copy_capture::ImageCopyCaptureState;
use smithay::wayland::input_method::InputMethodManagerState;
use smithay::wayland::output::OutputManagerState;
use smithay::wayland::presentation::PresentationState;
use smithay::wayland::selection::data_device::DataDeviceState;
use smithay::wayland::selection::primary_selection::PrimarySelectionState;
use smithay::wayland::shell::xdg::XdgShellState;
use smithay::wayland::shell::xdg::decoration::XdgDecorationState;
use smithay::wayland::shm::ShmState;
use smithay::wayland::text_input::TextInputManagerState;
use smithay::wayland::viewporter::ViewporterState;
use smithay::wayland::xdg_system_bell::XdgSystemBellState;
use smithay::wayland::xdg_toplevel_icon::XdgToplevelIconManager;
use smithay::wayland::xwayland_shell::XWaylandShellState;
use tracing::info;

use crate::config::Config;
use crate::damage::DamageTracker;
use crate::keybind::KeybindManager;
use crate::layer_shell::LayerShellManager;
use crate::layout::LayoutEngine;
use crate::output::OutputManager;
use crate::session_lock::SessionLockManager;
use crate::wallpaper::WallpaperState;
use crate::window::{WindowData, WindowId};
use crate::workspace::WorkspaceSet;

use super::types::{Backend, CapturedFrame};

pub struct State {
	pub compositor_state: CompositorState,
	pub xdg_shell_state: XdgShellState,
	pub shm_state: ShmState,
	pub seat_state: SeatState<Self>,
	pub _seat: smithay::input::Seat<Self>,
	pub focused_surface: Option<WlSurface>,
	pub needs_redraw: bool,

	pub idle_notifier_state: IdleNotifierState<Self>,

	pub windows: HashMap<WindowId, WindowData>,
	pub surface_to_window: HashMap<WlSurface, WindowId>,
	pub window_order: Vec<WindowId>,

	pub layout_engine: LayoutEngine,
	pub workspaces: WorkspaceSet,

	pub output_size: Size<i32, Logical>,

	pub config: Config,

	pub wallpaper: WallpaperState,

	pub keybind_manager: KeybindManager,

	pub output_manager: OutputManager,
	pub _output_manager_state: OutputManagerState,
	pub _wl_output_globals: HashMap<crate::output::OutputId, GlobalId>,
	pub smithay_outputs: HashMap<crate::output::OutputId, Output>,

	pub layer_shell_manager: LayerShellManager,

	pub session_lock_manager: SessionLockManager,

	pub _fractional_scale_state: FractionalScaleManagerState,

	pub _input_method_state: InputMethodManagerState,
	pub _presentation_state: PresentationState,
	pub _foreign_toplevel_list_state: ForeignToplevelListState,
	pub foreign_toplevel_handles: HashMap<WindowId, ForeignToplevelHandle>,
	pub _image_capture_source_state: ImageCaptureSourceState,
	pub _output_capture_source_state: OutputCaptureSourceState,
	pub _image_copy_capture_state: ImageCopyCaptureState,
	pub screencopy_sessions: Vec<smithay::wayland::image_copy_capture::Session>,

	pub xdg_activation_state: smithay::wayland::xdg_activation::XdgActivationState,

	pub xwayland: Option<smithay::xwayland::XWayland>,
	pub xwm: Option<smithay::xwayland::xwm::X11Wm>,
	pub xwayland_client: Option<smithay::reexports::wayland_server::Client>,
	pub xwayland_shell_state: XWaylandShellState,
	pub _xdg_decoration_state: XdgDecorationState,
	pub _text_input_manager_state: TextInputManagerState,
	pub _xdg_system_bell_state: XdgSystemBellState,
	pub _xdg_toplevel_icon_manager: XdgToplevelIconManager,
	pub _cursor_shape_manager_state: CursorShapeManagerState,

	pub session_env: crate::session::SessionEnvironment,

	pub damage_tracker: DamageTracker,

	pub idle_timeout_secs: u64,

	pub is_idle: bool,

	pub data_device_state: DataDeviceState,

	pub primary_selection_state: Option<PrimarySelectionState>,

	pub _viewporter_state: ViewporterState,

	pub output_workspaces: crate::workspace::OutputWorkspaces,

	pub backend: Option<Backend>,
	pub should_stop: bool,
	pub frame_interval: Option<std::time::Duration>,
	pub frame_timer_token: Option<calloop::RegistrationToken>,
	pub window_size: Option<Size<i32, Physical>>,
	pub display_handle: DisplayHandle,
	pub keyboard: KeyboardHandle<Self>,
	pub pointer: PointerHandle<Self>,
	pub pointer_location: Point<f64, Logical>,
	pub serial_counter: u32,
	pub start_time: Instant,
	pub ipc_receiver: Option<std::sync::mpsc::Receiver<crate::ipc_handler::IpcMessage>>,
	pub ipc_event_emitter: Option<crate::ipc_handler::IpcEventEmitter>,
	pub _config_manager: crate::config::ConfigManager,
	pub config_sender: std::sync::mpsc::Sender<crate::config::ConfigEvent>,
	pub config_receiver: std::sync::mpsc::Receiver<crate::config::ConfigEvent>,
	pub output_id: Option<crate::output::OutputId>,
	pub last_winit_capture: Option<CapturedFrame>,
}

impl State {
	pub fn new(
		config: Config,
		loop_handle: LoopHandle<'static, Self>,
		display_handle: DisplayHandle,
	) -> anyhow::Result<Self> {
		let compositor_state = CompositorState::new::<Self>(&display_handle);
		let xdg_shell_state = XdgShellState::new::<Self>(&display_handle);
		let shm_state = ShmState::new::<Self>(&display_handle, vec![]);
		let mut seat_state = SeatState::new();
		let mut seat = seat_state.new_wl_seat(&display_handle, "seat0");
		let idle_notifier_state = IdleNotifierState::new(&display_handle, loop_handle.clone());
		let fractional_scale_state = FractionalScaleManagerState::new::<Self>(&display_handle);
		let input_method_state = InputMethodManagerState::new::<Self, _>(&display_handle, |_| true);
		let presentation_state = PresentationState::new::<Self>(&display_handle, libc::CLOCK_MONOTONIC as u32);
		let foreign_toplevel_list_state = ForeignToplevelListState::new::<Self>(&display_handle);
		let image_capture_source_state = ImageCaptureSourceState::new();
		let output_capture_source_state = OutputCaptureSourceState::new::<Self>(&display_handle);
		let image_copy_capture_state = ImageCopyCaptureState::new::<Self>(&display_handle);
		let data_device_state = DataDeviceState::new::<Self>(&display_handle);
		let primary_selection_state = Some(PrimarySelectionState::new::<Self>(&display_handle));
		let viewporter_state = ViewporterState::new::<Self>(&display_handle);

		let xwayland_shell_state = XWaylandShellState::new::<Self>(&display_handle);
		let xdg_decoration_state = XdgDecorationState::new::<Self>(&display_handle);
		let text_input_manager_state = TextInputManagerState::new::<Self>(&display_handle);
		let xdg_system_bell_state = XdgSystemBellState::new::<Self>(&display_handle);
		let xdg_toplevel_icon_manager = XdgToplevelIconManager::new::<Self>(&display_handle);
		let cursor_shape_manager_state = CursorShapeManagerState::new::<Self>(&display_handle);
		let session_env = crate::session::SessionEnvironment::new(&config);

		let keybind_manager = KeybindManager::new(config.keybind.clone());
		let output_manager = OutputManager::new();
		let output_manager_state = OutputManagerState::new_with_xdg_output::<Self>(&display_handle);
		let layer_shell_manager = LayerShellManager::new(&display_handle);
		let session_lock_manager = SessionLockManager::new(&display_handle);

		let workspaces = WorkspaceSet::new();
		let layout_engine = LayoutEngine::new();
		let mut output_workspaces = crate::workspace::OutputWorkspaces::new();

		let default_output = crate::output::OutputId::next();
		output_workspaces.add_output(default_output, None);

		let wallpaper = WallpaperState::new(&config.wallpaper);

		let damage_tracker = DamageTracker::new(1920, 1080);

		let repeat_delay = i32::try_from(config.input.repeat_delay).unwrap_or(i32::MAX);
		let repeat_rate = i32::try_from(config.input.repeat_rate).unwrap_or(i32::MAX);

		let keyboard = seat.add_keyboard(smithay::input::keyboard::XkbConfig::default(), repeat_delay, repeat_rate)?;
		let pointer = seat.add_pointer();

		let (ipc_receiver, ipc_event_emitter) = match crate::ipc_handler::start_ipc_server() {
			Ok((receiver, emitter)) => {
				info!("IPC server started");
				(Some(receiver), Some(emitter))
			}
			Err(err) => {
				tracing::warn!(error = %err, "failed to start IPC server");
				(None, None)
			}
		};

		let (config_manager, _config, config_sender, config_receiver) = crate::config::ConfigManager::new()?;

		Ok(Self {
			compositor_state,
			xdg_shell_state,
			shm_state,
			seat_state,
			_seat: seat,
			focused_surface: None,
			needs_redraw: false,
			idle_notifier_state,
			windows: HashMap::new(),
			surface_to_window: HashMap::new(),
			window_order: Vec::new(),
			layout_engine,
			workspaces,
			output_size: Size::from((1920, 1080)),
			config,
			wallpaper,
			keybind_manager,
			output_manager,
			_output_manager_state: output_manager_state,
			_wl_output_globals: HashMap::new(),
			smithay_outputs: HashMap::new(),
			layer_shell_manager,
			session_lock_manager,
			_fractional_scale_state: fractional_scale_state,
			_input_method_state: input_method_state,
			_presentation_state: presentation_state,
			_foreign_toplevel_list_state: foreign_toplevel_list_state,
			foreign_toplevel_handles: HashMap::new(),
			_image_capture_source_state: image_capture_source_state,
			_output_capture_source_state: output_capture_source_state,
			_image_copy_capture_state: image_copy_capture_state,
			screencopy_sessions: Vec::new(),
			xdg_activation_state: smithay::wayland::xdg_activation::XdgActivationState::new::<Self>(&display_handle),
			xwayland: None,
			xwm: None,
			xwayland_client: None,
			xwayland_shell_state,
			_xdg_decoration_state: xdg_decoration_state,
			_text_input_manager_state: text_input_manager_state,
			_xdg_system_bell_state: xdg_system_bell_state,
			_xdg_toplevel_icon_manager: xdg_toplevel_icon_manager,
			_cursor_shape_manager_state: cursor_shape_manager_state,
			session_env,

			damage_tracker,

			idle_timeout_secs: 300,
			is_idle: false,

			data_device_state,
			primary_selection_state,
			_viewporter_state: viewporter_state,
			output_workspaces: output_workspaces,
			backend: None,
			should_stop: false,
			frame_interval: None,
			frame_timer_token: None,
			window_size: None,
			display_handle,
			keyboard,
			pointer,
			pointer_location: Point::from((960.0, 540.0)),
			serial_counter: 0,
			start_time: Instant::now(),
			ipc_receiver,
			ipc_event_emitter,
			_config_manager: config_manager,
			config_sender,
			config_receiver,
			output_id: None,
			last_winit_capture: None,
		})
	}

	#[cfg(test)]
	pub fn new_for_test(loop_handle: LoopHandle<'static, Self>, display_handle: DisplayHandle) -> anyhow::Result<Self> {
		let config = Config::default();
		let compositor_state = CompositorState::new::<Self>(&display_handle);
		let xdg_shell_state = XdgShellState::new::<Self>(&display_handle);
		let shm_state = ShmState::new::<Self>(&display_handle, vec![]);
		let mut seat_state = SeatState::new();
		let mut seat = seat_state.new_wl_seat(&display_handle, "seat0");
		let idle_notifier_state = IdleNotifierState::new(&display_handle, loop_handle.clone());
		let fractional_scale_state = FractionalScaleManagerState::new::<Self>(&display_handle);
		let input_method_state = InputMethodManagerState::new::<Self, _>(&display_handle, |_| true);
		let presentation_state = PresentationState::new::<Self>(&display_handle, libc::CLOCK_MONOTONIC as u32);
		let foreign_toplevel_list_state = ForeignToplevelListState::new::<Self>(&display_handle);
		let image_capture_source_state = ImageCaptureSourceState::new();
		let output_capture_source_state = OutputCaptureSourceState::new::<Self>(&display_handle);
		let image_copy_capture_state = ImageCopyCaptureState::new::<Self>(&display_handle);
		let data_device_state = DataDeviceState::new::<Self>(&display_handle);
		let primary_selection_state = Some(PrimarySelectionState::new::<Self>(&display_handle));
		let viewporter_state = ViewporterState::new::<Self>(&display_handle);
		let xwayland_shell_state = XWaylandShellState::new::<Self>(&display_handle);
		let xdg_decoration_state = XdgDecorationState::new::<Self>(&display_handle);
		let text_input_manager_state = TextInputManagerState::new::<Self>(&display_handle);
		let xdg_system_bell_state = XdgSystemBellState::new::<Self>(&display_handle);
		let xdg_toplevel_icon_manager = XdgToplevelIconManager::new::<Self>(&display_handle);
		let cursor_shape_manager_state = CursorShapeManagerState::new::<Self>(&display_handle);
		let session_env = crate::session::SessionEnvironment::new(&config);

		let keybind_manager = KeybindManager::new(config.keybind.clone());
		let output_manager = OutputManager::new();
		let output_manager_state = OutputManagerState::new_with_xdg_output::<Self>(&display_handle);
		let layer_shell_manager = LayerShellManager::new(&display_handle);
		let session_lock_manager = SessionLockManager::new(&display_handle);

		let workspaces = WorkspaceSet::new();
		let layout_engine = LayoutEngine::new();
		let mut output_workspaces = crate::workspace::OutputWorkspaces::new();
		let default_output = crate::output::OutputId::next();
		output_workspaces.add_output(default_output, None);
		let wallpaper = WallpaperState::new(&config.wallpaper);
		let damage_tracker = DamageTracker::new(1920, 1080);
		let repeat_delay = i32::try_from(config.input.repeat_delay).unwrap_or(i32::MAX);
		let repeat_rate = i32::try_from(config.input.repeat_rate).unwrap_or(i32::MAX);

		let keyboard = seat.add_keyboard(smithay::input::keyboard::XkbConfig::default(), repeat_delay, repeat_rate)?;
		let pointer = seat.add_pointer();

		let (config_sender, config_receiver) = std::sync::mpsc::channel::<crate::config::ConfigEvent>();
		let config_manager = crate::config::ConfigManager::new_test(config_sender.clone());

		Ok(Self {
			compositor_state,
			xdg_shell_state,
			shm_state,
			seat_state,
			_seat: seat,
			focused_surface: None,
			needs_redraw: false,
			idle_notifier_state,
			windows: HashMap::new(),
			surface_to_window: HashMap::new(),
			window_order: Vec::new(),
			layout_engine,
			workspaces,
			output_size: Size::from((1920, 1080)),
			config,
			wallpaper,
			keybind_manager,
			output_manager,
			_output_manager_state: output_manager_state,
			_wl_output_globals: HashMap::new(),
			smithay_outputs: HashMap::new(),
			layer_shell_manager,
			session_lock_manager,
			_fractional_scale_state: fractional_scale_state,
			_input_method_state: input_method_state,
			_presentation_state: presentation_state,
			_foreign_toplevel_list_state: foreign_toplevel_list_state,
			foreign_toplevel_handles: HashMap::new(),
			_image_capture_source_state: image_capture_source_state,
			_output_capture_source_state: output_capture_source_state,
			_image_copy_capture_state: image_copy_capture_state,
			screencopy_sessions: Vec::new(),
			xdg_activation_state: smithay::wayland::xdg_activation::XdgActivationState::new::<Self>(&display_handle),
			xwayland: None,
			xwm: None,
			xwayland_client: None,
			xwayland_shell_state,
			_xdg_decoration_state: xdg_decoration_state,
			_text_input_manager_state: text_input_manager_state,
			_xdg_system_bell_state: xdg_system_bell_state,
			_xdg_toplevel_icon_manager: xdg_toplevel_icon_manager,
			_cursor_shape_manager_state: cursor_shape_manager_state,
			session_env,
			damage_tracker,

			idle_timeout_secs: 300,
			is_idle: false,

			data_device_state,
			primary_selection_state,
			_viewporter_state: viewporter_state,
			output_workspaces: output_workspaces,
			backend: None,
			should_stop: false,
			frame_interval: None,
			frame_timer_token: None,
			window_size: None,
			display_handle,
			keyboard,
			pointer,
			pointer_location: Point::from((960.0, 540.0)),
			serial_counter: 0,
			start_time: Instant::now(),
			ipc_receiver: None,
			ipc_event_emitter: None,
			_config_manager: config_manager,
			config_sender,
			config_receiver,
			output_id: None,
			last_winit_capture: None,
		})
	}
}
