use anyhow::{Context, Result, anyhow};
use clap::Parser;
use image::{ImageBuffer, Rgba};
use std::fs::File;
use std::os::unix::fs::FileExt;
use std::os::unix::io::AsRawFd;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use wayland_client::protocol::{wl_buffer, wl_output, wl_registry, wl_shm, wl_shm_pool};
use wayland_client::{Connection, Dispatch, QueueHandle};

use wayland_protocols::ext::image_capture_source::v1::client::{
	ext_image_capture_source_v1, ext_output_image_capture_source_manager_v1,
};
use wayland_protocols::ext::image_copy_capture::v1::client::{
	ext_image_copy_capture_frame_v1, ext_image_copy_capture_manager_v1, ext_image_copy_capture_session_v1,
};

#[derive(Parser)]
struct Args {
	#[arg(short, long, help = "Save output to a file instead of clipboard")]
	file: Option<String>,

	#[arg(short, long, help = "Select output by name (e.g. eDP-1)")]
	name: Option<String>,

	#[arg(short, long, help = "Delay in seconds before taking screenshot")]
	delay: Option<u64>,

	#[arg(short, long, help = "List available outputs and exit")]
	list_outputs: bool,
}

struct OutputInfo {
	proxy: wl_output::WlOutput,
	name: Option<String>,
	width: i32,
	height: i32,
}

struct AppState {
	shm: Option<wl_shm::WlShm>,
	manager: Option<ext_image_copy_capture_manager_v1::ExtImageCopyCaptureManagerV1>,
	source_manager: Option<ext_output_image_capture_source_manager_v1::ExtOutputImageCaptureSourceManagerV1>,
	outputs: Vec<OutputInfo>,
	capture_result: Arc<Mutex<Option<CaptureData>>>,

	width: u32,
	height: u32,
	tmp_file: Option<File>,

	exit: bool,
}

struct CaptureData {
	width: u32,
	height: u32,
	data: Vec<u8>,
}

impl Dispatch<wl_registry::WlRegistry, ()> for AppState {
	fn event(
		state: &mut Self,
		proxy: &wl_registry::WlRegistry,
		event: wl_registry::Event,
		_: &(),
		_: &Connection,
		qh: &QueueHandle<Self>,
	) {
		if let wl_registry::Event::Global {
			name,
			interface,
			version,
		} = event
		{
			match interface.as_str() {
				"wl_shm" => {
					state.shm = Some(proxy.bind::<wl_shm::WlShm, _, _>(name, version, qh, ()));
				}
				"ext_image_copy_capture_manager_v1" => {
					state.manager = Some(
						proxy.bind::<ext_image_copy_capture_manager_v1::ExtImageCopyCaptureManagerV1, _, _>(
							name,
							version,
							qh,
							(),
						),
					);
				}
				"ext_output_image_capture_source_manager_v1" => {
					state.source_manager = Some(
						proxy
							.bind::<ext_output_image_capture_source_manager_v1::ExtOutputImageCaptureSourceManagerV1, _, _>(
								name,
								version,
								qh,
								(),
							),
					);
				}
				"wl_output" => {
					state.outputs.push(OutputInfo {
						proxy: proxy.bind(name, version.max(4), qh, ()),
						name: None,
						width: 0,
						height: 0,
					});
				}
				_ => {}
			}
		}
	}
}

impl Dispatch<wl_shm::WlShm, ()> for AppState {
	fn event(_: &mut Self, _: &wl_shm::WlShm, _: wl_shm::Event, _: &(), _: &Connection, _: &QueueHandle<Self>) {}
}

impl Dispatch<wl_output::WlOutput, ()> for AppState {
	fn event(
		state: &mut Self,
		proxy: &wl_output::WlOutput,
		event: wl_output::Event,
		_: &(),
		_: &Connection,
		_: &QueueHandle<Self>,
	) {
		match event {
			wl_output::Event::Mode { width, height, .. } => {
				if let Some(info) = state.outputs.iter_mut().find(|o| &o.proxy == proxy) {
					info.width = width;
					info.height = height;
				}
			}
			wl_output::Event::Name { name } => {
				if let Some(info) = state.outputs.iter_mut().find(|o| &o.proxy == proxy) {
					info.name = Some(name);
				}
			}
			_ => {}
		}
	}
}

impl Dispatch<ext_image_copy_capture_manager_v1::ExtImageCopyCaptureManagerV1, ()> for AppState {
	fn event(
		_: &mut Self,
		_: &ext_image_copy_capture_manager_v1::ExtImageCopyCaptureManagerV1,
		_: ext_image_copy_capture_manager_v1::Event,
		_: &(),
		_: &Connection,
		_: &QueueHandle<Self>,
	) {
	}
}

impl Dispatch<ext_image_copy_capture_session_v1::ExtImageCopyCaptureSessionV1, ()> for AppState {
	fn event(
		_: &mut Self,
		_: &ext_image_copy_capture_session_v1::ExtImageCopyCaptureSessionV1,
		_: ext_image_copy_capture_session_v1::Event,
		_: &(),
		_: &Connection,
		_: &QueueHandle<Self>,
	) {
	}
}

impl Dispatch<ext_image_copy_capture_frame_v1::ExtImageCopyCaptureFrameV1, ()> for AppState {
	fn event(
		state: &mut Self,
		_proxy: &ext_image_copy_capture_frame_v1::ExtImageCopyCaptureFrameV1,
		event: ext_image_copy_capture_frame_v1::Event,
		_: &(),
		_: &Connection,
		_: &QueueHandle<Self>,
	) {
		match event {
			ext_image_copy_capture_frame_v1::Event::Ready { .. } => {
				let width = state.width;
				let height = state.height;
				let file = state.tmp_file.as_ref().unwrap().try_clone().unwrap();
				let mut buf = vec![0u8; (width * height * 4) as usize];
				file.read_exact_at(&mut buf, 0).unwrap();

				let mut result = state.capture_result.lock().unwrap();
				*result = Some(CaptureData {
					width,
					height,
					data: buf,
				});
				state.exit = true;
			}
			ext_image_copy_capture_frame_v1::Event::Failed { reason } => {
				eprintln!("Capture failed: {:?}", reason);
				state.exit = true;
			}
			_ => {}
		}
	}
}

impl Dispatch<ext_output_image_capture_source_manager_v1::ExtOutputImageCaptureSourceManagerV1, ()> for AppState {
	fn event(
		_: &mut Self,
		_: &ext_output_image_capture_source_manager_v1::ExtOutputImageCaptureSourceManagerV1,
		_: ext_output_image_capture_source_manager_v1::Event,
		_: &(),
		_: &Connection,
		_: &QueueHandle<Self>,
	) {
	}
}

impl Dispatch<ext_image_capture_source_v1::ExtImageCaptureSourceV1, ()> for AppState {
	fn event(
		_: &mut Self,
		_: &ext_image_capture_source_v1::ExtImageCaptureSourceV1,
		_: ext_image_capture_source_v1::Event,
		_: &(),
		_: &Connection,
		_: &QueueHandle<Self>,
	) {
	}
}

impl Dispatch<wl_shm_pool::WlShmPool, ()> for AppState {
	fn event(
		_: &mut Self,
		_: &wl_shm_pool::WlShmPool,
		_: wl_shm_pool::Event,
		_: &(),
		_: &Connection,
		_: &QueueHandle<Self>,
	) {
	}
}

impl Dispatch<wl_buffer::WlBuffer, ()> for AppState {
	fn event(_: &mut Self, _: &wl_buffer::WlBuffer, _: wl_buffer::Event, _: &(), _: &Connection, _: &QueueHandle<Self>) {}
}

#[tokio::main]
async fn main() -> Result<()> {
	let args = Args::parse();

	let conn = Connection::connect_to_env().context("Failed to connect to Wayland")?;
	let display = conn.display();
	let mut event_queue = conn.new_event_queue();
	let qh = event_queue.handle();

	let mut state = AppState {
		shm: None,
		manager: None,
		source_manager: None,
		outputs: Vec::new(),
		capture_result: Arc::new(Mutex::new(None)),
		width: 0,
		height: 0,
		tmp_file: None,
		exit: false,
	};

	let _registry = display.get_registry(&qh, ());

	event_queue.roundtrip(&mut state).context("Failed initial roundtrip")?;
	event_queue.roundtrip(&mut state).context("Failed global bind roundtrip")?;
	event_queue.roundtrip(&mut state).context("Failed geometry roundtrip")?;

	if args.list_outputs {
		println!("Available outputs:");
		for info in &state.outputs {
			println!(
				"  {} ({}x{})",
				info.name.as_deref().unwrap_or("<unknown>"),
				info.width,
				info.height
			);
		}
		return Ok(());
	}

	if let Some(delay) = args.delay {
		println!("Taking screenshot in {} seconds...", delay);
		tokio::time::sleep(Duration::from_secs(delay)).await;
	}

	let shm = state.shm.as_ref().context("wl_shm not available")?;
	let manager = state
		.manager
		.as_ref()
		.context("ext_image_copy_capture_manager_v1 not available")?;
	let source_manager = state
		.source_manager
		.as_ref()
		.context("ext_output_image_capture_source_manager_v1 not available")?;

	let output_info = if let Some(target_name) = &args.name {
		state
			.outputs
			.iter()
			.find(|o| o.name.as_ref() == Some(target_name))
			.ok_or_else(|| anyhow!("Output '{}' not found", target_name))?
	} else {
		state.outputs.first().context("No output available")?
	};

	if output_info.width == 0 || output_info.height == 0 {
		return Err(anyhow!("Output size is unknown. Try running again."));
	}

	state.width = output_info.width as u32;
	state.height = output_info.height as u32;

	let source = source_manager.create_source(&output_info.proxy, &qh, ());
	let options = ext_image_copy_capture_manager_v1::Options::empty();
	let session = manager.create_session(&source, options, &qh, ());

	let stride = state.width * 4;
	let size = stride * state.height;

	let tmp_file = tempfile::tempfile().context("Failed to create tempfile")?;
	tmp_file.set_len(size as u64).context("Failed to set tempfile size")?;
	state.tmp_file = Some(tmp_file.try_clone()?);

	let raw_fd = tmp_file.as_raw_fd();
	use std::os::fd::BorrowedFd;
	let pool = shm.create_pool(unsafe { BorrowedFd::borrow_raw(raw_fd) }, size as i32, &qh, ());
	let buffer = pool.create_buffer(
		0,
		state.width as i32,
		state.height as i32,
		stride as i32,
		wl_shm::Format::Xrgb8888,
		&qh,
		(),
	);

	let frame = session.create_frame(&qh, ());
	frame.attach_buffer(&buffer);
	frame.capture();

	while !state.exit {
		event_queue
			.blocking_dispatch(&mut state)
			.context("Failed to dispatch events")?;
	}

	let result = state.capture_result.lock().unwrap().take();
	if let Some(capture) = result {
		let mut img: ImageBuffer<Rgba<u8>, Vec<u8>> =
			ImageBuffer::from_raw(capture.width, capture.height, capture.data).context("Failed to create image buffer")?;

		for pixel in img.pixels_mut() {
			let [b, g, r, a] = pixel.0;
			pixel.0 = [r, g, b, a];
		}

		if let Some(path) = args.file {
			img.save(&path).context("Failed to save image")?;
			println!("Screenshot saved to {} ({}x{})", path, capture.width, capture.height);
		} else {
			let mut clipboard = arboard::Clipboard::new().context("Failed to initialize clipboard")?;
			let image_data = arboard::ImageData {
				width: capture.width as usize,
				height: capture.height as usize,
				bytes: std::borrow::Cow::from(img.as_raw()),
			};
			clipboard.set_image(image_data).context("Failed to set image to clipboard")?;
			println!("Screenshot copied to clipboard ({}x{})", capture.width, capture.height);
		}
	} else {
		return Err(anyhow!("No capture data received"));
	}

	Ok(())
}
