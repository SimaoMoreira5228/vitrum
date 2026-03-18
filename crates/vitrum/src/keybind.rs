use smithay::backend::input::KeyState;
use smithay::input::keyboard::{Keysym, keysyms};
use tracing::{debug, info, warn};
use vitrum_config::{Action, Keybind};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KeyModifiers {
	pub ctrl: bool,
	pub shift: bool,
	pub alt: bool,
	pub super_: bool,
}

impl KeyModifiers {
	pub fn from_state(state: smithay::input::keyboard::ModifiersState) -> Self {
		Self {
			ctrl: state.ctrl,
			shift: state.shift,
			alt: state.alt,
			super_: state.logo,
		}
	}

	pub fn matches(&self, mods: &[String]) -> bool {
		let mut expect_ctrl = false;
		let mut expect_shift = false;
		let mut expect_alt = false;
		let mut expect_super = false;

		for m in mods {
			match m.to_lowercase().as_str() {
				"ctrl" | "control" => expect_ctrl = true,
				"shift" => expect_shift = true,
				"alt" => expect_alt = true,
				"super" | "logo" | "mod" | "mod4" => expect_super = true,
				_ => {
					warn!(modifier = %m, "Unknown modifier");
				}
			}
		}

		self.ctrl == expect_ctrl && self.shift == expect_shift && self.alt == expect_alt && self.super_ == expect_super
	}
}

impl Default for KeyModifiers {
	fn default() -> Self {
		Self {
			ctrl: false,
			shift: false,
			alt: false,
			super_: false,
		}
	}
}

pub fn parse_key(key: &str) -> Option<Keysym> {
	let keysym = match key.to_lowercase().as_str() {
		"a" => keysyms::KEY_a,
		"b" => keysyms::KEY_b,
		"c" => keysyms::KEY_c,
		"d" => keysyms::KEY_d,
		"e" => keysyms::KEY_e,
		"f" => keysyms::KEY_f,
		"g" => keysyms::KEY_g,
		"h" => keysyms::KEY_h,
		"i" => keysyms::KEY_i,
		"j" => keysyms::KEY_j,
		"k" => keysyms::KEY_k,
		"l" => keysyms::KEY_l,
		"m" => keysyms::KEY_m,
		"n" => keysyms::KEY_n,
		"o" => keysyms::KEY_o,
		"p" => keysyms::KEY_p,
		"q" => keysyms::KEY_q,
		"r" => keysyms::KEY_r,
		"s" => keysyms::KEY_s,
		"t" => keysyms::KEY_t,
		"u" => keysyms::KEY_u,
		"v" => keysyms::KEY_v,
		"w" => keysyms::KEY_w,
		"x" => keysyms::KEY_x,
		"y" => keysyms::KEY_y,
		"z" => keysyms::KEY_z,

		"0" | " KP_0" => keysyms::KEY_0,
		"1" | " KP_1" => keysyms::KEY_1,
		"2" | " KP_2" => keysyms::KEY_2,
		"3" | " KP_3" => keysyms::KEY_3,
		"4" | " KP_4" => keysyms::KEY_4,
		"5" | " KP_5" => keysyms::KEY_5,
		"6" | " KP_6" => keysyms::KEY_6,
		"7" | " KP_7" => keysyms::KEY_7,
		"8" | " KP_8" => keysyms::KEY_8,
		"9" | " KP_9" => keysyms::KEY_9,

		"f1" => keysyms::KEY_F1,
		"f2" => keysyms::KEY_F2,
		"f3" => keysyms::KEY_F3,
		"f4" => keysyms::KEY_F4,
		"f5" => keysyms::KEY_F5,
		"f6" => keysyms::KEY_F6,
		"f7" => keysyms::KEY_F7,
		"f8" => keysyms::KEY_F8,
		"f9" => keysyms::KEY_F9,
		"f10" => keysyms::KEY_F10,
		"f11" => keysyms::KEY_F11,
		"f12" => keysyms::KEY_F12,

		"return" | "enter" => keysyms::KEY_Return,
		"escape" | "esc" => keysyms::KEY_Escape,
		"backspace" => keysyms::KEY_BackSpace,
		"tab" => keysyms::KEY_Tab,
		"space" => keysyms::KEY_space,
		"delete" | "del" => keysyms::KEY_Delete,
		"insert" | "ins" => keysyms::KEY_Insert,
		"home" => keysyms::KEY_Home,
		"end" => keysyms::KEY_End,
		"pageup" | "page_up" => keysyms::KEY_Page_Up,
		"pagedown" | "page_down" => keysyms::KEY_Page_Down,

		"left" => keysyms::KEY_Left,
		"right" => keysyms::KEY_Right,
		"up" => keysyms::KEY_Up,
		"down" => keysyms::KEY_Down,

		"kp_enter" | "kp_return" => keysyms::KEY_KP_Enter,
		"kp_add" => keysyms::KEY_KP_Add,
		"kp_subtract" => keysyms::KEY_KP_Subtract,
		"kp_multiply" => keysyms::KEY_KP_Multiply,
		"kp_divide" => keysyms::KEY_KP_Divide,
		"kp_decimal" => keysyms::KEY_KP_Decimal,
		"kp_equal" => keysyms::KEY_KP_Equal,

		"volumemute" | "volume_mute" => keysyms::KEY_XF86AudioMute,
		"volumedown" | "volume_down" => keysyms::KEY_XF86AudioLowerVolume,
		"volumeup" | "volume_up" => keysyms::KEY_XF86AudioRaiseVolume,
		"play" | "playpause" => keysyms::KEY_XF86AudioPlay,
		"stop" => keysyms::KEY_XF86AudioStop,
		"prev" | "previous" => keysyms::KEY_XF86AudioPrev,
		"next" => keysyms::KEY_XF86AudioNext,

		_ => {
			if key.len() == 1 {
				let c = key.chars().next().expect("key length is 1");
				if c.is_ascii_lowercase() {
					keysyms::KEY_a + (c as u32 - 'a' as u32)
				} else if c.is_ascii_uppercase() {
					keysyms::KEY_A + (c as u32 - 'A' as u32)
				} else if c.is_ascii_digit() {
					keysyms::KEY_0 + (c as u32 - '0' as u32)
				} else {
					warn!(key = %key, "Unknown key name");
					return None;
				}
			} else {
				warn!(key = %key, "Unknown key name");
				return None;
			}
		}
	};

	Some(Keysym::from(keysym))
}

pub struct KeybindManager {
	keybinds: Vec<Keybind>,
}

impl KeybindManager {
	pub fn new(keybinds: Vec<Keybind>) -> Self {
		info!(count = keybinds.len(), "Keybind manager initialized");
		Self { keybinds }
	}

	pub fn update_keybinds(&mut self, keybinds: Vec<Keybind>) {
		info!(count = keybinds.len(), "Keybinds updated");
		self.keybinds = keybinds;
	}

	pub fn match_keybind(&self, keysym: Keysym, modifiers: KeyModifiers, key_state: KeyState) -> Option<Action> {
		if key_state != KeyState::Pressed {
			return None;
		}

		for bind in &self.keybinds {
			let bind_keysym = match parse_key(&bind.key) {
				Some(k) => k,
				None => continue,
			};

			if bind_keysym == keysym && modifiers.matches(&bind.mods) {
				debug!(
					key = %bind.key,
					mods = ?bind.mods,
					action = ?bind.action,
					"Keybind matched"
				);
				return Some(bind.action.clone());
			}
		}

		None
	}
}

impl Default for KeybindManager {
	fn default() -> Self {
		Self::new(Vec::new())
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_parse_key_letters() {
		assert_eq!(parse_key("a"), Some(Keysym::from(keysyms::KEY_a)));
		assert_eq!(parse_key("z"), Some(Keysym::from(keysyms::KEY_z)));
		assert_eq!(parse_key("A"), Some(Keysym::from(keysyms::KEY_a)));
	}

	#[test]
	fn test_parse_key_numbers() {
		assert_eq!(parse_key("1"), Some(Keysym::from(keysyms::KEY_1)));
		assert_eq!(parse_key("0"), Some(Keysym::from(keysyms::KEY_0)));
	}

	#[test]
	fn test_parse_key_special() {
		assert_eq!(parse_key("Return"), Some(Keysym::from(keysyms::KEY_Return)));
		assert_eq!(parse_key("space"), Some(Keysym::from(keysyms::KEY_space)));
		assert_eq!(parse_key("Escape"), Some(Keysym::from(keysyms::KEY_Escape)));
	}

	#[test]
	fn test_key_modifiers_matches() {
		let mods = KeyModifiers {
			ctrl: false,
			shift: false,
			alt: false,
			super_: true,
		};
		assert!(mods.matches(&vec!["super".to_string()]));
		assert!(!mods.matches(&vec!["ctrl".to_string()]));
		assert!(!mods.matches(&vec!["super".to_string(), "shift".to_string()]));

		let mods_with_shift = KeyModifiers {
			ctrl: false,
			shift: true,
			alt: false,
			super_: true,
		};
		assert!(mods_with_shift.matches(&vec!["super".to_string(), "shift".to_string()]));
	}
}
