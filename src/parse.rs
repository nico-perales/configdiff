//! Format detection and parsing of raw text into the Value model.

use std::path::Path;
use std::str::FromStr;

use crate::error::Error;
use crate::value::{Map, Value};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    Json,
    Toml,
    Yaml,
    Ini,
    Dotenv,
}

impl Format {
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

    #[must_use]
    pub fn from_path(path: &Path) -> Option<Format> {
        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
            if name == ".env" || name.starts_with(".env.") {
                return Some(Format::Dotenv);
            }
        }
        path.extension()
            .and_then(|e| e.to_str())
            .and_then(Format::from_extension)
    }

    #[must_use]
    pub fn guess_from_content(content: &str) -> Option<Format> {
        let trimmed = content.trim_start();
        let first = trimmed.chars().find(|c| !c.is_whitespace())?;
        match first {
            '{' | '[' => Some(Format::Json),
            '-' if trimmed.starts_with("---") => Some(Format::Yaml),
            _ => None,
        }
    }

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

pub fn parse(content: &str, format: Format) -> Result<Value, Error> {
    match format {
        Format::Json => Ok(serde_json::from_str(content)?),
        Format::Toml => {
            let mut value = toml::from_str(content)?;
            crate::value::collapse_toml_datetimes(&mut value);
            Ok(value)
        }
        Format::Yaml => Ok(serde_norway::from_str(content)?),
        Format::Ini => parse_ini(content),
        Format::Dotenv => Ok(parse_dotenv(content)),
    }
}

// INI -> nested object; section-less keys at the top level; all values strings.
fn parse_ini(content: &str) -> Result<Value, Error> {
    let ini = ini::Ini::load_from_str(content)?;
    let mut root = Map::new();
    for (section, properties) in &ini {
        let mut table = Map::new();
        for (key, value) in properties {
            table.insert(key.to_owned(), Value::String(value.to_owned()));
        }
        match section {
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

// dotenv -> flat object of string values.
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

pub fn parse_auto(content: &str, hint: Option<Format>) -> Result<Value, Error> {
    let format = hint
        .or_else(|| Format::guess_from_content(content))
        .ok_or_else(|| Error::UnknownFormat("<input>".to_owned()))?;
    parse(content, format)
}
