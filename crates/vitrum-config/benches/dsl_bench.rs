use criterion::{Criterion, black_box, criterion_group, criterion_main};

use serde::Deserialize;
use vitrum_config::Config;
use vitrum_config::script;

const BASIC_VT: &str = r##"
[theme]
accent = "#FF0000"
background = "#000000"
dpi = 96
"##;

const ADVANCED_VT: &str = r##"
# Advanced Vitrum Config Example
let base_dpi = 96
let is_hidpi = false
let accent_color = "#1A6B8A"
let gaps_val = 8

[theme]
accent = accent_color
dpi = if is_hidpi then base_dpi * 2 else base_dpi
border_width = 2
corner_radius = if is_hidpi then 8 else 0

[layout]
gaps_inner = gaps_val
gaps_outer = gaps_val * 1.5

[session]
default_terminal = "foot"
"##;

const KEYBIND_LOOP_VT: &str = r##"
for n in 1..=9 {
    keybind(["super"], str(n), workspace(n))
    keybind(["super", "shift"], str(n), move_to_workspace(n))
}

keybind(["super"], "return", spawn("foot"))
keybind(["super", "shift"], "q", kill_focused())

[theme]
accent = "#FF0000"
"##;

fn bench_basic(c: &mut Criterion) {
	c.bench_function("eval_basic", |b| {
		b.iter(|| {
			let acc = script::eval(black_box(BASIC_VT)).unwrap();
			let _config = Config::deserialize(script::AccumulatorDeserializer::new(&acc)).unwrap();
		})
	});
}

fn bench_advanced(c: &mut Criterion) {
	c.bench_function("eval_advanced", |b| {
		b.iter(|| {
			let acc = script::eval(black_box(ADVANCED_VT)).unwrap();
			let _config = Config::deserialize(script::AccumulatorDeserializer::new(&acc)).unwrap();
		})
	});
}

fn bench_keybind_loop(c: &mut Criterion) {
	c.bench_function("eval_keybind_loop", |b| {
		b.iter(|| {
			let acc = script::eval(black_box(KEYBIND_LOOP_VT)).unwrap();
			let _config = Config::deserialize(script::AccumulatorDeserializer::new(&acc)).unwrap();
		})
	});
}

criterion_group!(benches, bench_basic, bench_advanced, bench_keybind_loop);
criterion_main!(benches);
