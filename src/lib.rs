//! # configdiff
//!
//! Semantic diff for configuration files. `configdiff` parses TOML, YAML, JSON,
//! INI, and dotenv (`.env`) into a single value model and compares them by
//! **structure and value**, not by text — so reordered keys and formatting
//! differences are invisible, and real changes stand out.
//!
//! Highlights:
//!
//! - **Cross-format.** Diff a `config.toml` against a `config.yaml`; both become
//!   the same [`Value`] tree first.
//! - **Type-aware.** `port = 8080` (integer) versus `port = "8080"` (string) is
//!   reported as a *type change*, not a value change.
//! - **Smart arrays.** Longest-common-subsequence matching detects inserted and
//!   removed elements; key-based matching diffs arrays of tables field-by-field.
//! - **Tunable.** Ignore paths by glob, compare numbers loosely, or set a float
//!   tolerance — see [`DiffOptions`].
//!
//! ## Example
//!
//! ```
//! use configdiff::{DiffOptions, Format, diff, parse};
//!
//! let old = parse(r#"{ "port": 8080, "debug": true }"#, Format::Json).unwrap();
//! let new = parse("port = 9090\n", Format::Toml).unwrap();
//!
//! let d = diff(&old, &new, &DiffOptions::default());
//! assert_eq!(d.len(), 2); // port changed, debug removed
//! ```

mod diff;
mod error;
mod options;
mod parse;
mod path;
mod value;

pub mod render;

pub use diff::{Change, ChangeKind, Diff, Summary, diff};
pub use error::Error;
pub use options::{ArrayStrategy, DiffOptions};
pub use parse::{Format, parse, parse_auto};
pub use path::{Path, Segment};
pub use value::{Map, Value};

/// Parses two documents (inferring format when a hint is not given) and diffs
/// them in one call.
///
/// This is a convenience wrapper over [`parse_auto`] and [`diff`] for the common
/// case of comparing two raw strings.
///
/// # Errors
/// Returns an [`Error`] if either document fails to parse or its format cannot
/// be determined.
pub fn diff_str(
    old: &str,
    new: &str,
    old_format: Option<Format>,
    new_format: Option<Format>,
    opts: &DiffOptions,
) -> Result<Diff, Error> {
    let old = parse_auto(old, old_format)?;
    let new = parse_auto(new, new_format)?;
    Ok(diff(&old, &new, opts))
}
