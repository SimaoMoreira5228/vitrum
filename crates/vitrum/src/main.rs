mod autostart;
mod backend;
mod cli;
mod compositor;
mod config;
mod damage;
mod ipc_handler;
mod keybind;
mod launcher;
mod layer_shell;
mod layout;
mod output;
mod session;
mod session_lock;
mod session_services;
mod wallpaper;
mod wayland_runtime;
mod window;
mod workspace;

#[cfg(test)]
mod tests;

use anyhow::Result;
use clap::Parser;
use tracing::{error, info};

#[tokio::main]
async fn main() -> Result<()> {
	init_tracing();

	let cli = cli::Cli::parse();
	info!(backend = %cli.backend, "starting vitrum");

	if let Err(err) = compositor::run(cli.backend, cli.max_fps).await {
		error!(error = %err, "compositor failed");
		return Err(err);
	}

	Ok(())
}

fn init_tracing() {
	let filter =
		tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));

	tracing_subscriber::fmt()
		.with_env_filter(filter)
		.with_target(true)
		.compact()
		.init();
}
