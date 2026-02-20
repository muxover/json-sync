# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - Initial release

### Added

- **JsonSync<K, V, M>** — Persistent JSON-backed key-value store with pluggable map backend.
- **MapBackend** trait — Implemented for ShardMap, RwLock<HashMap>, and (feature `dashmap`) DashMap.
- **FlushPolicy** — Immediate, Async(Duration), Manual. Async worker exits gracefully on drop.
- **open / open_with_policy** — Load or create; returns JsonSyncHandle (derefs to store).
- **insert, get, remove, flush, iter, path** — Unified Error type for all fallible operations.
- **Atomic writes** — Temp file then rename for crash safety.
- **Optimizations** — Pre-allocated flush/snapshot buffers; lighter async worker (fewer allocations).

See [release-notes/v0.1.0.md](release-notes/v0.1.0.md) for more detail.

[0.1.0]: https://github.com/muxover/json-sync/releases/tag/v0.1.0
