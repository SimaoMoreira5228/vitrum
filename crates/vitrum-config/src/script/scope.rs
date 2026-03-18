use std::collections::HashMap;

use crate::script::eval::Value;

pub struct Scope<'a> {
	bindings: HashMap<&'a str, Value>,
	parent: Option<&'a Scope<'a>>,
}

impl<'a> Scope<'a> {
	pub fn new() -> Self {
		Self {
			bindings: HashMap::new(),
			parent: None,
		}
	}

	pub fn child(parent: &'a Scope<'a>) -> Self {
		Self {
			bindings: HashMap::new(),
			parent: Some(parent),
		}
	}

	pub fn define(&mut self, name: &'a str, val: Value) {
		self.bindings.insert(name, val);
	}

	pub fn get(&self, name: &str) -> Option<&Value> {
		self.bindings.get(name).or_else(|| self.parent.and_then(|p| p.get(name)))
	}
}
