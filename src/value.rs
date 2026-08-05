//! Format-agnostic value model that every input parses into.

use std::fmt;

use indexmap::IndexMap;
use serde::de::{Deserialize, Deserializer, MapAccess, SeqAccess, Visitor};
use serde::ser::{Serialize, SerializeMap, Serializer};

pub type Map = IndexMap<String, Value>;

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Null,
    Bool(bool),
    Integer(i64),
    Float(f64),
    String(String),
    Array(Vec<Value>),
    Object(Map),
}

impl Value {
    #[must_use]
    pub fn type_name(&self) -> &'static str {
        match self {
            Value::Null => "null",
            Value::Bool(_) => "boolean",
            Value::Integer(_) => "integer",
            Value::Float(_) => "float",
            Value::String(_) => "string",
            Value::Array(_) => "array",
            Value::Object(_) => "object",
        }
    }

    #[must_use]
    pub fn is_scalar(&self) -> bool {
        !matches!(self, Value::Array(_) | Value::Object(_))
    }

    #[must_use]
    pub fn as_object(&self) -> Option<&Map> {
        match self {
            Value::Object(m) => Some(m),
            _ => None,
        }
    }

    #[must_use]
    pub fn as_array(&self) -> Option<&[Value]> {
        match self {
            Value::Array(a) => Some(a),
            _ => None,
        }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Null => f.write_str("null"),
            Value::Bool(b) => write!(f, "{b}"),
            Value::Integer(i) => write!(f, "{i}"),
            Value::Float(x) => write!(f, "{x}"),
            Value::String(s) => write!(f, "{s:?}"),
            Value::Array(a) => write!(f, "[{} items]", a.len()),
            Value::Object(m) => write!(f, "{{{} keys}}", m.len()),
        }
    }
}

impl Serialize for Value {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Value::Null => serializer.serialize_none(),
            Value::Bool(b) => serializer.serialize_bool(*b),
            Value::Integer(i) => serializer.serialize_i64(*i),
            Value::Float(x) => serializer.serialize_f64(*x),
            Value::String(s) => serializer.serialize_str(s),
            Value::Array(a) => a.serialize(serializer),
            Value::Object(m) => {
                let mut map = serializer.serialize_map(Some(m.len()))?;
                for (k, v) in m {
                    map.serialize_entry(k, v)?;
                }
                map.end()
            }
        }
    }
}

const TOML_DATETIME_KEY: &str = "$__toml_private_datetime";

impl<'de> Deserialize<'de> for Value {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(ValueVisitor)
    }
}

struct ValueVisitor;

impl<'de> Visitor<'de> for ValueVisitor {
    type Value = Value;

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("any valid configuration value")
    }

    fn visit_bool<E>(self, v: bool) -> Result<Value, E> {
        Ok(Value::Bool(v))
    }

    fn visit_i64<E>(self, v: i64) -> Result<Value, E> {
        Ok(Value::Integer(v))
    }

    #[allow(clippy::cast_precision_loss)]
    fn visit_i128<E>(self, v: i128) -> Result<Value, E> {
        Ok(i64::try_from(v).map_or_else(|_| Value::Float(v as f64), Value::Integer))
    }

    #[allow(clippy::cast_precision_loss)]
    fn visit_u64<E>(self, v: u64) -> Result<Value, E> {
        Ok(i64::try_from(v).map_or_else(|_| Value::Float(v as f64), Value::Integer))
    }

    #[allow(clippy::cast_precision_loss)]
    fn visit_u128<E>(self, v: u128) -> Result<Value, E> {
        Ok(i64::try_from(v).map_or_else(|_| Value::Float(v as f64), Value::Integer))
    }

    fn visit_f64<E>(self, v: f64) -> Result<Value, E> {
        Ok(Value::Float(v))
    }

    fn visit_str<E>(self, v: &str) -> Result<Value, E> {
        Ok(Value::String(v.to_owned()))
    }

    fn visit_string<E>(self, v: String) -> Result<Value, E> {
        Ok(Value::String(v))
    }

    fn visit_none<E>(self) -> Result<Value, E> {
        Ok(Value::Null)
    }

    fn visit_unit<E>(self) -> Result<Value, E> {
        Ok(Value::Null)
    }

    fn visit_some<D>(self, deserializer: D) -> Result<Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        Deserialize::deserialize(deserializer)
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let mut items = Vec::with_capacity(seq.size_hint().unwrap_or(0));
        while let Some(item) = seq.next_element()? {
            items.push(item);
        }
        Ok(Value::Array(items))
    }

    fn visit_map<A>(self, mut map: A) -> Result<Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let mut out = Map::with_capacity(map.size_hint().unwrap_or(0));
        while let Some((key, value)) = map.next_entry::<String, Value>()? {
            out.insert(key, value);
        }
        Ok(Value::Object(out))
    }
}

// Folds toml's datetime sentinel maps back into plain strings (TOML only).
pub(crate) fn collapse_toml_datetimes(value: &mut Value) {
    match value {
        Value::Object(map) => {
            if map.len() == 1 {
                if let Some(Value::String(s)) = map.get(TOML_DATETIME_KEY) {
                    let s = s.clone();
                    *value = Value::String(s);
                    return;
                }
            }
            for v in map.values_mut() {
                collapse_toml_datetimes(v);
            }
        }
        Value::Array(items) => {
            for v in items {
                collapse_toml_datetimes(v);
            }
        }
        _ => {}
    }
}
