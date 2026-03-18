use vitrum_config::{FontsConfig, ThemeConfig};

pub fn generate_xrdb_resources(theme: &ThemeConfig, fonts: &FontsConfig) -> String {
	format!(
		r#"Xft.antialias: {antialias}
			 Xft.hinting: {hinting}
			 Xft.hintstyle: {hintstyle}
			 Xft.rgba: {rgba}
			 Xft.dpi: {dpi}
			 Xft.font: {font}
			 Xcursor.theme: {cursor_theme}
			 Xcursor.size: {cursor_size}
		"#,
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
		font = format!("{}:size={}", fonts.ui, fonts.ui_size),
		cursor_theme = theme.cursor_theme,
		cursor_size = theme.cursor_size,
	)
}
