//! Knobs that control how strictly two documents are compared.

use globset::{Glob, GlobSet, GlobSetBuilder};

use crate::error::Error;

/// How arrays are compared.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ArrayStrategy {
    /// Longest-common-subsequence matching: detects inserted and removed
    /// elements instead of reporting every element after a shift as "changed".
    /// This is the default and the right choice for most config lists.
    #[default]
    Lcs,
    /// Strict positional comparison: element `i` on the left is compared with
    /// element `i` on the right. Fast, but a single insertion cascades.
    Positional,
    /// Match array-of-object elements by one or more key fields (see
    /// [`DiffOptions::array_keys`]). Falls back to [`ArrayStrategy::Lcs`] for
    /// elements that lack every configured key.
    Keyed,
}

/// Options controlling equality and array handling during a diff.
///
/// Construct with [`DiffOptions::default`] and adjust fields, or use the
/// chaining helpers. Everything is opt-in: the defaults give an exact,
/// type-aware comparison.
#[derive(Debug, Clone, Default)]
pub struct DiffOptions {
    /// Treat integers and floats with the same numeric value as equal
    /// (`1` == `1.0`). Off by default, so a change from `1` to `1.0` is
    /// reported as a type change.
    pub numbers_loose: bool,
    /// Absolute tolerance when comparing floats (and loose int/float pairs).
    /// `None` means exact bit-for-bit-ish comparison.
    pub float_tolerance: Option<f64>,
    /// Compiled set of paths to ignore. Built from glob patterns via
    /// [`DiffOptions::ignore`].
    pub(crate) ignore: Option<GlobSet>,
    /// Strategy used to diff arrays.
    pub array_strategy: ArrayStrategy,
    /// Key fields used by [`ArrayStrategy::Keyed`] to match object elements,
    /// tried in order (first key present on both sides wins).
    pub array_keys: Vec<String>,
    /// When an entire subtree is added or removed, report every leaf inside it
    /// as its own change instead of a single change carrying the whole subtree.
    pub expand: bool,
}

impl DiffOptions {
    /// Enables loose numeric comparison (`1` == `1.0`).
    #[must_use]
    pub fn numbers_loose(mut self, yes: bool) -> Self {
        self.numbers_loose = yes;
        self
    }

    /// Sets an absolute float comparison tolerance.
    #[must_use]
    pub fn float_tolerance(mut self, tol: Option<f64>) -> Self {
        self.float_tolerance = tol;
        self
    }

    /// Sets the array diffing strategy.
    #[must_use]
    pub fn array_strategy(mut self, strategy: ArrayStrategy) -> Self {
        self.array_strategy = strategy;
        self
    }

    /// Sets the key fields used for [`ArrayStrategy::Keyed`] matching.
    #[must_use]
    pub fn array_keys(mut self, keys: Vec<String>) -> Self {
        self.array_keys = keys;
        self
    }

    /// Enables leaf-level expansion of added and removed subtrees.
    #[must_use]
    pub fn expand(mut self, yes: bool) -> Self {
        self.expand = yes;
        self
    }

    /// Compiles a set of ignore glob patterns.
    ///
    /// Patterns are matched against a `/`-joined form of each path, where object
    /// keys and array indices are segments: the node at `server.hosts[2].name`
    /// is matched as `server/hosts/2/name`. Use `*` to match within a segment
    /// and `**` to match across segments — e.g. `**/updated_at` ignores every
    /// `updated_at` key at any depth, and `secrets/*` ignores every direct child
    /// of `secrets`.
    ///
    /// # Errors
    /// Returns [`Error::Glob`] if any pattern is not a valid glob.
    pub fn ignore<I, S>(mut self, patterns: I) -> Result<Self, Error>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let mut builder = GlobSetBuilder::new();
        let mut any = false;
        for pat in patterns {
            builder.add(Glob::new(pat.as_ref())?);
            any = true;
        }
        self.ignore = if any { Some(builder.build()?) } else { None };
        Ok(self)
    }

    /// Returns `true` if the given match-string is covered by an ignore pattern.
    pub(crate) fn is_ignored(&self, match_string: &str) -> bool {
        self.ignore
            .as_ref()
            .is_some_and(|set| set.is_match(match_string))
    }
}
