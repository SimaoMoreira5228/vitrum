use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use smithay::backend::allocator::Fourcc;
use smithay::backend::renderer::gles::{GlesRenderer, GlesTexture};
use smithay::backend::renderer::{ImportMem, Texture};
use tracing::{debug, error, info, warn};
use vitrum_config::WallpaperConfig;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FitMode {
	Fill,

	Fit,

	Center,

	Tile,
}

impl FitMode {
	fn from_str(s: &str) -> Self {
		match s.to_lowercase().as_str() {
			"fill" => FitMode::Fill,
			"fit" => FitMode::Fit,
			"center" => FitMode::Center,
			"tile" => FitMode::Tile,
			_ => {
				warn!(mode = %s, "Unknown fit mode, defaulting to fill");
				FitMode::Fill
			}
		}
	}
}

#[derive(Debug, Clone)]
pub enum WallpaperMode {
	Solid {
		color: [f32; 4],
	},

	Image {
		path: PathBuf,
		fit: FitMode,
	},

	Slideshow {
		dir: PathBuf,
		fit: FitMode,
		interval: Duration,
	},
}

pub struct TextureCache {
	textures: HashMap<PathBuf, GlesTexture>,
}

impl TextureCache {
	pub fn new() -> Self {
		Self {
			textures: HashMap::new(),
		}
	}

	pub fn get(&self, path: &Path) -> Option<&GlesTexture> {
		self.textures.get(path)
	}

	pub fn insert(&mut self, path: PathBuf, texture: GlesTexture) {
		self.textures.insert(path, texture);
	}

	pub fn clear(&mut self) {
		self.textures.clear();
	}

	pub fn remove(&mut self, path: &Path) -> Option<GlesTexture> {
		self.textures.remove(path)
	}
}

pub struct WallpaperState {
	mode: WallpaperMode,

	background_color: [f32; 4],

	current_image: Option<PathBuf>,

	textures: TextureCache,

	slideshow_images: Vec<PathBuf>,
	slideshow_index: usize,
	last_slideshow_change: Instant,

	output_size: (i32, i32),
}

impl WallpaperState {
	pub fn new(config: &WallpaperConfig) -> Self {
		let mode = Self::parse_mode(config);
		let background_color = parse_color(&config.color);

		Self {
			mode,
			background_color,
			current_image: None,
			textures: TextureCache::new(),
			slideshow_images: Vec::new(),
			slideshow_index: 0,
			last_slideshow_change: Instant::now(),
			output_size: (1920, 1080),
		}
	}

	fn parse_mode(config: &WallpaperConfig) -> WallpaperMode {
		match config.mode.as_str() {
			"solid" => WallpaperMode::Solid {
				color: parse_color(&config.color),
			},
			"image" => {
				if let Some(path) = &config.path {
					WallpaperMode::Image {
						path: PathBuf::from(path),
						fit: FitMode::from_str(&config.fit),
					}
				} else {
					warn!("Image mode requested but no path provided, using solid color");
					WallpaperMode::Solid {
						color: parse_color(&config.color),
					}
				}
			}
			"slideshow" => {
				if let Some(dir) = &config.dir {
					let interval = config
						.interval
						.map(|s| Duration::from_secs(s as u64))
						.unwrap_or(Duration::from_secs(300));
					WallpaperMode::Slideshow {
						dir: PathBuf::from(dir),
						fit: FitMode::from_str(&config.fit),
						interval,
					}
				} else {
					warn!("Slideshow mode requested but no directory provided, using solid color");
					WallpaperMode::Solid {
						color: parse_color(&config.color),
					}
				}
			}
			_ => {
				warn!(mode = %config.mode, "Unknown wallpaper mode, using solid color");
				WallpaperMode::Solid {
					color: parse_color(&config.color),
				}
			}
		}
	}

	pub fn initialize(&mut self) -> Result<()> {
		match &self.mode {
			WallpaperMode::Solid { .. } => {
				info!("Wallpaper initialized in solid color mode");
			}
			WallpaperMode::Image { path, .. } => {
				info!(path = %path.display(), "Wallpaper initialized in image mode");
				if !path.exists() {
					error!(path = %path.display(), "Wallpaper image does not exist");
				}
			}
			WallpaperMode::Slideshow { dir, interval, .. } => {
				info!(dir = %dir.display(), interval = ?interval, "Wallpaper initialized in slideshow mode");
				self.scan_slideshow_directory()?;
			}
		}
		Ok(())
	}

	fn scan_slideshow_directory(&mut self) -> Result<()> {
		if let WallpaperMode::Slideshow { dir, .. } = &self.mode {
			if !dir.exists() {
				return Err(anyhow::anyhow!("Slideshow directory does not exist: {}", dir.display()));
			}

			let mut images = Vec::new();
			for entry in std::fs::read_dir(dir)? {
				let entry = entry?;
				let path = entry.path();
				if is_image_file(&path) {
					images.push(path);
				}
			}

			images.sort();
			info!(count = images.len(), dir = %dir.display(), "Scanned slideshow directory");
			self.slideshow_images = images;
			self.slideshow_index = 0;
		}
		Ok(())
	}

	pub fn set_output_size(&mut self, width: i32, height: i32) {
		self.output_size = (width, height);
	}

	pub fn update_config(&mut self, config: &WallpaperConfig) {
		let new_mode = Self::parse_mode(config);

		if !Self::mode_compatible(&self.mode, &new_mode) {
			self.textures.clear();
			self.current_image = None;
			self.slideshow_images.clear();
		}

		self.mode = new_mode;
		self.background_color = parse_color(&config.color);

		if let Err(e) = self.initialize() {
			error!(error = %e, "Failed to reinitialize wallpaper");
		}
	}

	fn mode_compatible(a: &WallpaperMode, b: &WallpaperMode) -> bool {
		matches!(
			(a, b),
			(WallpaperMode::Solid { .. }, WallpaperMode::Solid { .. })
				| (WallpaperMode::Image { .. }, WallpaperMode::Image { .. })
				| (WallpaperMode::Slideshow { .. }, WallpaperMode::Slideshow { .. })
		)
	}

	pub fn update_slideshow(&mut self) -> bool {
		if let WallpaperMode::Slideshow { interval, .. } = self.mode {
			if self.last_slideshow_change.elapsed() >= interval {
				self.last_slideshow_change = Instant::now();
				self.advance_slideshow();
				return true;
			}
		}
		false
	}

	fn advance_slideshow(&mut self) {
		if self.slideshow_images.is_empty() {
			return;
		}
		self.slideshow_index = (self.slideshow_index + 1) % self.slideshow_images.len();
		let new_image = &self.slideshow_images[self.slideshow_index];
		debug!(index = self.slideshow_index, path = %new_image.display(), "Advanced slideshow");
	}

	pub fn current_image_path(&self) -> Option<&Path> {
		match &self.mode {
			WallpaperMode::Image { path, .. } => Some(path),
			WallpaperMode::Slideshow { .. } => self.slideshow_images.get(self.slideshow_index).map(|p| p.as_path()),
			WallpaperMode::Solid { .. } => None,
		}
	}

	pub fn current_fit_mode(&self) -> Option<FitMode> {
		match &self.mode {
			WallpaperMode::Image { fit, .. } => Some(*fit),
			WallpaperMode::Slideshow { fit, .. } => Some(*fit),
			WallpaperMode::Solid { .. } => None,
		}
	}

	pub fn background_color(&self) -> [f32; 4] {
		self.background_color
	}

	pub fn is_solid(&self) -> bool {
		matches!(self.mode, WallpaperMode::Solid { .. })
	}

	pub fn current_texture(&self) -> Option<&GlesTexture> {
		self.current_image_path().and_then(|path| self.textures.get(path))
	}

	pub fn ensure_texture_loaded(&mut self, renderer: &mut GlesRenderer) -> Result<()> {
		let path = match self.current_image_path() {
			Some(p) => p.to_path_buf(),
			None => return Ok(()),
		};

		if self.textures.get(&path).is_some() {
			return Ok(());
		}

		debug!(path = %path.display(), "Loading wallpaper texture");
		let texture = load_image_texture(renderer, &path)?;
		self.textures.insert(path, texture);
		Ok(())
	}

	pub fn calculate_rects(&self) -> Option<TextureRect> {
		let texture = self.current_texture()?;
		let fit = self.current_fit_mode()?;

		let tex_size = texture.size();
		let tex_w = tex_size.w as f32;
		let tex_h = tex_size.h as f32;
		let out_w = self.output_size.0 as f32;
		let out_h = self.output_size.1 as f32;

		let (src_rect, dst_rect) = match fit {
			FitMode::Fill => {
				let scale_x = out_w / tex_w;
				let scale_y = out_h / tex_h;
				let scale = scale_x.max(scale_y);

				let src_x = (tex_w - out_w / scale) / 2.0;
				let src_y = (tex_h - out_h / scale) / 2.0;
				let src_w = out_w / scale;
				let src_h = out_h / scale;

				(
					[src_x.max(0.0), src_y.max(0.0), src_w.min(tex_w), src_h.min(tex_h)],
					[0.0, 0.0, out_w, out_h],
				)
			}
			FitMode::Fit => {
				let scale_x = out_w / tex_w;
				let scale_y = out_h / tex_h;
				let scale = scale_x.min(scale_y);

				let scaled_w = tex_w * scale;
				let scaled_h = tex_h * scale;

				let dst_x = (out_w - scaled_w) / 2.0;
				let dst_y = (out_h - scaled_h) / 2.0;

				([0.0, 0.0, tex_w, tex_h], [dst_x, dst_y, scaled_w, scaled_h])
			}
			FitMode::Center => {
				let dst_x = (out_w - tex_w) / 2.0;
				let dst_y = (out_h - tex_h) / 2.0;

				([0.0, 0.0, tex_w, tex_h], [dst_x, dst_y, tex_w, tex_h])
			}
			FitMode::Tile => ([0.0, 0.0, out_w, out_h], [0.0, 0.0, out_w, out_h]),
		};

		Some(TextureRect {
			src: src_rect,
			dst: dst_rect,
			tile: fit == FitMode::Tile,
		})
	}
}

pub struct TextureRect {
	pub src: [f32; 4],
	pub dst: [f32; 4],
	pub tile: bool,
}

fn parse_color(hex: &str) -> [f32; 4] {
	let hex = hex.trim_start_matches('#');
	let len = hex.len();

	match len {
		6 => {
			if let (Ok(r), Ok(g), Ok(b)) = (
				u8::from_str_radix(&hex[0..2], 16),
				u8::from_str_radix(&hex[2..4], 16),
				u8::from_str_radix(&hex[4..6], 16),
			) {
				[r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0, 1.0]
			} else {
				warn!(color = %hex, "Failed to parse hex color, using default");
				[0.11, 0.12, 0.18, 1.0]
			}
		}
		8 => {
			if let (Ok(r), Ok(g), Ok(b), Ok(a)) = (
				u8::from_str_radix(&hex[0..2], 16),
				u8::from_str_radix(&hex[2..4], 16),
				u8::from_str_radix(&hex[4..6], 16),
				u8::from_str_radix(&hex[6..8], 16),
			) {
				[r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0, a as f32 / 255.0]
			} else {
				warn!(color = %hex, "Failed to parse hex color with alpha, using default");
				[0.11, 0.12, 0.18, 1.0]
			}
		}
		_ => {
			warn!(color = %hex, len, "Invalid hex color length, using default");
			[0.11, 0.12, 0.18, 1.0]
		}
	}
}

fn is_image_file(path: &Path) -> bool {
	let ext = path.extension().and_then(|e| e.to_str()).map(|e| e.to_lowercase());

	matches!(
		ext.as_deref(),
		Some("png") | Some("jpg") | Some("jpeg") | Some("webp") | Some("gif") | Some("bmp")
	)
}

fn load_image_texture(renderer: &mut GlesRenderer, path: &Path) -> Result<GlesTexture> {
	let img = image::open(path).with_context(|| format!("Failed to open image: {}", path.display()))?;

	let rgba = img.to_rgba8();
	let (width, height) = rgba.dimensions();

	debug!(
		path = %path.display(),
		width = width,
		height = height,
		"Loaded image"
	);

	let size = smithay::utils::Size::from((width as i32, height as i32));
	let texture = renderer
		.import_memory(&rgba, Fourcc::Abgr8888, size, false)
		.map_err(|e| anyhow::anyhow!("Failed to import texture: {}", e))?;

	Ok(texture)
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_parse_color_rgb() {
		let c = parse_color("#1C1C2E");
		assert!((c[0] - 0.11).abs() < 0.01);
		assert!((c[1] - 0.11).abs() < 0.01);
		assert!((c[2] - 0.18).abs() < 0.01);
		assert_eq!(c[3], 1.0);
	}

	#[test]
	fn test_parse_color_rgba() {
		let c = parse_color("#FF0000FF");
		assert_eq!(c[0], 1.0);
		assert_eq!(c[1], 0.0);
		assert_eq!(c[2], 0.0);
		assert_eq!(c[3], 1.0);
	}

	#[test]
	fn test_fit_mode_parsing() {
		assert_eq!(FitMode::from_str("fill"), FitMode::Fill);
		assert_eq!(FitMode::from_str("FIT"), FitMode::Fit);
		assert_eq!(FitMode::from_str("Center"), FitMode::Center);
		assert_eq!(FitMode::from_str("TILE"), FitMode::Tile);
		assert_eq!(FitMode::from_str("unknown"), FitMode::Fill);
	}

	#[test]
	fn test_is_image_file() {
		assert!(is_image_file(Path::new("test.png")));
		assert!(is_image_file(Path::new("test.jpg")));
		assert!(is_image_file(Path::new("test.JPEG")));
		assert!(!is_image_file(Path::new("test.txt")));
		assert!(!is_image_file(Path::new("test")));
	}
}
