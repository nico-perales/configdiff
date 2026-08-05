# Contributing to configdiff

Thanks for your interest! Contributions of all kinds are welcome — bug reports,
feature ideas, docs, and code.

## Getting started

```bash
git clone https://github.com/nico159756/configdiff
cd configdiff
cargo test
```

## Before opening a pull request

Please make sure the same checks CI runs pass locally:

```bash
cargo fmt --all --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
```

A quick way to run all three:

```bash
cargo fmt --all --check && cargo clippy --all-targets --all-features -- -D warnings && cargo test
```

## Guidelines

- **Add a test for behavior changes.** Library behavior lives in `tests/behavior.rs`;
  end-to-end CLI behavior lives in `tests/cli.rs`.
- **Keep the library free of CLI dependencies.** Anything CLI-only (argument
  parsing, terminal handling) belongs behind the `cli` feature, in `src/main.rs`.
- **Document public items.** `missing_docs` is a warning; new public API needs a
  doc comment.
- **Update the CHANGELOG** under `## [Unreleased]` for user-visible changes.

## License

By contributing, you agree that your contributions will be dual licensed under the
MIT and Apache-2.0 licenses, without any additional terms or conditions.
