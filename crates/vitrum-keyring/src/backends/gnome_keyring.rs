use anyhow::{Context, Result};
use std::process::Command;
use tracing::{info, warn};
use vitrum_config::KeyringConfig;

pub fn start(config: &KeyringConfig) -> Result<Option<Vec<(String, String)>>> {
	let daemon_path = match which::which("gnome-keyring-daemon") {
		Ok(path) => path,
		Err(_) => return Ok(None),
	};

	let components_arg = format!("--components={}", config.components.join(","));

	let mut cmd = Command::new(&daemon_path);
	cmd.arg("--start").arg(&components_arg);

	let output = cmd.output().context("Failed to spawn gnome-keyring-daemon")?;

	if !output.status.success() {
		let stderr = String::from_utf8_lossy(&output.stderr);
		warn!("gnome-keyring-daemon failed to start: {}", stderr);
		return Ok(None);
	}

	let stdout = String::from_utf8_lossy(&output.stdout);
	let env_vars = parse_stdout(&stdout);

	info!("gnome-keyring-daemon started successfully");
	Ok(Some(env_vars))
}

fn parse_stdout(stdout: &str) -> Vec<(String, String)> {
	let mut env_vars = Vec::new();
	for line in stdout.lines() {
		if let Some((key, value)) = line.split_once('=') {
			let key = key.trim().to_string();
			let value = value.trim().trim_end_matches(';').to_string();
			env_vars.push((key, value));
		}
	}
	env_vars
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_parse_stdout() {
		let output = "SSH_AUTH_SOCK=/run/user/1000/keyring/ssh\nGNOME_KEYRING_CONTROL=/run/user/1000/keyring\n";
		let vars = parse_stdout(output);
		assert_eq!(vars.len(), 2);
		assert_eq!(vars[0].0, "SSH_AUTH_SOCK");
		assert_eq!(vars[0].1, "/run/user/1000/keyring/ssh");
		assert_eq!(vars[1].0, "GNOME_KEYRING_CONTROL");
		assert_eq!(vars[1].1, "/run/user/1000/keyring");

		let output_with_export = "SSH_AUTH_SOCK=/tmp/ssh.sock;\nexport SSH_AUTH_SOCK;\n";
		let vars = parse_stdout(output_with_export);
		assert_eq!(vars.len(), 1);
		assert_eq!(vars[0].0, "SSH_AUTH_SOCK");
		assert_eq!(vars[0].1, "/tmp/ssh.sock");
	}
}
