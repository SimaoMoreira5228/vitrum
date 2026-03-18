use std::collections::HashMap;
use std::env;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{OnceLock, RwLock};
use std::thread;

use anyhow::{Result, anyhow};
use tracing::{error, info, warn};

const ENV_KEYS: &[&str] = &[
	"PATH",
	"WAYLAND_DISPLAY",
	"DISPLAY",
	"XDG_RUNTIME_DIR",
	"XDG_SESSION_TYPE",
	"XDG_CURRENT_DESKTOP",
	"DESKTOP_SESSION",
	"SHELL",
	"HOME",
	"USER",
];

static WAYLAND_DISPLAY_SOCKET: OnceLock<String> = OnceLock::new();
static SESSION_ENV: OnceLock<RwLock<HashMap<String, String>>> = OnceLock::new();

pub fn set_wayland_display_socket(socket: String) {
	let _ = WAYLAND_DISPLAY_SOCKET.set(socket);
}

pub fn set_session_env(env_map: HashMap<String, String>) {
	let lock = SESSION_ENV.get_or_init(|| RwLock::new(HashMap::new()));
	if let Ok(mut guard) = lock.write() {
		*guard = env_map;
	}
}

fn wayland_display_socket() -> Option<&'static str> {
	WAYLAND_DISPLAY_SOCKET.get().map(|s| s.as_str())
}

fn session_env() -> Option<HashMap<String, String>> {
	SESSION_ENV.get().and_then(|lock| lock.read().ok()).map(|guard| guard.clone())
}

pub fn spawn_command(command: &str, source: &str) -> Result<()> {
	if command.trim().is_empty() {
		let snapshot = env_snapshot();
		error!(source = %source, env = %snapshot, "Refusing to spawn empty command");
		return Err(anyhow!("empty command"));
	}

	let maybe_exec = simple_executable_name(command);
	if let Some(exec) = maybe_exec {
		if let Some(resolved) = resolve_executable(exec) {
			info!(source = %source, command = %command, executable = %resolved.display(), "Launcher preflight resolved executable");
		} else {
			let snapshot = env_snapshot();
			error!(
				source = %source,
				command = %command,
				executable = %exec,
				env = %snapshot,
				"Launcher preflight failed: executable not found in PATH"
			);
			return Err(anyhow!("executable '{}' not found in PATH", exec));
		}
	} else {
		warn!(source = %source, command = %command, "Skipping executable preflight for shell expression");
	}

	let mut process = Command::new("sh");
	process.arg("-c").arg(command).stdin(Stdio::null());

	if let Some(ref env_map) = session_env() {
		process.envs(env_map.iter());
	} else {
		if let Some(socket) = wayland_display_socket() {
			process.env("WAYLAND_DISPLAY", socket);
			process.env("XDG_SESSION_TYPE", "wayland");
		}
	}

	match process.spawn() {
		Ok(mut child) => {
			let pid = child.id();
			info!(
				source = %source,
				command = %command,
				pid,
				"Spawned command"
			);

			let command_owned = command.to_string();
			let source_owned = source.to_string();
			let _ = thread::Builder::new()
				.name("vitrum-child-wait".to_string())
				.spawn(move || match child.wait() {
					Ok(status) if !status.success() => {
						warn!(
							source = %source_owned,
							command = %command_owned,
							pid,
							status = ?status,
							"Spawned command exited with non-zero status"
						);
					}
					Ok(status) => {
						info!(
							source = %source_owned,
							command = %command_owned,
							pid,
							status = ?status,
							"Spawned command exited"
						);
					}
					Err(err) => {
						warn!(
							source = %source_owned,
							command = %command_owned,
							pid,
							error = %err,
							"Failed waiting for spawned command"
						);
					}
				});
			Ok(())
		}
		Err(err) => {
			let snapshot = env_snapshot();
			error!(
				source = %source,
				command = %command,
				error = %err,
				env = %snapshot,
				"Failed to spawn command"
			);
			Err(anyhow!(err))
		}
	}
}

fn env_snapshot() -> String {
	ENV_KEYS
		.iter()
		.map(|key| {
			let value = env::var(key).unwrap_or_else(|_| "<unset>".to_string());
			format!("{}={}", key, value)
		})
		.collect::<Vec<_>>()
		.join(" ")
}

fn simple_executable_name(command: &str) -> Option<&str> {
	let trimmed = command.trim();
	if trimmed.contains('|')
		|| trimmed.contains('&')
		|| trimmed.contains(';')
		|| trimmed.contains('>')
		|| trimmed.contains('<')
		|| trimmed.contains('$')
		|| trimmed.contains('`')
		|| trimmed.contains('(')
		|| trimmed.contains(')')
		|| trimmed.contains('"')
		|| trimmed.contains('\'')
	{
		return None;
	}

	trimmed.split_whitespace().next()
}

fn resolve_executable(executable: &str) -> Option<PathBuf> {
	if executable.contains('/') {
		let path = PathBuf::from(executable);
		return is_executable_file(&path).then_some(path);
	}

	let path_env = env::var_os("PATH")?;
	for dir in env::split_paths(&path_env) {
		let candidate = dir.join(executable);
		if is_executable_file(&candidate) {
			return Some(candidate);
		}
	}

	None
}

fn is_executable_file(path: &Path) -> bool {
	let Ok(metadata) = path.metadata() else {
		return false;
	};
	if !metadata.is_file() {
		return false;
	}

	#[cfg(unix)]
	{
		use std::os::unix::fs::PermissionsExt;
		metadata.permissions().mode() & 0o111 != 0
	}

	#[cfg(not(unix))]
	{
		true
	}
}
