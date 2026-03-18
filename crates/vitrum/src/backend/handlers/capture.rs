use smithay::output::Output;
use smithay::reexports::wayland_server::protocol::wl_shm;
use smithay::wayland::image_capture_source::{ImageCaptureSource, ImageCaptureSourceHandler, OutputCaptureSourceHandler};
use smithay::wayland::image_copy_capture::{
	BufferConstraints, Frame, ImageCopyCaptureHandler, ImageCopyCaptureState, Session, SessionRef,
};

use super::super::utils::{fill_checker_to_shm_buffer, write_capture_to_shm_buffer};
use crate::backend::State;

impl ImageCaptureSourceHandler for State {}

impl OutputCaptureSourceHandler for State {
	fn output_capture_source_state(&mut self) -> &mut smithay::wayland::image_capture_source::OutputCaptureSourceState {
		&mut self._output_capture_source_state
	}

	fn output_source_created(&mut self, source: ImageCaptureSource, output: &Output) {
		source.user_data().insert_if_missing(|| output.downgrade());
	}
}

impl ImageCopyCaptureHandler for State {
	fn image_copy_capture_state(&mut self) -> &mut ImageCopyCaptureState {
		&mut self._image_copy_capture_state
	}

	fn capture_constraints(&mut self, source: &ImageCaptureSource) -> Option<BufferConstraints> {
		let weak_output = source.user_data().get::<smithay::output::WeakOutput>()?;
		let output = weak_output.upgrade()?;

		let size = self
			.output_manager
			.map()
			.get_by_name(output.name().as_str())
			.map(|o| o.size)
			.unwrap_or(self.output_size);

		Some(BufferConstraints {
			size: (size.w, size.h).into(),
			shm: vec![wl_shm::Format::Xrgb8888, wl_shm::Format::Argb8888],
			dma: None,
		})
	}

	fn new_session(&mut self, session: Session) {
		self._image_copy_capture_state.cleanup();
		self.screencopy_sessions.push(session);
	}

	fn frame(&mut self, _session: &SessionRef, frame: Frame) {
		let buffer = frame.buffer();

		let write_result = if let Some(capture) = self.last_winit_capture.as_ref() {
			write_capture_to_shm_buffer(capture, &buffer)
		} else {
			fill_checker_to_shm_buffer(&buffer)
		};

		if let Err(reason) = write_result {
			frame.fail(reason);
			return;
		}

		frame.success(smithay::utils::Transform::Normal, None, self.start_time.elapsed());
	}

	fn session_destroyed(&mut self, session: SessionRef) {
		self.screencopy_sessions.retain(|active| active != &session);
	}
}
