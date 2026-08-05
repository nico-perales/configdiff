//! Paths that point at a location inside a configuration tree.
//!
//! A [`Path`] is rendered in a compact, familiar form: object keys are dotted
//! (`server.port`) and array elements are bracketed (`hosts[2].name`). Keys that
//! are not simple identifiers are quoted (`log["odd key"]`).

use std::fmt;

/// One step in a [`Path`]: either an object key or an array index.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Segment {
    /// An object key.
    Key(String),
    /// A zero-based array index.
    Index(usize),
}

/// A location inside a configuration tree, from the root down to a node.
///
/// Paths are cheap to clone and are built immutably as the diff recurses, so a
/// child path never disturbs its parent.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Path {
    segments: Vec<Segment>,
}

impl Path {
    /// The empty path, referring to the document root.
    #[must_use]
    pub fn root() -> Self {
        Path::default()
    }

    /// Returns a new path with an object key appended.
    #[must_use]
    pub fn child_key(&self, key: impl Into<String>) -> Self {
        let mut segments = self.segments.clone();
        segments.push(Segment::Key(key.into()));
        Path { segments }
    }

    /// Returns a new path with an array index appended.
    #[must_use]
    pub fn child_index(&self, index: usize) -> Self {
        let mut segments = self.segments.clone();
        segments.push(Segment::Index(index));
        Path { segments }
    }

    /// Returns `true` if this path is the document root.
    #[must_use]
    pub fn is_root(&self) -> bool {
        self.segments.is_empty()
    }

    /// The path's segments, root-first.
    #[must_use]
    pub fn segments(&self) -> &[Segment] {
        &self.segments
    }

    /// Renders the path as a `/`-joined match-string for ignore-glob matching:
    /// keys and indices become path segments (`server/hosts/2/name`). The root
    /// is the empty string.
    #[must_use]
    pub fn match_string(&self) -> String {
        let mut out = String::new();
        for (i, seg) in self.segments.iter().enumerate() {
            if i > 0 {
                out.push('/');
            }
            match seg {
                Segment::Key(k) => out.push_str(k),
                Segment::Index(idx) => {
                    use std::fmt::Write as _;
                    let _ = write!(out, "{idx}");
                }
            }
        }
        out
    }
}

/// A key is "simple" if it reads back unambiguously as a bare dotted segment.
fn is_simple_key(key: &str) -> bool {
    !key.is_empty()
        && key
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

impl fmt::Display for Path {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.segments.is_empty() {
            return f.write_str("(root)");
        }
        let mut first = true;
        for seg in &self.segments {
            match seg {
                Segment::Key(k) if is_simple_key(k) => {
                    if !first {
                        f.write_str(".")?;
                    }
                    f.write_str(k)?;
                }
                Segment::Key(k) => write!(f, "[{k:?}]")?,
                Segment::Index(i) => write!(f, "[{i}]")?,
            }
            first = false;
        }
        Ok(())
    }
}
