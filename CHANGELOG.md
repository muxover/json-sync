# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0] - 2026-03-02

### Added
- `JsonSync<K, V, M>` — persistent JSON-backed key-value store.
- `MapBackend` trait with implementations for ShardMap, `RwLock<HashMap>`, and DashMap (feature `dashmap`).
- Flush policies: `Immediate`, `Async(Duration)`, `Manual`.
- Builder API with pretty-print JSON option.
- Atomic writes via temp-file-then-rename.
- Operations: `insert`, `get`, `remove`, `clear`, `update`, `get_or_insert`, `get_or_insert_with`, `extend`, `keys`, `values`, `iter`, `contains_key`, `len`, `is_empty`, `flush`, `path`.
- `Debug` implementations for `JsonSync`, `JsonSyncHandle`, and `JsonSyncBuilder`.
- `#[must_use]` annotations on read-only methods.
- `#[non_exhaustive]` on `Error` and `FlushPolicy` for forward compatibility.

[Unreleased]: https://github.com/muxover/json-sync/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/muxover/json-sync/releases/tag/v0.1.0
