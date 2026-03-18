use vitrum_config::ThemeConfig;

pub fn generate_kvantum_config(theme: &ThemeConfig) -> String {
	format!(
		r#"[General]
			 author=Vitrum
			 comment=Auto-generated Vitrum theme
 
			 [%themestyle]
			 inherits=kvantum
 
			 [NormalColor]
			 base.color={background}
			 base.foreground={text}
			 button.color={surface}
			 button.foreground={text}
			 button.light={surface_raised}
			 button.midlight={border}
			 button.dark={border}
			 button.mid={border}
			 button.text={text}
			 button.bright_text={text}
			 button.button_text={text}
			 window.color={background}
			 window.foreground={text}
			 window.button={surface}
			 window.light={surface_raised}
			 window.midlight={border}
			 window.dark={border}
			 window.mid={border}
			 window.text={text}
			 window.bright_text={text}
			 window.button_text={text}
			 highlight.color={accent}
			 highlight.foreground={text}
			 highlightedtext.color={accent}
			 highlightedtext.foreground={text}
			 tooltip.base={surface}
			 tooltip.text={text}
			 text={text}
			 text.foreground={text}
			 link={accent}
			 linkvisited={accent}
			 alternate.base={surface_raised}
 
			 [DisabledColor]
			 text.foreground={text_muted}
 
			 [FocusColor]
			 interiorFocus.color={accent}
		"#,
		background = theme.background,
		text = theme.text,
		surface = theme.surface,
		surface_raised = theme.surface_raised,
		border = theme.border,
		accent = theme.accent,
		text_muted = theme.text_muted,
	)
}
