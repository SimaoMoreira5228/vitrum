pub mod drm;
pub mod headless;
pub mod render;
pub mod winit;

pub mod handlers;
pub mod state;
pub mod state_methods;
pub mod types;
pub mod utils;

#[cfg(test)]
mod tests;

pub use handlers::ClientState;
pub use state::State;
pub use types::{Backend, CapturedFrame};

smithay::delegate_compositor!(State);
smithay::delegate_xwayland_shell!(State);
smithay::delegate_xdg_shell!(State);
smithay::delegate_xdg_decoration!(State);
smithay::delegate_shm!(State);
smithay::delegate_seat!(State);
smithay::delegate_text_input_manager!(State);
smithay::delegate_xdg_toplevel_icon!(State);
smithay::delegate_xdg_system_bell!(State);
smithay::delegate_cursor_shape!(State);
smithay::delegate_viewporter!(State);
smithay::delegate_idle_notify!(State);
smithay::delegate_fractional_scale!(State);
smithay::delegate_input_method_manager!(State);
smithay::delegate_presentation!(State);
smithay::delegate_data_device!(State);
smithay::delegate_primary_selection!(State);
smithay::delegate_output!(State);
smithay::delegate_foreign_toplevel_list!(State);
smithay::delegate_image_capture_source!(State);
smithay::delegate_output_capture_source!(State);
smithay::delegate_image_copy_capture!(State);
