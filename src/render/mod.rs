//! Renderers that turn a [`Diff`](crate::Diff) into output.
//!
//! Two formats are provided: a human-friendly [`pretty`] renderer (optionally
//! colored) and a machine-readable [`json`] renderer for tooling and CI.

pub mod json;
pub mod pretty;
