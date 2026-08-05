//! configdiff: semantic diff for config files (TOML/YAML/JSON/INI/.env).

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

// Parse two documents (inferring format when not given) and diff them.
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
