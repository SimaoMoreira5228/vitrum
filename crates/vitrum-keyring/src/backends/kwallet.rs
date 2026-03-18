use anyhow::Result;
use std::process::Command;
use tracing::{info, warn};
use vitrum_config::KeyringConfig;

pub fn start(_config: &KeyringConfig) -> Result<Option<Vec<(String, String)>>> {
	let daemon_path = match which::which("kwalletd6").or_else(|_| which::which("kwalletd5")) {
		Ok(path) => path,
		Err(_) => return Ok(None),
	};

	let mut cmd = Command::new(&daemon_path);

	match cmd.spawn() {
		Ok(_) => {
			info!("kwalletd started successfully");

			Ok(Some(vec![]))
		}
		Err(e) => {
			warn!("kwalletd failed to start: {}", e);
			Ok(None)
		}
	}
}
