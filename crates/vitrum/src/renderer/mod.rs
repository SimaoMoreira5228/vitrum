#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderBackend {
	Gles,

	Vulkan,
}

impl std::fmt::Display for RenderBackend {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			RenderBackend::Gles => write!(f, "GLES"),
			RenderBackend::Vulkan => write!(f, "Vulkan"),
		}
	}
}
