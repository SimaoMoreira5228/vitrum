use smithay::backend::allocator::Fourcc;
use smithay::reexports::wayland_server::protocol::wl_shm;
use smithay::wayland::image_copy_capture::CaptureFailureReason;
use smithay::wayland::shm::with_buffer_contents_mut;

use super::types::CapturedFrame;

pub fn write_capture_to_shm_buffer(
	capture: &CapturedFrame,
	buffer: &smithay::reexports::wayland_server::protocol::wl_buffer::WlBuffer,
) -> Result<(), CaptureFailureReason> {
	if capture.format != Fourcc::Argb8888 {
		return Err(CaptureFailureReason::BufferConstraints);
	}

	with_buffer_contents_mut(buffer, |ptr, len, data| {
		let bytes_per_pixel = match data.format {
			wl_shm::Format::Xrgb8888 | wl_shm::Format::Argb8888 => 4usize,
			_ => return Err(CaptureFailureReason::BufferConstraints),
		};

		let offset = usize::try_from(data.offset).map_err(|_| CaptureFailureReason::BufferConstraints)?;
		let width = usize::try_from(data.width).map_err(|_| CaptureFailureReason::BufferConstraints)?;
		let height = usize::try_from(data.height).map_err(|_| CaptureFailureReason::BufferConstraints)?;
		let stride = usize::try_from(data.stride).map_err(|_| CaptureFailureReason::BufferConstraints)?;

		if stride < width.saturating_mul(bytes_per_pixel) {
			return Err(CaptureFailureReason::BufferConstraints);
		}

		let required = offset
			.checked_add(stride.checked_mul(height).ok_or(CaptureFailureReason::BufferConstraints)?)
			.ok_or(CaptureFailureReason::BufferConstraints)?;

		if required > len {
			return Err(CaptureFailureReason::BufferConstraints);
		}

		let src_width = usize::try_from(capture.width).map_err(|_| CaptureFailureReason::BufferConstraints)?;
		let src_height = usize::try_from(capture.height).map_err(|_| CaptureFailureReason::BufferConstraints)?;
		if src_width == 0 || src_height == 0 {
			return Err(CaptureFailureReason::Unknown);
		}

		let src_row_len = src_width
			.checked_mul(bytes_per_pixel)
			.ok_or(CaptureFailureReason::BufferConstraints)?;

		if capture.data.len()
			< src_row_len
				.checked_mul(src_height)
				.ok_or(CaptureFailureReason::BufferConstraints)?
		{
			return Err(CaptureFailureReason::BufferConstraints);
		}

		let pool = unsafe { std::slice::from_raw_parts_mut(ptr, len) };

		for y in 0..height {
			let row_start = offset + y * stride;
			let row_end = row_start + width * bytes_per_pixel;
			let row = &mut pool[row_start..row_end];

			let mut src_y = y.saturating_mul(src_height) / height.max(1);
			if capture.flipped {
				src_y = src_height.saturating_sub(1).saturating_sub(src_y);
			}
			let src_row_start = src_y
				.checked_mul(src_row_len)
				.ok_or(CaptureFailureReason::BufferConstraints)?;
			let src_row = &capture.data[src_row_start..src_row_start + src_row_len];

			for x in 0..width {
				let px = x * bytes_per_pixel;
				let src_x = x.saturating_mul(src_width) / width.max(1);
				let src_px = src_x
					.checked_mul(bytes_per_pixel)
					.ok_or(CaptureFailureReason::BufferConstraints)?;

				row[px] = src_row[src_px];
				row[px + 1] = src_row[src_px + 1];
				row[px + 2] = src_row[src_px + 2];
				row[px + 3] = if matches!(data.format, wl_shm::Format::Xrgb8888) {
					0xFF
				} else {
					src_row[src_px + 3]
				};
			}
		}

		Ok(())
	})
	.map_err(|_| CaptureFailureReason::BufferConstraints)?
}

pub fn fill_checker_to_shm_buffer(
	buffer: &smithay::reexports::wayland_server::protocol::wl_buffer::WlBuffer,
) -> Result<(), CaptureFailureReason> {
	with_buffer_contents_mut(buffer, |ptr, len, data| {
		let bytes_per_pixel = match data.format {
			wl_shm::Format::Xrgb8888 | wl_shm::Format::Argb8888 => 4usize,
			_ => return Err(CaptureFailureReason::BufferConstraints),
		};

		let offset = usize::try_from(data.offset).map_err(|_| CaptureFailureReason::BufferConstraints)?;
		let width = usize::try_from(data.width).map_err(|_| CaptureFailureReason::BufferConstraints)?;
		let height = usize::try_from(data.height).map_err(|_| CaptureFailureReason::BufferConstraints)?;
		let stride = usize::try_from(data.stride).map_err(|_| CaptureFailureReason::BufferConstraints)?;

		if stride < width.saturating_mul(bytes_per_pixel) {
			return Err(CaptureFailureReason::BufferConstraints);
		}

		let required = offset
			.checked_add(stride.checked_mul(height).ok_or(CaptureFailureReason::BufferConstraints)?)
			.ok_or(CaptureFailureReason::BufferConstraints)?;

		if required > len {
			return Err(CaptureFailureReason::BufferConstraints);
		}

		let pool = unsafe { std::slice::from_raw_parts_mut(ptr, len) };

		for y in 0..height {
			let row_start = offset + y * stride;
			let row_end = row_start + width * bytes_per_pixel;
			let row = &mut pool[row_start..row_end];

			for x in 0..width {
				let px = x * bytes_per_pixel;
				let checker = ((x / 32) + (y / 32)) % 2 == 0;
				let (r, g, b) = if checker { (0x22, 0x22, 0x22) } else { (0x44, 0x44, 0x44) };

				row[px] = b;
				row[px + 1] = g;
				row[px + 2] = r;
				row[px + 3] = 0xFF;
			}
		}

		Ok(())
	})
	.map_err(|_| CaptureFailureReason::BufferConstraints)?
}
