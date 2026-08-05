# configdiff

**Semantic diff for config files.** Compares TOML, YAML, JSON, INI, and dotenv
(`.env`) by *value and structure*, not by text — so reordered keys and formatting
differences never show up as changes, and the differences that matter stand out.

```text
$ configdiff old.toml new.yaml --array-key id
! port: integer 8080 -> string "8080"
~ server.host: "localhost" -> "0.0.0.0"
~ server.workers: 4 -> 8
+ server.timeout: 30
~ users[1].role: "viewer" -> "editor"

5 changes: +1  ~3  !1
```

## Why

`git diff` and `diff` compare text. For config files that produces noise: reorder
two keys, reindent a block, or convert `1` to `1.0`, and you get a diff full of
changes that don't change anything. Worse, a real change — a port that silently
became a string, a value dropped from a list — is buried in that noise.

`configdiff` parses each file into a common value model and compares *that*, so:

- **Reordered keys and formatting are invisible.** Only real differences appear.
- **It's cross-format.** Diff a `config.toml` against the `config.yaml` that
  replaced it — both become the same tree first.
- **It's type-aware.** `port = 8080` (integer) vs `port = "8080"` (string) is
  reported as a *type change* (`!`), not a value change. That class of bug —
  numbers that became strings — is exactly what text diff hides.
- **Arrays diff intelligently.** Inserting one element into a list doesn't cascade
  into "everything after it changed", and arrays of tables can be matched by a key
  field so a reordered-and-edited entry shows only the field that actually changed.

## Install

From source (requires a Rust toolchain):

```bash
cargo install --path .
```

Or, once published:

```bash
cargo install configdiff
```

## Usage

```bash
configdiff <OLD> <NEW> [OPTIONS]
```

`OLD` and `NEW` are files; use `-` for standard input. The format is inferred from
each file's extension and can be overridden.

### Common examples

```bash
# Straightforward semantic diff
configdiff config.old.toml config.new.toml

# Cross-format: did the YAML rewrite preserve the TOML's meaning?
configdiff config.toml config.yaml

# Match list-of-tables by a key so edits diff field-by-field
configdiff a.yaml b.yaml --array-key id --array-key name

# Ignore volatile fields anywhere in the tree
configdiff a.json b.json --ignore '**/updated_at' --ignore 'metadata/*'

# Machine-readable output for tooling / CI
configdiff a.toml b.toml -o json

# Use in a script: exit code tells you if anything changed
configdiff a.toml b.toml --quiet && echo "no drift"

# CI drift gate: only fail the build on removals or type changes
configdiff baseline.toml deployed.toml --fail-on removed --fail-on type-changed

# Expand added/removed subtrees to see every leaf, not just "{3 keys}"
configdiff a.json b.json --expand
```

### Supported formats

| Format | Extensions | Notes |
| --- | --- | --- |
| JSON | `.json` | |
| TOML | `.toml` | Datetimes compare as RFC 3339 strings. |
| YAML | `.yaml`, `.yml` | |
| INI | `.ini` | `[section]` blocks become nested objects; all values are strings. |
| dotenv | `.env`, `.env.*` | Flat `KEY=VALUE` pairs; all values are strings. |

Because INI and dotenv have no types, their values are always strings — comparing
`.env`'s `PORT=8080` against JSON's `"port": 8080` reports a type change, which is
usually what you want.

### Options

| Option | Description |
| --- | --- |
| `-f, --format <FMT>` | Force the format of both inputs (`json`, `toml`, `yaml`, `ini`, `env`). |
| `--old-format`, `--new-format` | Force one side's format (overrides `--format`). |
| `-o, --output <FMT>` | `pretty` (default) or `json`. |
| `--color <WHEN>` | `auto` (default), `always`, or `never`. Honors `NO_COLOR`. |
| `--ignore <GLOB>` | Ignore matching paths (repeatable). See below. |
| `--array <STRATEGY>` | `lcs` (default), `positional`, or `keyed`. |
| `--array-key <KEY>` | Key field for matching array elements (repeatable). Implies `--array keyed`. |
| `--loose-numbers` | Treat `1` and `1.0` as equal. |
| `--float-tolerance <EPS>` | Consider floats within `EPS` equal. |
| `--expand` | Report each leaf of an added/removed subtree instead of summarizing it. |
| `--fail-on <KIND>` | Only exit non-zero on these change kinds (repeatable): `added`, `removed`, `changed`, `type-changed`. |
| `-q, --quiet` | No output; communicate only via the exit code. |
| `--exit-zero` | Always exit `0`, even when the documents differ. |

### Exit codes

Like `diff(1)`:

| Code | Meaning |
| --- | --- |
| `0` | The documents are semantically equal. |
| `1` | They differ. |
| `2` | An error occurred (bad input, unknown format, ...). |

`--fail-on` narrows what counts as "differ" for the exit code (e.g. fail only on
removals), while still printing every change. `--exit-zero` forces `0` regardless.

### Ignore patterns

Ignore globs are matched against a `/`-joined form of each path, where object keys
and array indices are segments — the node at `server.hosts[2].name` is matched as
`server/hosts/2/name`. Use `*` within a segment and `**` across segments:

- `**/updated_at` — every `updated_at` key at any depth
- `metadata/*` — every direct child of `metadata`
- `server/hosts/*/name` — the `name` of every host

### Array strategies

- **`lcs`** (default) — longest-common-subsequence matching. Detects inserted and
  removed elements without cascading. Best for scalar lists and lists where whole
  elements come and go. A scalar edited in place shows as a removal plus an
  addition (an LCS can't know it was an edit).
- **`positional`** — compares element `i` on the left with element `i` on the
  right. Simple and predictable, but one insertion shifts everything after it.
- **`keyed`** — matches array-of-table elements by one or more key fields
  (`--array-key`), so a reordered-and-edited entry diffs field-by-field. Elements
  without a usable key fall back to additions/removals.

## Library

`configdiff` is a library first; the CLI is a thin layer on top. Add it without
the CLI dependencies:

```toml
[dependencies]
configdiff = { version = "0.1", default-features = false }
```

```rust
use configdiff::{diff, parse, DiffOptions, Format};

let old = parse(r#"{ "port": 8080, "debug": true }"#, Format::Json)?;
let new = parse("port = 9090\n", Format::Toml)?;

let d = diff(&old, &new, &DiffOptions::default());
for change in d.changes() {
    println!("{}: {:?}", change.path, change.kind);
}
# Ok::<(), configdiff::Error>(())
```

See the [API docs](https://docs.rs/configdiff) for `DiffOptions`, the `Value`
model, and the `render` module.

## How it works

```text
old ──▶ parse ──┐
                ├──▶ Value tree ──▶ diff engine ──▶ Diff ──▶ render (pretty | json)
new ──▶ parse ──┘
```

Every input is deserialized into one format-agnostic `Value` type (with ordered
object keys preserved), so the diff engine never has to care which format a
document came from. That single decision is what makes cross-format diffing,
type-awareness, and consistent output fall out naturally.

## Security & limits

`configdiff` reads only the inputs you give it and writes only to stdout/stderr —
no network, no telemetry. Still, if you point it at **untrusted** documents (e.g.
diffing config from an unknown source, or embedding the library in a service),
keep these bounds in mind:

- **Recursion is depth-limited.** Extremely deep input cannot overflow the stack;
  nodes past the limit report a single truncation marker instead of recursing.
- **Array diffing is memory-bounded.** Very large arrays skip the LCS matrix and
  fall back to positional comparison, so two huge lists cannot exhaust memory.
- **Parser depth for untrusted YAML/TOML** is governed by the underlying parser,
  not by `configdiff`. When processing untrusted input in a service, run it under
  an external resource limit (memory/CPU) as defense in depth.
- **Values are printed in full**, including secrets from `.env`/config files. Do
  not pipe the diff of secret-bearing files into logs you don't control.

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option. Unless you explicitly state otherwise, any contribution you
intentionally submit for inclusion shall be dual licensed as above, without any
additional terms or conditions.
