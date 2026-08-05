//! The crate's error type.

/// Errors that can occur while parsing input or configuring a diff.
///
/// Diffing itself is infallible once both sides have parsed; these errors all
/// come from reading, decoding, or configuring inputs.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// A format could not be inferred from a file's extension or contents.
    #[error("could not determine format for {0}: pass one explicitly")]
    UnknownFormat(String),

    /// Failed to parse a document as JSON.
    #[error("JSON parse error: {0}")]
    Json(#[from] serde_json::Error),

    /// Failed to parse a document as TOML.
    #[error("TOML parse error: {0}")]
    Toml(#[from] toml::de::Error),

    /// Failed to parse a document as YAML.
    #[error("YAML parse error: {0}")]
    Yaml(#[from] serde_norway::Error),

    /// Failed to parse a document as INI.
    #[error("INI parse error: {0}")]
    Ini(#[from] ini::ParseError),

    /// An ignore glob pattern was invalid.
    #[error("invalid ignore pattern: {0}")]
    Glob(#[from] globset::Error),

    /// An I/O error occurred while reading input.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}
