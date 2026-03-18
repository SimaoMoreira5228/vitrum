mod fontconfig;
mod gtk;
mod kvantum;
mod qt;
mod xrdb;

pub use fontconfig::generate_fontconfig;
pub use gtk::{generate_gtk3_css, generate_gtk3_settings, generate_gtk4_css, generate_gtk4_settings};
pub use kvantum::generate_kvantum_config;
pub use qt::{generate_qt5ct_config, generate_qt6ct_config};
pub use xrdb::generate_xrdb_resources;

use std::path::PathBuf;

use anyhow::Result;
use tracing::{info, warn};
use vitrum_config::{Config, FontsConfig, LocaleConfig, ThemeConfig};

pub struct ThemeState {
	config_dir: PathBuf,
}

impl ThemeState {
	pub fn new() -> Result<Self> {
		let config_dir = dirs::config_dir()
			.ok_or_else(|| anyhow::anyhow!("Could not determine config directory"))?
			.join("vitrum")
			.join("theme");
		Ok(Self { config_dir })
	}

	pub fn apply(&self, config: &Config) -> Result<()> {
		info!("Applying theme propagation");

		std::fs::create_dir_all(&self.config_dir)?;

		self.apply_gtk(&config.theme, &config.fonts)?;
		self.apply_qt(&config.theme, &config.fonts)?;
		self.apply_fontconfig(&config.fonts)?;
		self.apply_xrdb(&config.theme, &config.fonts)?;
		self.apply_gsettings(&config.theme, &config.fonts)?;
		self.apply_locale(&config.locale)?;

		notify_portal_color_scheme(config.theme.color_scheme == "dark");

		info!("Theme propagation complete");
		Ok(())
	}

	fn apply_gtk(&self, theme: &ThemeConfig, fonts: &FontsConfig) -> Result<()> {
		let gtk3_dir = dirs::config_dir()
			.unwrap_or_else(|| PathBuf::from("~/.config"))
			.join("gtk-3.0");
		std::fs::create_dir_all(&gtk3_dir)?;

		let settings_ini = generate_gtk3_settings(theme, fonts);
		std::fs::write(gtk3_dir.join("settings.ini"), settings_ini)?;
		info!("Wrote GTK3 settings.ini");

		let css = generate_gtk3_css(theme);
		std::fs::write(gtk3_dir.join("gtk.css"), css)?;
		info!("Wrote GTK3 gtk.css");

		let gtk4_dir = dirs::config_dir()
			.unwrap_or_else(|| PathBuf::from("~/.config"))
			.join("gtk-4.0");
		std::fs::create_dir_all(&gtk4_dir)?;

		let settings_ini = generate_gtk4_settings(theme, fonts);
		std::fs::write(gtk4_dir.join("settings.ini"), settings_ini)?;
		info!("Wrote GTK4 settings.ini");

		let css = generate_gtk4_css(theme);
		std::fs::write(gtk4_dir.join("gtk.css"), css)?;
		info!("Wrote GTK4 gtk.css");

		Ok(())
	}

	fn apply_qt(&self, theme: &ThemeConfig, fonts: &FontsConfig) -> Result<()> {
		let qt6ct_dir = dirs::config_dir().unwrap_or_else(|| PathBuf::from("~/.config")).join("qt6ct");
		std::fs::create_dir_all(&qt6ct_dir)?;

		let conf = generate_qt6ct_config(theme, fonts);
		std::fs::write(qt6ct_dir.join("qt6ct.conf"), conf)?;
		info!("Wrote qt6ct config");

		let qt5ct_dir = dirs::config_dir().unwrap_or_else(|| PathBuf::from("~/.config")).join("qt5ct");
		std::fs::create_dir_all(&qt5ct_dir)?;

		let conf = generate_qt5ct_config(theme, fonts);
		std::fs::write(qt5ct_dir.join("qt5ct.conf"), conf)?;
		info!("Wrote qt5ct config");

		let kvantum_dir = dirs::config_dir()
			.unwrap_or_else(|| PathBuf::from("~/.config"))
			.join("Kvantum")
			.join("vitrum");
		std::fs::create_dir_all(&kvantum_dir)?;

		let config = generate_kvantum_config(theme);
		std::fs::write(kvantum_dir.join("vitrum.kvconfig"), config)?;
		info!("Wrote Kvantum config");

		Ok(())
	}

	fn apply_fontconfig(&self, fonts: &FontsConfig) -> Result<()> {
		let fc_dir = dirs::config_dir()
			.unwrap_or_else(|| PathBuf::from("~/.config"))
			.join("fontconfig");
		std::fs::create_dir_all(&fc_dir)?;

		let xml = generate_fontconfig(fonts);
		std::fs::write(fc_dir.join("fonts.conf"), xml)?;
		info!("Wrote fontconfig fonts.conf");

		Ok(())
	}

	fn apply_xrdb(&self, theme: &ThemeConfig, fonts: &FontsConfig) -> Result<()> {
		let resources = generate_xrdb_resources(theme, fonts);

		let xrdb_dir = self.config_dir.clone();
		std::fs::create_dir_all(&xrdb_dir)?;
		std::fs::write(xrdb_dir.join("xrdb.conf"), &resources)?;
		info!("Wrote xrdb resources");

		Ok(())
	}

	fn apply_gsettings(&self, theme: &ThemeConfig, fonts: &FontsConfig) -> Result<()> {
		let commands: Vec<Vec<&str>> = vec![
			vec![
				"gsettings",
				"set",
				"org.gnome.desktop.interface",
				"color-scheme",
				if theme.color_scheme == "dark" {
					"prefer-dark"
				} else {
					"prefer-light"
				},
			],
			vec![
				"gsettings",
				"set",
				"org.gnome.desktop.interface",
				"cursor-theme",
				&theme.cursor_theme,
			],
			vec![
				"gsettings",
				"set",
				"org.gnome.desktop.interface",
				"icon-theme",
				&theme.icon_theme,
			],
			vec![
				"gsettings",
				"set",
				"org.gnome.desktop.interface",
				"gtk-theme",
				if theme.color_scheme == "dark" {
					"Adwaita-dark"
				} else {
					"Adwaita"
				},
			],
			vec!["gsettings", "set", "org.gnome.desktop.interface", "font-name", &fonts.ui],
			vec![
				"gsettings",
				"set",
				"org.gnome.desktop.interface",
				"document-font-name",
				&fonts.ui,
			],
			vec![
				"gsettings",
				"set",
				"org.gnome.desktop.interface",
				"monospace-font-name",
				&fonts.mono,
			],
		];

		for args in &commands {
			match std::process::Command::new(args[0]).args(&args[1..]).status() {
				Ok(status) if status.success() => {}
				Ok(_) => warn!("gsettings command failed: {:?}", args),
				Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
					info!("gsettings not found, skipping");
					break;
				}
				Err(e) => warn!("gsettings error: {}", e),
			}
		}

		Ok(())
	}

	fn apply_locale(&self, locale: &LocaleConfig) -> Result<()> {
		let locale_dir = self.config_dir.clone();
		std::fs::create_dir_all(&locale_dir)?;

		let mut content = String::new();
		content.push_str(&format!("LANG={}\n", locale.lang));
		if let Some(ref v) = locale.lc_time {
			content.push_str(&format!("LC_TIME={}\n", v));
		}
		if let Some(ref v) = locale.lc_numeric {
			content.push_str(&format!("LC_NUMERIC={}\n", v));
		}
		if let Some(ref v) = locale.lc_monetary {
			content.push_str(&format!("LC_MONETARY={}\n", v));
		}
		if let Some(ref v) = locale.lc_paper {
			content.push_str(&format!("LC_PAPER={}\n", v));
		}
		if let Some(ref v) = locale.lc_measurement {
			content.push_str(&format!("LC_MEASUREMENT={}\n", v));
		}
		if let Some(ref v) = locale.lc_collate {
			content.push_str(&format!("LC_COLLATE={}\n", v));
		}

		std::fs::write(locale_dir.join("locale.conf"), &content)?;
		info!("Wrote locale.conf");

		Ok(())
	}

	pub fn build_env(config: &Config) -> std::collections::HashMap<String, String> {
		let mut env = std::collections::HashMap::new();

		let theme = &config.theme;
		let fonts = &config.fonts;
		let locale = &config.locale;

		env.insert("XCURSOR_THEME".into(), theme.cursor_theme.clone());
		env.insert("XCURSOR_SIZE".into(), theme.cursor_size.to_string());
		env.insert("GTK_CURSOR_THEME".into(), theme.cursor_theme.clone());

		env.insert("GTK_THEME".into(), "vitrum-generated".into());
		env.insert("GTK_ICON_THEME".into(), theme.icon_theme.clone());
		env.insert("GDK_BACKEND".into(), "wayland,x11".into());
		env.insert("GDK_SCALE".into(), theme.gdk_scale.to_string());
		env.insert("GDK_DPI_SCALE".into(), theme.gdk_dpi_scale.to_string());

		env.insert("QT_QPA_PLATFORM".into(), "wayland;xcb".into());
		env.insert("QT_QPA_PLATFORMTHEME".into(), "qt6ct".into());
		env.insert("QT_STYLE_OVERRIDE".into(), "kvantum".into());
		env.insert("QT_WAYLAND_DISABLE_WINDOWDECORATION".into(), "1".into());
		env.insert("QT_SCALE_FACTOR".into(), theme.qt_scale_factor.to_string());
		env.insert("QT_AUTO_SCREEN_SCALE_FACTOR".into(), "0".into());
		env.insert("QT_ENABLE_HIGHDPI_SCALING".into(), "1".into());

		env.insert("MOZ_ENABLE_WAYLAND".into(), "1".into());
		env.insert("MOZ_WEBRENDER".into(), "1".into());

		env.insert("ELECTRON_OZONE_PLATFORM_HINT".into(), "auto".into());
		env.insert("NIXOS_OZONE_WL".into(), "1".into());

		env.insert("SDL_VIDEODRIVER".into(), "wayland,x11".into());

		env.insert("_JAVA_AWT_WM_NONREPARENTING".into(), "1".into());
		env.insert(
			"JDK_JAVA_OPTIONS".into(),
			"-Dawt.useSystemAAFontSettings=on -Dswing.aatext=true".into(),
		);

		env.insert("CLUTTER_BACKEND".into(), "wayland".into());

		let mut fontconfig_path = String::from("~/.config/fontconfig:/etc/fonts");
		if !fonts.extra_dirs.is_empty() {
			fontconfig_path.push(':');
			fontconfig_path.push_str(&fonts.extra_dirs.join(":"));
		}
		env.insert("FONTCONFIG_PATH".into(), fontconfig_path);

		env.insert("LANG".into(), locale.lang.clone());
		if let Some(ref v) = locale.lc_time {
			env.insert("LC_TIME".into(), v.clone());
		}
		if let Some(ref v) = locale.lc_numeric {
			env.insert("LC_NUMERIC".into(), v.clone());
		}
		if let Some(ref v) = locale.lc_monetary {
			env.insert("LC_MONETARY".into(), v.clone());
		}
		if let Some(ref v) = locale.lc_paper {
			env.insert("LC_PAPER".into(), v.clone());
		}
		if let Some(ref v) = locale.lc_measurement {
			env.insert("LC_MEASUREMENT".into(), v.clone());
		}
		if let Some(ref v) = locale.lc_collate {
			env.insert("LC_COLLATE".into(), v.clone());
		}

		env
	}

	pub fn xrdb_resources(config: &Config) -> String {
		generate_xrdb_resources(&config.theme, &config.fonts)
	}

	pub fn xrdb_conf_path() -> Result<PathBuf> {
		let config_dir = dirs::config_dir().ok_or_else(|| anyhow::anyhow!("Could not determine config directory"))?;
		Ok(config_dir.join("vitrum").join("theme").join("xrdb.conf"))
	}
}

pub fn notify_portal_color_scheme(is_dark: bool) {
	let value: u32 = if is_dark { 1 } else { 0 };
	std::thread::spawn(move || match send_portal_color_scheme(value) {
		Ok(()) => info!("Portal color-scheme notification sent"),
		Err(e) => warn!("Portal color-scheme notification failed: {}", e),
	});
}

fn send_portal_color_scheme(value: u32) -> Result<()> {
	let conn = zbus::blocking::Connection::session()?;

	conn.emit_signal(
		Some("org.freedesktop.portal.Desktop"),
		"/org/freedesktop/portal/desktop",
		"org.freedesktop.portal.Settings",
		"SettingChanged",
		&(
			"org.freedesktop.appearance",
			"color-scheme",
			zbus::zvariant::Value::U32(value),
		),
	)?;

	Ok(())
}

impl Default for ThemeState {
	fn default() -> Self {
		Self::new().unwrap_or_else(|_| Self {
			config_dir: PathBuf::from("/tmp/vitrum-theme"),
		})
	}
}
