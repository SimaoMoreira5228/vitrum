use vitrum_config::{FontsConfig, ThemeConfig};

pub fn generate_qt6ct_config(theme: &ThemeConfig, fonts: &FontsConfig) -> String {
	format!(
		r#"[Appearance]
			 color_scheme_path=
			 custom_palette=false
			 icon_theme={icon_theme}
			 standard_dialogs=default
			 style=kvantum-dark
 
			 [Fonts]
			 fixed="{mono}, {mono_size},-1,5,50,0,0,0,0,0"
			 general="{ui}, {ui_size},-1,5,50,0,0,0,0,0"
 
			 [Interface]
			 activate_item_on_single_click=1
			 buttonbox_layout=0
			 cursor_flash_time=1000
			 dialog_buttons_have_icons=1
			 double_click_interval=400
			 gui_effects=@Invalid()
			 keyboard_scheme=2
			 menus_have_icons=true
			 show_shortcuts_in_context_menus=true
			 stylesheets=@Invalid()
			 toolbutton_style=4
			 underline_shortcut=1
			 wheel_scroll_lines=3
		"#,
		icon_theme = theme.icon_theme,
		ui = fonts.ui,
		ui_size = fonts.ui_size,
		mono = fonts.mono,
		mono_size = fonts.mono_size,
	)
}

pub fn generate_qt5ct_config(theme: &ThemeConfig, fonts: &FontsConfig) -> String {
	format!(
		r#"[Appearance]
			 color_scheme_path=
			 custom_palette=false
			 icon_theme={icon_theme}
			 standard_dialogs=default
			 style=kvantum-dark
 
			 [Fonts]
			 fixed="{mono}, {mono_size},-1,5,50,0,0,0,0,0"
			 general="{ui}, {ui_size},-1,5,50,0,0,0,0,0"
 
			 [Interface]
			 activate_item_on_single_click=1
			 buttonbox_layout=0
			 cursor_flash_time=1000
			 dialog_buttons_have_icons=1
			 double_click_interval=400
			 gui_effects=@Invalid()
			 keyboard_scheme=2
			 menus_have_icons=true
			 show_shortcuts_in_context_menus=true
			 stylesheets=@Invalid()
			 toolbutton_style=4
			 underline_shortcut=1
			 wheel_scroll_lines=3
		"#,
		icon_theme = theme.icon_theme,
		ui = fonts.ui,
		ui_size = fonts.ui_size,
		mono = fonts.mono,
		mono_size = fonts.mono_size,
	)
}
