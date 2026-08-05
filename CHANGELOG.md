# Changelog

All notable changes to this project are documented here. The format is based on
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project adheres
to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Initial release.
- Semantic diff of TOML, YAML, JSON, INI, and dotenv (`.env`) via a common value
  model.
- Cross-format comparison (e.g. TOML against YAML).
- Type-aware diffing: distinguishes value changes from type changes.
- Array diffing strategies: LCS (default), positional, and key-based matching.
- Ignore paths by glob, loose number comparison, and float tolerance.
- `--expand` to report each leaf of an added/removed subtree individually.
- `--fail-on <kind>` to gate the exit code on specific change kinds (for CI drift
  detection), while still printing every change.
- `pretty` (colored) and `json` output renderers.
- `diff(1)`-style exit codes (`0` equal, `1` differ, `2` error).
- Library API with an optional `cli` feature so library consumers avoid CLI deps.
- Exact float round-tripping when parsing JSON (`serde_json`'s `float_roundtrip`),
  so precise numeric values are never silently altered.

### Security

- Bounded the diff recursion (depth limit) so pathologically deep input cannot
  overflow the stack; overly deep nodes report a single truncation marker.
- Capped the LCS array-diff matrix, falling back to positional comparison for very
  large arrays so two huge lists cannot exhaust memory.
- Added a `cargo audit` job to CI to catch dependency advisories.

[Unreleased]: https://github.com/nico159756/configdiff/commits/main
