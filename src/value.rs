//! The format-agnostic value model that every input is parsed into.
//!
//! TOML, YAML, and JSON are all deserialized into a single [`Value`] tree so the
//! diff engine never has to care which format a document came from. Object key
//! order is preserved (via [`IndexMap`]) so rendered output can follow the
//! document's own ordering instead of an arbitrary hash order.

use std::fmt;

use indexmap::IndexMap;
use serde::de::{Deserialize, Deserializer, MapAccess, SeqAccess, Visitor};
use serde::ser::{Serialize, SerializeMap, Serializer};

/// An ordered string-keyed map. Insertion order matches document order.
pub type Map = IndexMap<String, Value>;

/// A single node in a parsed configuration document.
///
/// Numbers are split into [`Value::Integer`] and [`Value::Float`] so the diff
/// engine can, optionally, treat `1` and `1.0` as different (a type change) or
/// equal, depending on [`crate::DiffOptions`].
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    /// A null / nil / absent-but-present value (`null`, `~`, YAML `null`).
    Null,
    /// A boolean.
    Bool(bool),
    /// A signed 64-bit integer.
    Integer(i64),
    /// A 64-bit floating point number. Also used as a fallback for integers
    /// that do not fit in an `i64` (e.g. large JSON numbers).
    Float(f64),
    /// A UTF-8 string. TOML datetimes are represented here in RFC 3339 form.
    String(String),
    /// An ordered sequence of values.
    Array(Vec<Value>),
    /// An ordered map of string keys to values.
    Object(Map),
}

impl Value {
    /// A short human-readable name for this value's type, used in diff output
    /// (`"string"`, `"integer"`, `"array"`, ...).
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

    /// Returns `true` for scalar (non-container) values.
    #[must_use]
    pub fn is_scalar(&self) -> bool {
        !matches!(self, Value::Array(_) | Value::Object(_))
    }

    /// Borrows the inner map if this is an [`Value::Object`].
    #[must_use]
    pub fn as_object(&self) -> Option<&Map> {
        match self {
            Value::Object(m) => Some(m),
            _ => None,
        }
    }

    /// Borrows the inner slice if this is an [`Value::Array`].
    #[must_use]
    pub fn as_array(&self) -> Option<&[Value]> {
        match self {
            Value::Array(a) => Some(a),
            _ => None,
        }
    }
}

impl fmt::Display for Value {
    /// A compact, single-line rendering of a value, suitable for inline diff
    /// output. Strings are quoted; containers are summarized.
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

// The key `toml` uses internally to smuggle datetimes through serde. When a TOML
// datetime is deserialized into an untyped structure it arrives as a one-entry
// map with this key and an RFC 3339 string value; we collapse it back to a
// plain string so datetimes diff sensibly across formats.
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

    // For integers that overflow `i64` we fall back to `f64`. That can lose
    // precision, but it is the best representation the value model offers for
    // out-of-range integers, and such values are vanishingly rare in config.
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

        // A TOML datetime arrives as a one-entry map keyed by the sentinel;
        // collapse it back into the plain RFC 3339 string it wraps.
        if out.len() == 1 {
            if let Some(Value::String(_)) = out.get(TOML_DATETIME_KEY) {
                if let Some((_, Value::String(s))) = out.swap_remove_index(0) {
                    return Ok(Value::String(s));
                }
            }
        }

        Ok(Value::Object(out))
    }
}
