# Contributing to Json-Sync

Thank you for your interest in contributing!

---

## Table of Contents

- [Getting started](#getting-started)
- [Running tests](#running-tests)
- [Running benchmarks](#running-benchmarks)
- [Code style](#code-style)
- [Submitting changes](#submitting-changes)
- [Releasing](#releasing)
- [Reporting issues](#reporting-issues)

---

## Getting started

**Requirements:**
- Rust stable (1.70+)
- Cargo

```bash
git clone https://github.com/muxover/json-sync.git
cd json-sync
cargo build
```

## Running tests

```bash
# default (no optional features)
cargo test

# with dashmap backend
cargo test --features dashmap

# everything
cargo test --all-features
```

## Running benchmarks

```bash
cargo bench
```

Benchmarks use [Criterion](https://github.com/bheisler/criterion.rs) and cover insert/get/remove, flush policies, and backends (ShardMap, RwLock<HashMap>, DashMap).

## Code style

- Run `cargo fmt` before committing. All code must pass `cargo fmt --check`.
- Run `cargo clippy --all-features` and fix any warnings.
- New public items must have doc comments (`///`).
- Prefer editing existing files over creating new ones.
- No dead code, no commented-out blocks, no debug prints in committed code.

## Submitting changes

1. **Open an issue first** for significant changes.
2. Fork and branch from `main`.
3. Add or update tests for your changes.
4. Ensure CI checks pass (`cargo test --all-features`, `cargo clippy --all-features`, `cargo fmt --check`).
5. Open a PR with a clear description.

One logical change per PR.

## Releasing

Releases run when you push a tag `v*` (e.g. `v0.1.1`). The workflow runs tests, publishes to crates.io, and creates a GitHub Release.

1. Bump `version` in `Cargo.toml` to match the release.
2. Update `CHANGELOG.md` (move Unreleased items to the new version).
3. Add `release-notes/vX.Y.Z.md`.
4. Commit, then tag: `git tag vX.Y.Z` and `git push origin vX.Y.Z`.

**Maintainers:** In this repo's GitHub Settings → Secrets and variables → Actions, add `CARGO_REGISTRY_TOKEN` with a crates.io API token. Without it, the publish step will fail.

## Reporting issues

Open an issue at https://github.com/muxover/json-sync/issues. Include:
- Rust version (`rustc --version`)
- OS and architecture
- Minimal reproducer
- Full error message or panic backtrace
