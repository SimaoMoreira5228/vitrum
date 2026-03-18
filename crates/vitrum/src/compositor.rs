use anyhow::Result;
use tracing::info;

use crate::backend::drm;
use crate::backend::headless;
use crate::backend::winit;
use crate::cli::Backend as CliBackend;

pub async fn run(backend_type: CliBackend, max_fps: Option<u16>) -> Result<()> {
	info!("Starting vitrum compositor");

	match backend_type {
		CliBackend::Winit => run_winit(max_fps).await,
		CliBackend::Drm => run_drm(max_fps).await,
		CliBackend::Headless => run_headless(max_fps).await,
	}
}

async fn run_winit(max_fps: Option<u16>) -> Result<()> {
	winit::run(max_fps)
}

async fn run_drm(max_fps: Option<u16>) -> Result<()> {
	drm::run(max_fps)
}

async fn run_headless(max_fps: Option<u16>) -> Result<()> {
	headless::run(max_fps)
}
