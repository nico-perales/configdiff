//! Format detection and parsing of raw text into the [`Value`] model.

use std::path::Path;
use std::str::FromStr;

use crate::error::Error;
use crate::value::Value;

/// A supported configuration format.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    /// JSON.
    Json,
    /// TOML.
    Toml,
    /// YAML.
    Yaml,
}

impl Format {
    /// Infers a format from a file extension (case-insensitive), if recognized.
    #[must_use]
    pub fn from_extension(ext: &str) -> Option<Format> {
        match ext.to_ascii_lowercase().as_str() {
            "json" => Some(Format::Json),
            "toml" => Some(Format::Toml),
            "yaml" | "yml" => Some(Format::Yaml),
            _ => None,
        }
    }

    /// Infers a format from a path's extension, if recognized.
    #[must_use]
    pub fn from_path(path: &Path) -> Option<Format> {
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
        Format::Toml => Ok(toml::from_str(content)?),
        Format::Yaml => Ok(serde_norway::from_str(content)?),
    }
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
