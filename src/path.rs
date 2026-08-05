//! Paths locating a node inside a config tree (e.g. server.hosts[2].name).

use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Segment {
    Key(String),
    Index(usize),
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Path {
    segments: Vec<Segment>,
}

impl Path {
    #[must_use]
    pub fn root() -> Self {
        Path::default()
    }

    #[must_use]
    pub fn child_key(&self, key: impl Into<String>) -> Self {
        let mut segments = self.segments.clone();
        segments.push(Segment::Key(key.into()));
        Path { segments }
    }

    #[must_use]
    pub fn child_index(&self, index: usize) -> Self {
        let mut segments = self.segments.clone();
        segments.push(Segment::Index(index));
        Path { segments }
    }

    #[must_use]
    pub fn is_root(&self) -> bool {
        self.segments.is_empty()
    }

    #[must_use]
    pub fn segments(&self) -> &[Segment] {
        &self.segments
    }

    // `/`-joined form used for ignore-glob matching (server/hosts/2/name).
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
