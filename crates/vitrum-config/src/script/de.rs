use std::collections::HashMap;

use serde::Deserializer;
use serde::de::{self, IntoDeserializer, MapAccess, SeqAccess, Visitor};
use serde::forward_to_deserialize_any;

use crate::script::eval::Value;

#[derive(Debug, Default)]
pub struct ConfigAccumulator {
	pub sections: HashMap<String, Value>,
	pub tagged: HashMap<String, Vec<Value>>,
}

impl ConfigAccumulator {
	pub fn new() -> Self {
		Self::default()
	}

	pub fn push_tagged(&mut self, map: HashMap<String, Value>) {
		let tag = match map.get("type") {
			Some(Value::String(t)) => t.clone(),
			_ => return,
		};
		self.tagged.entry(tag).or_default().push(Value::Map(map));
	}
}

#[derive(Debug)]
pub struct DeError(String);

impl DeError {
	fn type_mismatch(expected: &str, got: &'static str) -> Self {
		DeError(format!("expected {}, got {}", expected, got))
	}
}

impl std::fmt::Display for DeError {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}", self.0)
	}
}

impl std::error::Error for DeError {}

impl de::Error for DeError {
	fn custom<T: std::fmt::Display>(msg: T) -> Self {
		DeError(msg.to_string())
	}
}

pub struct AccumulatorDeserializer<'a> {
	acc: &'a ConfigAccumulator,
}

impl<'a> AccumulatorDeserializer<'a> {
	pub fn new(acc: &'a ConfigAccumulator) -> Self {
		Self { acc }
	}
}

impl<'de, 'a> Deserializer<'de> for AccumulatorDeserializer<'a> {
	type Error = DeError;

	fn deserialize_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
		self.deserialize_map(visitor)
	}

	fn deserialize_map<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
		visitor.visit_map(AccumulatorMapAccess::new(self.acc))
	}

	fn deserialize_struct<V: Visitor<'de>>(
		self,
		_name: &'static str,
		_fields: &'static [&'static str],
		visitor: V,
	) -> Result<V::Value, Self::Error> {
		self.deserialize_map(visitor)
	}

	forward_to_deserialize_any! {
		bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string
		bytes byte_buf option unit unit_struct newtype_struct seq tuple
		tuple_struct enum identifier ignored_any
	}
}

struct AccumulatorMapAccess {
	keys: Vec<String>,
	values: HashMap<String, Value>,
	pos: usize,
}

impl AccumulatorMapAccess {
	fn new(acc: &ConfigAccumulator) -> Self {
		let mut keys = Vec::new();
		let mut values = HashMap::new();

		for (k, v) in &acc.sections {
			keys.push(k.clone());
			values.insert(k.clone(), v.clone());
		}

		for (k, v) in &acc.tagged {
			if !values.contains_key(k) {
				keys.push(k.clone());
			}
			values.insert(k.clone(), Value::List(v.clone()));
		}

		Self { keys, values, pos: 0 }
	}
}

impl<'de> MapAccess<'de> for AccumulatorMapAccess {
	type Error = DeError;

	fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>, Self::Error>
	where
		K: de::DeserializeSeed<'de>,
	{
		if self.pos >= self.keys.len() {
			return Ok(None);
		}
		let key = self.keys[self.pos].clone();
		self.pos += 1;
		seed.deserialize(key.into_deserializer()).map(Some)
	}

	fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value, Self::Error>
	where
		V: de::DeserializeSeed<'de>,
	{
		let key = &self.keys[self.pos - 1];
		let val = self.values.get(key).cloned().unwrap_or(Value::Null);
		seed.deserialize(ValueDeserializer(val))
	}
}

pub struct ValueDeserializer(pub Value);

impl<'de> Deserializer<'de> for ValueDeserializer {
	type Error = DeError;

	fn deserialize_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
		match self.0 {
			Value::Null => visitor.visit_unit(),
			Value::Bool(b) => visitor.visit_bool(b),
			Value::Int(n) => visitor.visit_i64(n),
			Value::Float(f) => visitor.visit_f64(f),
			Value::String(s) => visitor.visit_string(s),
			Value::List(items) => visitor.visit_seq(ListSeqAccess { items, pos: 0 }),
			Value::Map(map) => visitor.visit_map(MapMapAccess::from(map)),
		}
	}

	fn deserialize_bool<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
		match self.0 {
			Value::Bool(b) => visitor.visit_bool(b),
			Value::Int(n) => visitor.visit_bool(n != 0),
			other => Err(DeError::type_mismatch("bool", other.type_name())),
		}
	}

	fn deserialize_i64<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
		match self.0 {
			Value::Int(n) => visitor.visit_i64(n),
			Value::Float(f) => visitor.visit_i64(f as i64),
			other => Err(DeError::type_mismatch("int", other.type_name())),
		}
	}

	fn deserialize_f64<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
		match self.0 {
			Value::Float(f) => visitor.visit_f64(f),
			Value::Int(n) => visitor.visit_f64(n as f64),
			other => Err(DeError::type_mismatch("float", other.type_name())),
		}
	}

	fn deserialize_string<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
		match self.0 {
			Value::String(s) => visitor.visit_string(s),
			Value::Int(n) => visitor.visit_string(n.to_string()),
			Value::Float(f) => visitor.visit_string(f.to_string()),
			Value::Bool(b) => visitor.visit_string(b.to_string()),
			other => Err(DeError::type_mismatch("string", other.type_name())),
		}
	}

	fn deserialize_str<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
		self.deserialize_string(visitor)
	}

	fn deserialize_option<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
		match self.0 {
			Value::Null => visitor.visit_none(),
			other => visitor.visit_some(ValueDeserializer(other)),
		}
	}

	fn deserialize_seq<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
		match self.0 {
			Value::List(items) => visitor.visit_seq(ListSeqAccess { items, pos: 0 }),
			Value::Null => visitor.visit_seq(ListSeqAccess { items: vec![], pos: 0 }),
			other => Err(DeError::type_mismatch("list", other.type_name())),
		}
	}

	fn deserialize_map<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
		match self.0 {
			Value::Map(map) => visitor.visit_map(MapMapAccess::from(map)),
			Value::Null => visitor.visit_map(MapMapAccess::from(HashMap::new())),
			other => Err(DeError::type_mismatch("map", other.type_name())),
		}
	}

	fn deserialize_struct<V: Visitor<'de>>(
		self,
		_name: &'static str,
		_fields: &'static [&'static str],
		visitor: V,
	) -> Result<V::Value, Self::Error> {
		self.deserialize_map(visitor)
	}

	fn deserialize_enum<V: Visitor<'de>>(
		self,
		_name: &'static str,
		_variants: &'static [&'static str],
		visitor: V,
	) -> Result<V::Value, Self::Error> {
		match self.0 {
			Value::String(s) => visitor.visit_enum(s.into_deserializer()),
			Value::Map(map) => visitor.visit_enum(MapEnumAccess { map }),
			other => Err(DeError::type_mismatch("enum", other.type_name())),
		}
	}

	fn deserialize_identifier<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
		self.deserialize_string(visitor)
	}

	fn deserialize_ignored_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
		visitor.visit_unit()
	}

	fn deserialize_newtype_struct<V: Visitor<'de>>(self, _name: &'static str, visitor: V) -> Result<V::Value, Self::Error> {
		visitor.visit_newtype_struct(self)
	}

	fn deserialize_tuple<V: Visitor<'de>>(self, _len: usize, visitor: V) -> Result<V::Value, Self::Error> {
		self.deserialize_seq(visitor)
	}

	fn deserialize_tuple_struct<V: Visitor<'de>>(
		self,
		_name: &'static str,
		_len: usize,
		visitor: V,
	) -> Result<V::Value, Self::Error> {
		self.deserialize_seq(visitor)
	}

	fn deserialize_unit<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
		visitor.visit_unit()
	}

	fn deserialize_unit_struct<V: Visitor<'de>>(self, _name: &'static str, visitor: V) -> Result<V::Value, Self::Error> {
		visitor.visit_unit()
	}

	fn deserialize_u32<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
		match self.0 {
			Value::Int(n) => visitor.visit_u32(n as u32),
			Value::Float(f) => visitor.visit_u32(f as u32),
			other => Err(DeError::type_mismatch("u32", other.type_name())),
		}
	}

	fn deserialize_u64<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
		match self.0 {
			Value::Int(n) => visitor.visit_u64(n as u64),
			Value::Float(f) => visitor.visit_u64(f as u64),
			other => Err(DeError::type_mismatch("u64", other.type_name())),
		}
	}

	forward_to_deserialize_any! {
		i8 i16 i32 i128 u8 u16 u128 f32 char bytes byte_buf
	}
}

struct ListSeqAccess {
	items: Vec<Value>,
	pos: usize,
}

impl<'de> SeqAccess<'de> for ListSeqAccess {
	type Error = DeError;

	fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, Self::Error>
	where
		T: de::DeserializeSeed<'de>,
	{
		if self.pos >= self.items.len() {
			return Ok(None);
		}
		let val = std::mem::replace(&mut self.items[self.pos], Value::Null);
		self.pos += 1;
		seed.deserialize(ValueDeserializer(val)).map(Some)
	}
}

struct MapMapAccess {
	entries: Vec<(String, Value)>,
	pos: usize,
}

impl From<HashMap<String, Value>> for MapMapAccess {
	fn from(map: HashMap<String, Value>) -> Self {
		Self {
			entries: map.into_iter().collect(),
			pos: 0,
		}
	}
}

impl<'de> MapAccess<'de> for MapMapAccess {
	type Error = DeError;

	fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>, Self::Error>
	where
		K: de::DeserializeSeed<'de>,
	{
		if self.pos >= self.entries.len() {
			return Ok(None);
		}
		let key = self.entries[self.pos].0.clone();
		self.pos += 1;
		seed.deserialize(key.into_deserializer()).map(Some)
	}

	fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value, Self::Error>
	where
		V: de::DeserializeSeed<'de>,
	{
		let val = std::mem::replace(&mut self.entries[self.pos - 1].1, Value::Null);
		seed.deserialize(ValueDeserializer(val))
	}
}

struct MapEnumAccess {
	map: HashMap<String, Value>,
}

impl<'de> de::EnumAccess<'de> for MapEnumAccess {
	type Error = DeError;
	type Variant = MapVariantAccess;

	fn variant_seed<V>(mut self, seed: V) -> Result<(V::Value, Self::Variant), Self::Error>
	where
		V: de::DeserializeSeed<'de>,
	{
		let tag = self
			.map
			.remove("type")
			.ok_or_else(|| DeError("enum map missing `type` field".to_string()))?;
		let variant = seed.deserialize(ValueDeserializer(tag))?;
		Ok((variant, MapVariantAccess { map: self.map }))
	}
}

struct MapVariantAccess {
	map: HashMap<String, Value>,
}

impl<'de> de::VariantAccess<'de> for MapVariantAccess {
	type Error = DeError;

	fn unit_variant(self) -> Result<(), Self::Error> {
		Ok(())
	}

	fn newtype_variant_seed<T>(self, seed: T) -> Result<T::Value, Self::Error>
	where
		T: de::DeserializeSeed<'de>,
	{
		seed.deserialize(ValueDeserializer(Value::Map(self.map)))
	}

	fn tuple_variant<V: Visitor<'de>>(self, _: usize, visitor: V) -> Result<V::Value, Self::Error> {
		visitor.visit_map(MapMapAccess::from(self.map))
	}

	fn struct_variant<V: Visitor<'de>>(self, _: &'static [&'static str], visitor: V) -> Result<V::Value, Self::Error> {
		visitor.visit_map(MapMapAccess::from(self.map))
	}
}
