//! Format detection and parsing of raw text into the [`Value`] model.

use std::path::Path;
use std::str::FromStr;

use crate::error::Error;
use crate::value::{Map, Value};

/// A supported configuration format.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    /// JSON.
    Json,
    /// TOML.
    Toml,
    /// YAML.
    Yaml,
    /// INI. All values parse as strings; `[section]` blocks become nested
    /// objects and section-less keys sit at the top level.
    Ini,
    /// dotenv (`.env`). A flat set of `KEY=VALUE` pairs; all values are strings.
    Dotenv,
}

impl Format {
    /// Infers a format from a file extension (case-insensitive), if recognized.
    #[must_use]
    pub fn from_extension(ext: &str) -> Option<Format> {
        match ext.to_ascii_lowercase().as_str() {
            "json" => Some(Format::Json),
            "toml" => Some(Format::Toml),
            "yaml" | "yml" => Some(Format::Yaml),
            "ini" => Some(Format::Ini),
            "env" => Some(Format::Dotenv),
            _ => None,
        }
    }

    /// Infers a format from a path's extension or well-known filename.
    #[must_use]
    pub fn from_path(path: &Path) -> Option<Format> {
        // A file literally named `.env` (or `.env.local`, ...) has no extension
        // as far as the OS is concerned, so match on the file name too.
        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
            if name == ".env" || name.starts_with(".env.") {
                return Some(Format::Dotenv);
            }
        }
        path.extension()
            .and_then(|e| e.to_str())
            .and_then(Format::from_extension)
    }

    /// Best-effort guess of a format from the document's contents.
    ///
    /// This is only used when the extension is missing or unknown (for example
    /// when reading from stdin). It is deliberately conservative: it recognizes
    /// the unambiguous shapes and otherwise returns `None`.
    #[must_use]
    pub fn guess_from_content(content: &str) -> Option<Format> {
        let trimmed = content.trim_start();
        let first = trimmed.chars().find(|c| !c.is_whitespace())?;
        match first {
            // A document that opens with a brace or bracket is JSON (YAML flow
            // style is rare in config files and still parses as YAML below).
            '{' | '[' => Some(Format::Json),
            // An explicit YAML document marker.
            '-' if trimmed.starts_with("---") => Some(Format::Yaml),
            _ => None,
        }
    }

    /// The canonical lowercase name of this format.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Format::Json => "json",
            Format::Toml => "toml",
            Format::Yaml => "yaml",
            Format::Ini => "ini",
            Format::Dotenv => "env",
        }
    }
}

impl FromStr for Format {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Format::from_extension(s).ok_or_else(|| Error::UnknownFormat(s.to_owned()))
    }
}

impl std::fmt::Display for Format {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name())
    }
}

/// Parses `content` in the given `format` into a [`Value`].
///
/// # Errors
/// Returns the format-specific parse error if `content` is not valid.
pub fn parse(content: &str, format: Format) -> Result<Value, Error> {
    match format {
        Format::Json => Ok(serde_json::from_str(content)?),
        Format::Toml => {
            let mut value = toml::from_str(content)?;
            // TOML datetimes deserialize into a sentinel map; fold them back
            // into plain RFC 3339 strings. This is TOML-specific on purpose.
            crate::value::collapse_toml_datetimes(&mut value);
            Ok(value)
        }
        Format::Yaml => Ok(serde_norway::from_str(content)?),
        Format::Ini => parse_ini(content),
        Format::Dotenv => Ok(parse_dotenv(content)),
    }
}

/// Parses INI text into a nested object. Section-less keys live at the top
/// level; each `[section]` becomes a nested object. All values are strings.
fn parse_ini(content: &str) -> Result<Value, Error> {
    let ini = ini::Ini::load_from_str(content)?;
    let mut root = Map::new();
    for (section, properties) in &ini {
        let mut table = Map::new();
        for (key, value) in properties {
            table.insert(key.to_owned(), Value::String(value.to_owned()));
        }
        match section {
            // The default (section-less) properties go straight to the root.
            None => {
                for (k, v) in table {
                    root.insert(k, v);
                }
            }
            Some(name) => {
                root.insert(name.to_owned(), Value::Object(table));
            }
        }
    }
    Ok(Value::Object(root))
}

/// Parses dotenv (`.env`) text into a flat object of string values.
///
/// Recognizes `KEY=VALUE` lines, an optional leading `export `, `#` comments,
/// and single- or double-quoted values (quotes are stripped). This covers the
/// common cases without pulling in a full dotenv runtime.
fn parse_dotenv(content: &str) -> Value {
    let mut map = Map::new();
    for raw in content.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let line = line.strip_prefix("export ").unwrap_or(line);
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let key = key.trim();
        if key.is_empty() {
            continue;
        }
        let value = strip_dotenv_quotes(value.trim());
        map.insert(key.to_owned(), Value::String(value));
    }
    Value::Object(map)
}

/// Strips a single matching pair of surrounding quotes from a dotenv value.
fn strip_dotenv_quotes(value: &str) -> String {
    let bytes = value.as_bytes();
    if bytes.len() >= 2 {
        let first = bytes[0];
        let last = bytes[bytes.len() - 1];
        if (first == b'"' || first == b'\'') && first == last {
            return value[1..value.len() - 1].to_owned();
        }
    }
    value.to_owned()
}

/// Parses `content`, inferring the format from `hint` (an extension or format
/// name) if given, otherwise from the content itself.
///
/// # Errors
/// Returns [`Error::UnknownFormat`] if the format cannot be determined, or a
/// parse error if the content is invalid for the resolved format.
pub fn parse_auto(content: &str, hint: Option<Format>) -> Result<Value, Error> {
    let format = hint
        .or_else(|| Format::guess_from_content(content))
        .ok_or_else(|| Error::UnknownFormat("<input>".to_owned()))?;
    parse(content, format)
}
