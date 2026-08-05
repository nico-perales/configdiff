//! The crate's error type.

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    #[error("could not determine format for {0}: pass one explicitly")]
    UnknownFormat(String),

    #[error("JSON parse error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("TOML parse error: {0}")]
    Toml(#[from] toml::de::Error),

    #[error("YAML parse error: {0}")]
    Yaml(#[from] serde_norway::Error),

    #[error("INI parse error: {0}")]
    Ini(#[from] ini::ParseError),

    #[error("invalid ignore pattern: {0}")]
    Glob(#[from] globset::Error),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}
