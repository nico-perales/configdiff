//! Diff engine: compares two Value trees into a list of Changes.

use crate::options::{ArrayStrategy, DiffOptions};
use crate::path::Path;
use crate::value::Value;

#[derive(Debug, Clone, PartialEq)]
pub enum ChangeKind {
    Added { new: Value },
    Removed { old: Value },
    Changed { old: Value, new: Value },
    TypeChanged { old: Value, new: Value },
}

#[derive(Debug, Clone, PartialEq)]
pub struct Change {
    pub path: Path,
    pub kind: ChangeKind,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Diff {
    changes: Vec<Change>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Summary {
    pub added: usize,
    pub removed: usize,
    pub changed: usize,
    pub type_changed: usize,
}

impl Diff {
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.changes.is_empty()
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.changes.len()
    }

    #[must_use]
    pub fn changes(&self) -> &[Change] {
        &self.changes
    }

    #[must_use]
    pub fn into_changes(self) -> Vec<Change> {
        self.changes
    }

    #[must_use]
    pub fn summary(&self) -> Summary {
        let mut s = Summary::default();
        for c in &self.changes {
            match c.kind {
                ChangeKind::Added { .. } => s.added += 1,
                ChangeKind::Removed { .. } => s.removed += 1,
                ChangeKind::Changed { .. } => s.changed += 1,
                ChangeKind::TypeChanged { .. } => s.type_changed += 1,
            }
        }
        s
    }
}

#[must_use]
pub fn diff(old: &Value, new: &Value, opts: &DiffOptions) -> Diff {
    let mut changes = Vec::new();
    diff_values(old, new, &Path::root(), opts, &mut changes);
    Diff { changes }
}

fn diff_values(old: &Value, new: &Value, path: &Path, opts: &DiffOptions, out: &mut Vec<Change>) {
    if !path.is_root() && opts.is_ignored(&path.match_string()) {
        return;
    }

    match (old, new) {
        (Value::Object(a), Value::Object(b)) => diff_objects(a, b, path, opts, out),
        (Value::Array(a), Value::Array(b)) => diff_arrays(a, b, path, opts, out),
        (a, b) if a.is_scalar() && b.is_scalar() => match scalar_relation(a, b, opts) {
            ScalarRel::Equal => {}
            ScalarRel::Changed => out.push(Change {
                path: path.clone(),
                kind: ChangeKind::Changed {
                    old: a.clone(),
                    new: b.clone(),
                },
            }),
            ScalarRel::TypeChanged => out.push(Change {
                path: path.clone(),
                kind: ChangeKind::TypeChanged {
                    old: a.clone(),
                    new: b.clone(),
                },
            }),
        },
        (a, b) => out.push(Change {
            path: path.clone(),
            kind: ChangeKind::TypeChanged {
                old: a.clone(),
                new: b.clone(),
            },
        }),
    }
}

// Records an addition, expanding subtrees leaf-by-leaf when opts.expand is set.
fn emit_added(value: &Value, path: &Path, opts: &DiffOptions, out: &mut Vec<Change>) {
    if opts.is_ignored(&path.match_string()) {
        return;
    }
    if opts.expand {
        match value {
            Value::Object(m) if !m.is_empty() => {
                for (k, v) in m {
                    emit_added(v, &path.child_key(k.clone()), opts, out);
                }
                return;
            }
            Value::Array(a) if !a.is_empty() => {
                for (i, v) in a.iter().enumerate() {
                    emit_added(v, &path.child_index(i), opts, out);
                }
                return;
            }
            _ => {}
        }
    }
    out.push(Change {
        path: path.clone(),
        kind: ChangeKind::Added { new: value.clone() },
    });
}

// Removal-side mirror of emit_added.
fn emit_removed(value: &Value, path: &Path, opts: &DiffOptions, out: &mut Vec<Change>) {
    if opts.is_ignored(&path.match_string()) {
        return;
    }
    if opts.expand {
        match value {
            Value::Object(m) if !m.is_empty() => {
                for (k, v) in m {
                    emit_removed(v, &path.child_key(k.clone()), opts, out);
                }
                return;
            }
            Value::Array(a) if !a.is_empty() => {
                for (i, v) in a.iter().enumerate() {
                    emit_removed(v, &path.child_index(i), opts, out);
                }
                return;
            }
            _ => {}
        }
    }
    out.push(Change {
        path: path.clone(),
        kind: ChangeKind::Removed { old: value.clone() },
    });
}

fn diff_objects(
    a: &crate::value::Map,
    b: &crate::value::Map,
    path: &Path,
    opts: &DiffOptions,
    out: &mut Vec<Change>,
) {
    for (key, av) in a {
        let child = path.child_key(key.clone());
        match b.get(key) {
            Some(bv) => diff_values(av, bv, &child, opts, out),
            None => emit_removed(av, &child, opts, out),
        }
    }
    for (key, bv) in b {
        if !a.contains_key(key) {
            emit_added(bv, &path.child_key(key.clone()), opts, out);
        }
    }
}

fn diff_arrays(a: &[Value], b: &[Value], path: &Path, opts: &DiffOptions, out: &mut Vec<Change>) {
    match opts.array_strategy {
        ArrayStrategy::Positional => diff_arrays_positional(a, b, path, opts, out),
        ArrayStrategy::Lcs => diff_arrays_lcs(a, b, path, opts, out),
        ArrayStrategy::Keyed => {
            if opts.array_keys.is_empty() {
                diff_arrays_lcs(a, b, path, opts, out);
            } else {
                diff_arrays_keyed(a, b, path, opts, out);
            }
        }
    }
}

fn diff_arrays_positional(
    a: &[Value],
    b: &[Value],
    path: &Path,
    opts: &DiffOptions,
    out: &mut Vec<Change>,
) {
    let common = a.len().min(b.len());
    for i in 0..common {
        diff_values(&a[i], &b[i], &path.child_index(i), opts, out);
    }
    for (i, av) in a.iter().enumerate().skip(common) {
        emit_removed(av, &path.child_index(i), opts, out);
    }
    for (i, bv) in b.iter().enumerate().skip(common) {
        emit_added(bv, &path.child_index(i), opts, out);
    }
}

// LCS-based array diff: anchors equal elements, reports the rest as add/remove.
#[allow(clippy::many_single_char_names)]
fn diff_arrays_lcs(
    a: &[Value],
    b: &[Value],
    path: &Path,
    opts: &DiffOptions,
    out: &mut Vec<Change>,
) {
    let n = a.len();
    let m = b.len();
    let mut dp = vec![vec![0usize; m + 1]; n + 1];
    for i in (0..n).rev() {
        for j in (0..m).rev() {
            dp[i][j] = if deep_equal(&a[i], &b[j], opts) {
                dp[i + 1][j + 1] + 1
            } else {
                dp[i + 1][j].max(dp[i][j + 1])
            };
        }
    }

    let (mut i, mut j) = (0, 0);
    while i < n && j < m {
        if deep_equal(&a[i], &b[j], opts) {
            i += 1;
            j += 1;
        } else if dp[i + 1][j] >= dp[i][j + 1] {
            emit_removed(&a[i], &path.child_index(i), opts, out);
            i += 1;
        } else {
            emit_added(&b[j], &path.child_index(j), opts, out);
            j += 1;
        }
    }
    while i < n {
        emit_removed(&a[i], &path.child_index(i), opts, out);
        i += 1;
    }
    while j < m {
        emit_added(&b[j], &path.child_index(j), opts, out);
        j += 1;
    }
}

// Key-based array diff: matches object elements by a shared key field.
fn diff_arrays_keyed(
    a: &[Value],
    b: &[Value],
    path: &Path,
    opts: &DiffOptions,
    out: &mut Vec<Change>,
) {
    let mut new_by_key: Vec<(Value, usize, bool)> = b
        .iter()
        .enumerate()
        .filter_map(|(idx, v)| key_of(v, &opts.array_keys).map(|k| (k, idx, false)))
        .collect();

    for (i, av) in a.iter().enumerate() {
        match key_of(av, &opts.array_keys) {
            Some(akey) => {
                let matched = new_by_key
                    .iter_mut()
                    .find(|(k, _, used)| !*used && *k == akey);
                match matched {
                    Some((_, new_idx, used)) => {
                        *used = true;
                        let new_idx = *new_idx;
                        diff_values(av, &b[new_idx], &path.child_index(i), opts, out);
                    }
                    None => emit_removed(av, &path.child_index(i), opts, out),
                }
            }
            None => emit_removed(av, &path.child_index(i), opts, out),
        }
    }

    let mut consumed = new_by_key.into_iter().filter(|(_, _, used)| *used).fold(
        std::collections::HashSet::new(),
        |mut set, (_, idx, _)| {
            set.insert(idx);
            set
        },
    );
    for (j, bv) in b.iter().enumerate() {
        let is_keyed_and_consumed = consumed.remove(&j);
        if !is_keyed_and_consumed {
            emit_added(bv, &path.child_index(j), opts, out);
        }
    }
}

fn key_of(value: &Value, keys: &[String]) -> Option<Value> {
    let obj = value.as_object()?;
    for k in keys {
        if let Some(v) = obj.get(k) {
            if v.is_scalar() {
                return Some(v.clone());
            }
        }
    }
    None
}

enum ScalarRel {
    Equal,
    Changed,
    TypeChanged,
}

fn scalar_relation(a: &Value, b: &Value, opts: &DiffOptions) -> ScalarRel {
    if scalar_equal(a, b, opts) {
        return ScalarRel::Equal;
    }
    if a.type_name() == b.type_name() {
        return ScalarRel::Changed;
    }
    if is_numeric(a) && is_numeric(b) && opts.numbers_loose {
        return ScalarRel::Changed;
    }
    ScalarRel::TypeChanged
}

fn is_numeric(v: &Value) -> bool {
    matches!(v, Value::Integer(_) | Value::Float(_))
}

#[allow(clippy::cast_precision_loss)]
fn numeric_value(v: &Value) -> Option<f64> {
    match v {
        Value::Integer(i) => Some(*i as f64),
        Value::Float(f) => Some(*f),
        _ => None,
    }
}

fn scalar_equal(a: &Value, b: &Value, opts: &DiffOptions) -> bool {
    match (a, b) {
        (Value::Null, Value::Null) => true,
        (Value::Bool(x), Value::Bool(y)) => x == y,
        (Value::String(x), Value::String(y)) => x == y,
        _ if is_numeric(a) && is_numeric(b) => {
            if !opts.numbers_loose && a.type_name() != b.type_name() {
                return false;
            }
            match (numeric_value(a), numeric_value(b)) {
                (Some(x), Some(y)) => match opts.float_tolerance {
                    Some(tol) => (x - y).abs() <= tol,
                    #[allow(clippy::float_cmp)]
                    None => x == y,
                },
                _ => false,
            }
        }
        _ => false,
    }
}

fn deep_equal(a: &Value, b: &Value, opts: &DiffOptions) -> bool {
    match (a, b) {
        (Value::Object(x), Value::Object(y)) => {
            x.len() == y.len()
                && x.iter()
                    .all(|(k, xv)| y.get(k).is_some_and(|yv| deep_equal(xv, yv, opts)))
        }
        (Value::Array(x), Value::Array(y)) => {
            x.len() == y.len()
                && x.iter()
                    .zip(y.iter())
                    .all(|(xv, yv)| deep_equal(xv, yv, opts))
        }
        _ if a.is_scalar() && b.is_scalar() => scalar_equal(a, b, opts),
        _ => false,
    }
}
