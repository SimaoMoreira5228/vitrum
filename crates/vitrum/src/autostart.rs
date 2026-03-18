use std::collections::HashMap;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use tracing::{debug, error, info, warn};

#[derive(Debug, Clone)]
pub struct AutostartEntry {
	pub name: String,
	pub exec: String,
	pub try_exec: Option<String>,
	pub _icon: Option<String>,
	pub _comment: Option<String>,
	pub path: Option<PathBuf>,
	pub _terminal: bool,
	pub hidden: bool,
	pub only_show_in: Vec<String>,
	pub not_show_in: Vec<String>,
	pub _source: AutostartSource,
}

#[derive(Debug, Clone)]
pub enum AutostartSource {
	DesktopFile(PathBuf),
	Config,
}

impl AutostartEntry {
	pub fn should_show(&self, desktop_env: &str) -> bool {
		if self.hidden {
			return false;
		}

		if !self.only_show_in.is_empty() {
			return self.only_show_in.iter().any(|de| de.eq_ignore_ascii_case(desktop_env));
		}

		if !self.not_show_in.is_empty() {
			return !self.not_show_in.iter().any(|de| de.eq_ignore_ascii_case(desktop_env));
		}

		true
	}

	pub fn try_exec_ok(&self) -> bool {
		if let Some(ref try_exec) = self.try_exec {
			which::which(try_exec).is_ok()
		} else {
			true
		}
	}
}

pub struct AutostartManager {
	entries: Vec<AutostartEntry>,
	desktop_env: String,
	disabled: bool,
}

impl AutostartManager {
	pub fn new() -> Self {
		Self {
			entries: Vec::new(),
			desktop_env: std::env::var("XDG_CURRENT_DESKTOP").unwrap_or_else(|_| "vitrum".to_string()),
			disabled: false,
		}
	}

	pub fn with_config_entries(entries: Vec<AutostartEntry>) -> Self {
		let mut manager = Self::new();
		manager.entries = entries;
		manager
	}

	pub fn set_disabled(&mut self, disabled: bool) {
		self.disabled = disabled;
	}

	pub fn load_xdg_autostart(&mut self) -> Result<()> {
		let autostart_dirs = Self::autostart_dirs();

		for dir in autostart_dirs {
			if !dir.exists() {
				continue;
			}

			debug!(dir = %dir.display(), "Scanning autostart directory");

			for entry in std::fs::read_dir(&dir)? {
				let entry = entry?;
				let path = entry.path();

				if path.extension().map(|e| e == "desktop").unwrap_or(false) {
					match Self::parse_desktop_file(&path) {
						Ok(Some(autostart_entry)) => {
							debug!(
								name = %autostart_entry.name,
								exec = %autostart_entry.exec,
								"Loaded autostart entry"
							);
							self.entries.push(autostart_entry);
						}
						Ok(None) => {
							debug!(path = %path.display(), "Skipping non-autostart desktop file");
						}
						Err(e) => {
							warn!(path = %path.display(), error = %e, "Failed to parse desktop file");
						}
					}
				}
			}
		}

		info!(count = self.entries.len(), "Loaded XDG autostart entries");
		Ok(())
	}

	fn autostart_dirs() -> Vec<PathBuf> {
		let mut dirs = Vec::new();

		dirs.push(PathBuf::from("/etc/xdg/autostart"));

		if let Ok(config_home) = std::env::var("XDG_CONFIG_HOME") {
			dirs.push(PathBuf::from(config_home).join("autostart"));
		} else if let Ok(home) = std::env::var("HOME") {
			dirs.push(PathBuf::from(home).join(".config/autostart"));
		}

		dirs
	}

	fn parse_desktop_file(path: &Path) -> Result<Option<AutostartEntry>> {
		let content =
			std::fs::read_to_string(path).with_context(|| format!("Failed to read desktop file: {}", path.display()))?;

		let mut in_desktop_entry = false;
		let mut fields: HashMap<String, String> = HashMap::new();

		for line in content.lines() {
			let line = line.trim();

			if line == "[Desktop Entry]" {
				in_desktop_entry = true;
				continue;
			}

			if line.starts_with('[') && line.ends_with(']') {
				in_desktop_entry = false;
				continue;
			}

			if !in_desktop_entry {
				continue;
			}

			if let Some((key, value)) = line.split_once('=') {
				fields.insert(key.trim().to_string(), value.trim().to_string());
			}
		}

		let type_ = fields.get("Type").map(|s| s.as_str()).unwrap_or("");
		if type_ != "Application" {
			return Ok(None);
		}

		let no_display = fields.get("NoDisplay").map(|s| s == "true" || s == "1").unwrap_or(false);
		if no_display {
			return Ok(None);
		}

		let name = fields.get("Name").cloned().unwrap_or_else(|| "Unknown".to_string());
		let exec = fields.get("Exec").cloned();

		if exec.is_none() {
			return Ok(None);
		}

		let only_show_in = fields
			.get("OnlyShowIn")
			.map(|s| s.split(';').map(|s| s.to_string()).collect())
			.unwrap_or_default();
		let not_show_in = fields
			.get("NotShowIn")
			.map(|s| s.split(';').map(|s| s.to_string()).collect())
			.unwrap_or_default();

		let hidden = fields.get("Hidden").map(|s| s == "true" || s == "1").unwrap_or(false);

		let terminal = fields.get("Terminal").map(|s| s == "true" || s == "1").unwrap_or(false);

		Ok(Some(AutostartEntry {
			name,
			exec: exec.expect("exec is Some after None check"),
			try_exec: fields.get("TryExec").cloned(),
			_icon: fields.get("Icon").cloned(),
			_comment: fields.get("Comment").cloned(),
			path: fields.get("Path").map(PathBuf::from),
			_terminal: terminal,
			hidden,
			only_show_in,
			not_show_in,
			_source: AutostartSource::DesktopFile(path.to_path_buf()),
		}))
	}

	pub fn add_config_entry(&mut self, entry: AutostartEntry) {
		self.entries.push(entry);
	}

	pub fn execute(&self) {
		if self.disabled {
			info!("Autostart is disabled, skipping");
			return;
		}

		for entry in &self.entries {
			if !entry.should_show(&self.desktop_env) {
				debug!(name = %entry.name, "Skipping entry - not for current desktop environment");
				continue;
			}

			if !entry.try_exec_ok() {
				debug!(name = %entry.name, "Skipping entry - TryExec not satisfied");
				continue;
			}

			match Self::spawn_entry(entry) {
				Ok(_) => {
					info!(name = %entry.name, exec = %entry.exec, "Autostarted application");
				}
				Err(e) => {
					error!(name = %entry.name, error = %e, "Failed to autostart application");
				}
			}
		}
	}

	fn spawn_entry(entry: &AutostartEntry) -> Result<()> {
		let mut command = std::process::Command::new("sh");
		command.arg("-c").arg(&entry.exec);

		if let Some(ref path) = entry.path {
			command.current_dir(path);
		}

		command.env("VITRUM_AUTOSTART", "1");

		command.spawn().with_context(|| format!("Failed to spawn: {}", entry.exec))?;

		Ok(())
	}

	pub fn entries(&self) -> &[AutostartEntry] {
		&self.entries
	}

	pub fn clear(&mut self) {
		self.entries.clear();
	}
}

impl Default for AutostartManager {
	fn default() -> Self {
		Self::new()
	}
}

pub fn create_config_entry(name: impl Into<String>, cmd: impl Into<String>) -> AutostartEntry {
	let cmd = cmd.into();
	AutostartEntry {
		name: name.into(),
		exec: cmd.clone(),
		try_exec: None,
		_icon: None,
		_comment: None,
		path: None,
		_terminal: false,
		hidden: false,
		only_show_in: Vec::new(),
		not_show_in: Vec::new(),
		_source: AutostartSource::Config,
	}
}

pub fn init_autostart(config_entries: Vec<vitrum_config::AutostartEntry>, disabled: bool) -> AutostartManager {
	let mut manager = AutostartManager::new();
	manager.set_disabled(disabled);

	for entry in config_entries {
		manager.add_config_entry(create_config_entry(
			entry.cmd.clone(),
			format!("{} {}", entry.cmd, entry.args.join(" ")),
		));
	}

	if let Err(e) = manager.load_xdg_autostart() {
		warn!(error = %e, "Failed to load XDG autostart entries");
	}

	manager.execute();

	manager
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_autostart_entry_should_show() {
		let entry = AutostartEntry {
			name: "Test".to_string(),
			exec: "test".to_string(),
			try_exec: None,
			_icon: None,
			_comment: None,
			path: None,
			_terminal: false,
			hidden: false,
			only_show_in: vec!["GNOME".to_string()],
			not_show_in: vec![],
			_source: AutostartSource::Config,
		};

		assert!(entry.should_show("GNOME"));
		assert!(!entry.should_show("KDE"));

		let entry2 = AutostartEntry {
			name: "Test2".to_string(),
			exec: "test2".to_string(),
			try_exec: None,
			_icon: None,
			_comment: None,
			path: None,
			_terminal: false,
			hidden: false,
			only_show_in: vec![],
			not_show_in: vec!["KDE".to_string()],
			_source: AutostartSource::Config,
		};

		assert!(entry2.should_show("GNOME"));
		assert!(!entry2.should_show("KDE"));
	}

	#[test]
	fn test_autostart_entry_hidden() {
		let entry = AutostartEntry {
			name: "Hidden".to_string(),
			exec: "hidden".to_string(),
			try_exec: None,
			_icon: None,
			_comment: None,
			path: None,
			_terminal: false,
			hidden: true,
			only_show_in: vec![],
			not_show_in: vec![],
			_source: AutostartSource::Config,
		};

		assert!(!entry.should_show("GNOME"));
	}

	#[test]
	fn test_parse_desktop_file_content() {
		let content = r#"[Desktop Entry]
Name=Test Application
Exec=/usr/bin/test-app
Type=Application
Icon=test-icon
Comment=This is a test application
"#;

		let temp_dir = std::env::temp_dir();
		let temp_file = temp_dir.join("test-autostart.desktop");
		std::fs::write(&temp_file, content).unwrap();

		let entry = AutostartManager::parse_desktop_file(&temp_file).unwrap().unwrap();

		assert_eq!(entry.name, "Test Application");
		assert_eq!(entry.exec, "/usr/bin/test-app");
		assert_eq!(entry._icon, Some("test-icon".to_string()));
		assert_eq!(entry._comment, Some("This is a test application".to_string()));

		std::fs::remove_file(&temp_file).unwrap();
	}
}
