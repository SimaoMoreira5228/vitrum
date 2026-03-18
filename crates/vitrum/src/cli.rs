use clap::{Parser, ValueEnum};

#[derive(Debug, Clone, Parser)]
#[command(name = "vitrum", version, about = "Vitrum Wayland compositor")]
pub struct Cli {
	#[arg(long, value_enum, default_value_t = Backend::Winit)]
	pub backend: Backend,

	#[arg(long, value_name = "FPS")]
	pub max_fps: Option<u16>,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, ValueEnum)]
pub enum Backend {
	Winit,
	Drm,
	Headless,
}

impl core::fmt::Display for Backend {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		let text = match self {
			Backend::Winit => "winit",
			Backend::Drm => "drm",
			Backend::Headless => "headless",
		};

		f.write_str(text)
	}
}
