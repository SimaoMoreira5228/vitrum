use std::path::PathBuf;
use vitrum_config::Config;

#[test]
fn test_dsl_basic() {
	let source = r##"
        [theme]
        accent = "#1A6B8A"
        border_width = 2
    "##;

	let temp_dir = tempfile::tempdir().unwrap();
	let config_path = temp_dir.path().join("test.vt");
	std::fs::write(&config_path, source).unwrap();

	let config = Config::load_from(&config_path).unwrap();
	assert_eq!(config.theme.accent, "#1A6B8A");
	assert_eq!(config.theme.border_width, 2);
}

#[test]
fn test_dsl_expressions() {
	let source = r#"
        let base = 8
        let is_wide = true
        
        [layout]
        gaps_inner = base
        gaps_outer = if is_wide then base * 2 else base
    "#;

	let temp_dir = tempfile::tempdir().unwrap();
	let config_path = temp_dir.path().join("test.vt");
	std::fs::write(&config_path, source).unwrap();

	let config = Config::load_from(&config_path).unwrap();
	assert_eq!(config.layout.gaps_inner, 8);
	assert_eq!(config.layout.gaps_outer, 16);
}

#[test]
fn test_dsl_arithmetic() {
	let source = r#"
        [layout]
        gaps_inner = 4 + 4
        gaps_outer = 8 * 2 - 4
        master_ratio = 1.0 / 2.0
    "#;

	let temp_dir = tempfile::tempdir().unwrap();
	let config_path = temp_dir.path().join("test.vt");
	std::fs::write(&config_path, source).unwrap();

	let config = Config::load_from(&config_path).unwrap();
	assert_eq!(config.layout.gaps_inner, 8);
	assert_eq!(config.layout.gaps_outer, 12);
	assert_eq!(config.layout.master_ratio, 0.5);
}

#[test]
fn test_dsl_examples() {
	let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));

	let basic_path = manifest_dir.join("examples/basic.vt");
	let config_basic = Config::load_from(&basic_path).expect("Failed to load basic.vt");
	assert_eq!(config_basic.theme.accent, "#1A6B8A");

	let advanced_path = manifest_dir.join("examples/advanced.vt");
	let config_advanced = Config::load_from(&advanced_path).expect("Failed to load advanced.vt");
	assert_eq!(config_advanced.theme.accent, "#1A6B8A");
	assert_eq!(config_advanced.theme.dpi, 96);
}

#[test]
fn test_config_snapshot() {
	let source = r##"
        [session]
        default_terminal = "alacritty"
        
        [theme]
        accent = "#FF0000"
        border_width = 4
        
        [layout]
        gaps_inner = 10
    "##;

	let temp_dir = tempfile::tempdir().unwrap();
	let config_path = temp_dir.path().join("snapshot.vt");
	std::fs::write(&config_path, source).unwrap();

	let config = Config::load_from(&config_path).unwrap();

	insta::assert_yaml_snapshot!(config);
}
