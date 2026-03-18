use anyhow::Result;
use std::path::PathBuf;
use vitrum_config::Config;

fn main() -> Result<()> {
	let args: Vec<String> = std::env::args().collect();
	let path = if args.len() > 1 {
		PathBuf::from(&args[1])
	} else {
		PathBuf::from("crates/vitrum-config/examples/basic.vt")
	};

	println!("Loading config from: {:?}", path);

	match Config::load_from(&path) {
		Ok(config) => {
			println!("Successfully loaded config!");
			println!("{:#?}", config);
		}
		Err(e) => {
			eprintln!("Error loading config: {}", e);
			std::process::exit(1);
		}
	}

	Ok(())
}
