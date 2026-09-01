//! Options controlling how strictly two documents are compared.

use globset::{Glob, GlobSet, GlobSetBuilder};

use crate::error::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ArrayStrategy {
    /// Match arrays of objects by an automatically inferred identity key (`id`,
    /// `name`, …), falling back to `Lcs` when no such key fits. The default.
    #[default]
    Auto,
    Lcs,
    Positional,
    Keyed,
}

#[derive(Debug, Clone, Default)]
pub struct DiffOptions {
    pub numbers_loose: bool,
    pub float_tolerance: Option<f64>,
    pub(crate) ignore: Option<GlobSet>,
    pub array_strategy: ArrayStrategy,
    pub array_keys: Vec<String>,
    pub expand: bool,
}

impl DiffOptions {
    #[must_use]
    pub fn numbers_loose(mut self, yes: bool) -> Self {
        self.numbers_loose = yes;
        self
    }

    #[must_use]
    pub fn float_tolerance(mut self, tol: Option<f64>) -> Self {
        self.float_tolerance = tol;
        self
    }

    #[must_use]
    pub fn array_strategy(mut self, strategy: ArrayStrategy) -> Self {
        self.array_strategy = strategy;
        self
    }

    #[must_use]
    pub fn array_keys(mut self, keys: Vec<String>) -> Self {
        self.array_keys = keys;
        self
    }

    #[must_use]
    pub fn expand(mut self, yes: bool) -> Self {
        self.expand = yes;
        self
    }

    // Compiles ignore globs; patterns match the `/`-joined path form.
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

    pub(crate) fn is_ignored(&self, match_string: &str) -> bool {
        self.ignore
            .as_ref()
            .is_some_and(|set| set.is_match(match_string))
    }
}
