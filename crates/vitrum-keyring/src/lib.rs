pub mod backends;

use anyhow::Result;
use tracing::{info, warn};
use vitrum_config::KeyringConfig;

pub fn start(config: &KeyringConfig) -> Result<Vec<(String, String)>> {
	info!("Starting keyring manager");

	let order: Vec<&str> = match config.prefer.as_str() {
		"gnome-keyring" => vec!["gnome-keyring"],
		"kwallet" => vec!["kwallet"],
		"keepassxc" => vec!["keepassxc"],
		"none" => vec![],
		"auto" | _ => vec!["keepassxc", "gnome-keyring", "kwallet"],
	};

	let mut env_vars = vec![];
	let mut started = false;

	for backend in order {
		match backend {
			"gnome-keyring" => match backends::gnome_keyring::start(config) {
				Ok(Some(vars)) => {
					env_vars = vars;
					started = true;
					break;
				}
				Ok(None) => continue,
				Err(e) => warn!("gnome-keyring backend error: {}", e),
			},
			"kwallet" => match backends::kwallet::start(config) {
				Ok(Some(vars)) => {
					env_vars = vars;
					started = true;
					break;
				}
				Ok(None) => continue,
				Err(e) => warn!("kwallet backend error: {}", e),
			},
			"keepassxc" => match backends::keepassxc::start(config) {
				Ok(Some(vars)) => {
					env_vars = vars;
					started = true;
					break;
				}
				Ok(None) => continue,
				Err(e) => warn!("keepassxc backend error: {}", e),
			},
			_ => {}
		}
	}

	if !started && config.prefer != "none" {
		warn!("No supported keyring backend could be started in '{:?}'", config.prefer);
	}

	Ok(env_vars)
}
