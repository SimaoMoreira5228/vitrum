use vitrum_config::{FontsConfig, ThemeConfig};

pub fn generate_gtk3_settings(theme: &ThemeConfig, fonts: &FontsConfig) -> String {
	let is_dark = theme.color_scheme == "dark";
	format!(
		r#"[Settings]
			 gtk-theme-name                    = vitrum-generated
			 gtk-icon-theme-name               = {icon_theme}
			 gtk-cursor-theme-name             = {cursor_theme}
			 gtk-cursor-theme-size             = {cursor_size}
			 gtk-font-name                     = {font_name}
			 gtk-sound-theme-name              = {sound_theme}
			 gtk-application-prefer-dark-theme = {dark}
			 gtk-decoration-layout             = :minimize,maximize,close
			 gtk-xft-antialias                 = {antialias}
			 gtk-xft-hinting                   = {hinting}
			 gtk-xft-hintstyle                 = {hintstyle}
			 gtk-xft-rgba                      = {rgba}
			 gtk-xft-dpi                       = {dpi}
		"#,
		icon_theme = theme.icon_theme,
		cursor_theme = theme.cursor_theme,
		cursor_size = theme.cursor_size,
		font_name = format!("{} {}", fonts.ui, fonts.ui_size),
		sound_theme = theme.sound_theme,
		dark = is_dark,
		antialias = if fonts.rendering == "none" { "0" } else { "1" },
		hinting = if fonts.hinting == "none" { "0" } else { "1" },
		hintstyle = match fonts.hinting.as_str() {
			"slight" => "hintslight",
			"medium" => "hintmedium",
			"full" => "hintfull",
			_ => "hintslight",
		},
		rgba = match fonts.rendering.as_str() {
			"subpixel" => "rgb",
			"grayscale" => "none",
			_ => "none",
		},
		dpi = theme.dpi,
	)
}

pub fn generate_gtk4_settings(theme: &ThemeConfig, fonts: &FontsConfig) -> String {
	let is_dark = theme.color_scheme == "dark";
	format!(
		r#"[Settings]
			 gtk-theme-name                    = vitrum-generated
			 gtk-icon-theme-name               = {icon_theme}
			 gtk-cursor-theme-name             = {cursor_theme}
			 gtk-cursor-theme-size             = {cursor_size}
			 gtk-font-name                     = {font_name}
			 gtk-application-prefer-dark-theme = {dark}
			 gtk-decoration-layout             = :minimize,maximize,close
			 gtk-xft-antialias                 = {antialias}
			 gtk-xft-hinting                   = {hinting}
			 gtk-xft-hintstyle                 = {hintstyle}
			 gtk-xft-rgba                      = {rgba}
			 gtk-xft-dpi                       = {dpi}
	"#,
		icon_theme = theme.icon_theme,
		cursor_theme = theme.cursor_theme,
		cursor_size = theme.cursor_size,
		font_name = format!("{} {}", fonts.ui, fonts.ui_size),
		dark = is_dark,
		antialias = if fonts.rendering == "none" { "0" } else { "1" },
		hinting = if fonts.hinting == "none" { "0" } else { "1" },
		hintstyle = match fonts.hinting.as_str() {
			"slight" => "hintslight",
			"medium" => "hintmedium",
			"full" => "hintfull",
			_ => "hintslight",
		},
		rgba = match fonts.rendering.as_str() {
			"subpixel" => "rgb",
			"grayscale" => "none",
			_ => "none",
		},
		dpi = theme.dpi,
	)
}

pub fn generate_gtk3_css(theme: &ThemeConfig) -> String {
	format!(
		r#"/* Vitrum theme — auto-generated, do not edit */
			@define-color accent_color {accent};
			@define-color bg_color {background};
			@define-color fg_color {text};
			@define-color selected_bg_color {accent};
			@define-color selected_fg_color {text};
			@define-color error_color {error};
			@define-color warning_color {warning};
			@define-color success_color {success};
			@define-color borders {border};
			@define-color view_bg_color {surface};

			/* Apply surface background to all windows */
			window {{
				background-color: @view_bg_color;
			}}
	"#,
		accent = theme.accent,
		background = theme.background,
		text = theme.text,
		error = theme.error,
		warning = theme.warning,
		success = theme.success,
		border = theme.border,
		surface = theme.surface,
	)
}

pub fn generate_gtk4_css(theme: &ThemeConfig) -> String {
	format!(
		r#"/* Vitrum theme — auto-generated, do not edit */
			@define-color accent_color {accent};
			@define-color window_bg_color {background};
			@define-color window_fg_color {text};
			@define-color view_bg_color {surface};
			@define-color view_fg_color {text};
			@define-color error_color {error};
			@define-color warning_color {warning};
			@define-color success_color {success};
			@define-color borders {border};
		"#,
		accent = theme.accent,
		background = theme.background,
		text = theme.text,
		surface = theme.surface,
		error = theme.error,
		warning = theme.warning,
		success = theme.success,
		border = theme.border,
	)
}
