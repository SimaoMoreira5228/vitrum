use vitrum_config::FontsConfig;

pub fn generate_fontconfig(fonts: &FontsConfig) -> String {
	let antialias = if fonts.rendering == "none" { "false" } else { "true" };
	let hinting = if fonts.hinting == "none" { "false" } else { "true" };
	let hintstyle = match fonts.hinting.as_str() {
		"slight" => "hintslight",
		"medium" => "hintmedium",
		"full" => "hintfull",
		_ => "hintslight",
	};
	let rgba = match fonts.rendering.as_str() {
		"subpixel" => "rgb",
		"grayscale" => "none",
		_ => "none",
	};

	let mut xml = String::from(
		"<?xml version=\"1.0\"?>\n<!DOCTYPE fontconfig SYSTEM \"fonts.dtd\">\n<fontconfig>\n  <!-- Vitrum font configuration — auto-generated, do not edit -->\n\n",
	);

	xml.push_str(&format!(
		"  <alias>\n    <family>sans-serif</family>\n    <prefer>\n      <family>{}</family>\n    </prefer>\n  </alias>\n",
		fonts.ui
	));

	xml.push_str(&format!(
		"  <alias>\n    <family>serif</family>\n    <prefer>\n      <family>{}</family>\n    </prefer>\n  </alias>\n",
		fonts.document
	));

	xml.push_str(&format!(
		"  <alias>\n    <family>monospace</family>\n    <prefer>\n      <family>{}</family>\n    </prefer>\n  </alias>\n",
		fonts.mono
	));

	xml.push_str(&format!(
		"\n  <match target=\"font\">\n    <edit name=\"antialias\" mode=\"assign\">\n      <bool>{}</bool>\n    </edit>\n    <edit name=\"hinting\" mode=\"assign\">\n      <bool>{}</bool>\n    </edit>\n    <edit name=\"hintstyle\" mode=\"assign\">\n      <const>{}</const>\n    </edit>\n    <edit name=\"rgba\" mode=\"assign\">\n      <const>{}</const>\n    </edit>\n    <edit name=\"lcdfilter\" mode=\"assign\">\n      <const>lcddefault</const>\n    </edit>\n  </match>\n",
		antialias, hinting, hintstyle, rgba
	));

	for dir in &fonts.extra_dirs {
		xml.push_str(&format!("  <dir>{}</dir>\n", dir));
	}

	xml.push_str("</fontconfig>\n");
	xml
}
