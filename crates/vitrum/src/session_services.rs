use std::path::PathBuf;
use std::process::{Command, Stdio};
use tracing::{info, warn};

pub fn start_session_services(config: &crate::config::Config) {
	info!("Starting session services");

	setup_portal_config();

	if !is_dbus_running() {
		start_dbus_session();
	} else {
		info!("D-Bus session bus already running");
	}

	match vitrum_keyring::start(&config.keyring) {
		Ok(vars) => {
			for (key, val) in vars {
				unsafe { std::env::set_var(&key, &val) };
			}
		}
		Err(e) => warn!("Failed to start keyring: {}", e),
	}

	if std::env::var("DBUS_SESSION_BUS_ADDRESS").is_err() {
		if let Some(runtime_dir) = std::env::var("XDG_RUNTIME_DIR").ok() {
			let bus_path = format!("{}/bus", runtime_dir);
			unsafe {
				std::env::set_var("DBUS_SESSION_BUS_ADDRESS", format!("unix:path={}", bus_path));
			}
			info!("Set DBUS_SESSION_BUS_ADDRESS to {}", bus_path);
		}
	}

	start_portal();

	start_portal_gtk();

	start_notify_daemon();

	import_activation_environment();
}

fn is_dbus_running() -> bool {
	std::env::var("DBUS_SESSION_BUS_ADDRESS").is_ok()
		&& std::path::Path::new(
			&std::env::var("XDG_RUNTIME_DIR")
				.map(|d| format!("{}/bus", d))
				.unwrap_or_default(),
		)
		.exists()
}

fn start_dbus_session() {
	info!("Starting D-Bus session bus");
	match Command::new("dbus-daemon")
		.args(["--session", "--address=unix:path=$XDG_RUNTIME_DIR/bus"])
		.env("DBUS_SESSION_BUS_ADDRESS", "")
		.stdout(Stdio::null())
		.stderr(Stdio::null())
		.spawn()
	{
		Ok(_) => {
			std::thread::sleep(std::time::Duration::from_millis(100));
			info!("D-Bus session bus started");
		}
		Err(e) => {
			warn!("Failed to start dbus-daemon: {}", e);
		}
	}
}

fn start_portal() {
	info!("Starting xdg-desktop-portal");
	match Command::new("xdg-desktop-portal")
		.env("XDG_CURRENT_DESKTOP", "Vitrum")
		.stdout(Stdio::null())
		.stderr(Stdio::null())
		.spawn()
	{
		Ok(_) => info!("xdg-desktop-portal started"),
		Err(e) => {
			if e.kind() == std::io::ErrorKind::NotFound {
				info!("xdg-desktop-portal not found, skipping");
			} else {
				warn!("Failed to start xdg-desktop-portal: {}", e);
			}
		}
	}
}

fn start_portal_gtk() {
	info!("Starting xdg-desktop-portal-gtk");
	match Command::new("xdg-desktop-portal-gtk")
		.env("XDG_CURRENT_DESKTOP", "Vitrum")
		.stdout(Stdio::null())
		.stderr(Stdio::null())
		.spawn()
	{
		Ok(_) => info!("xdg-desktop-portal-gtk started"),
		Err(e) => {
			if e.kind() == std::io::ErrorKind::NotFound {
				info!("xdg-desktop-portal-gtk not found, skipping");
			} else {
				warn!("Failed to start xdg-desktop-portal-gtk: {}", e);
			}
		}
	}
}

fn start_notify_daemon() {
	let bin = which::which("vitrum-notify").ok();

	if let Some(bin) = bin {
		info!(path = %bin.display(), "Starting vitrum-notify daemon");
		match Command::new(&bin).stdout(Stdio::null()).stderr(Stdio::null()).spawn() {
			Ok(_) => info!("vitrum-notify daemon started"),
			Err(e) => warn!("Failed to start vitrum-notify daemon: {}", e),
		}
	} else {
		info!("vitrum-notify daemon not found in PATH, skipping");
	}
}

fn import_activation_environment() {
	let variables = [
		"WAYLAND_DISPLAY",
		"DISPLAY",
		"XDG_CURRENT_DESKTOP",
		"XDG_SESSION_TYPE",
		"DBUS_SESSION_BUS_ADDRESS",
		"XCURSOR_THEME",
		"XCURSOR_SIZE",
		"GTK_THEME",
		"GTK_ICON_THEME",
		"GDK_BACKEND",
		"QT_QPA_PLATFORM",
		"QT_QPA_PLATFORMTHEME",
		"MOZ_ENABLE_WAYLAND",
		"ELECTRON_OZONE_PLATFORM_HINT",
		"FONTCONFIG_PATH",
		"SSH_AUTH_SOCK",
		"GNOME_KEYRING_CONTROL",
	];

	let vars_str = variables.join(" ");

	let systemd_cmd = format!("systemctl --user import-environment {vars_str}");
	if let Err(e) = Command::new("/bin/sh").arg("-c").arg(&systemd_cmd).status() {
		warn!("Failed to import environment into systemd: {}", e);
	}

	if let Ok(path) = which::which("dbus-update-activation-environment") {
		if let Err(e) = Command::new(&path).args(&variables).status() {
			warn!("Failed to update dbus activation environment: {}", e);
		}
	}

	info!("Activation environment imported");
}

fn setup_portal_config() {
	let config_dir = match dirs::config_dir() {
		Some(d) => d.join("xdg-desktop-portal"),
		None => return,
	};

	if let Err(e) = std::fs::create_dir_all(&config_dir) {
		warn!("Failed to create portal config directory: {}", e);
		return;
	}

	let portal_conf = config_dir.join("vitrum-portals.conf");
	let content = include_str!("../resources/vitrum-portals.conf");

	if let Err(e) = std::fs::write(&portal_conf, content) {
		warn!("Failed to write portal config: {}", e);
	} else {
		info!(path = %portal_conf.display(), "Portal configuration written");
	}
}
