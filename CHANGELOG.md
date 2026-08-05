# Changelog

All notable changes to this project are documented here. The format is based on
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project adheres
to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Initial release.
- Semantic diff of TOML, YAML, and JSON via a common value model.
- Cross-format comparison (e.g. TOML against YAML).
- Type-aware diffing: distinguishes value changes from type changes.
- Array diffing strategies: LCS (default), positional, and key-based matching.
- Ignore paths by glob, loose number comparison, and float tolerance.
- `pretty` (colored) and `json` output renderers.
- `diff(1)`-style exit codes (`0` equal, `1` differ, `2` error).
- Library API with an optional `cli` feature so library consumers avoid CLI deps.

[Unreleased]: https://github.com/nico159756/configdiff/commits/main
