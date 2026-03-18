use smithay::backend::allocator::Fourcc;
use smithay::backend::renderer::gles::GlesRenderer;
use smithay::backend::winit::WinitGraphicsBackend;

pub enum Backend {
	Winit(WinitGraphicsBackend<GlesRenderer>),
	Drm(crate::backend::drm::DrmBackend),
	Headless,
}

impl std::fmt::Debug for Backend {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Backend::Winit(_) => write!(f, "Backend::Winit"),
			Backend::Drm(_) => write!(f, "Backend::Drm"),
			Backend::Headless => write!(f, "Backend::Headless"),
		}
	}
}

#[derive(Debug, Clone)]
pub struct CapturedFrame {
	pub width: u32,
	pub height: u32,
	pub format: Fourcc,
	pub flipped: bool,
	pub data: Vec<u8>,
}
