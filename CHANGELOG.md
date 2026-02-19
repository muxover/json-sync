# Changelog

## 0.1.0 (initial release)

- **JsonSync<K, V, M>** — Persistent JSON-backed key-value store with pluggable map backend.
- **MapBackend** trait — Implemented for ShardMap, RwLock<HashMap>, and (feature `dashmap`) DashMap.
- **FlushPolicy** — Immediate, Async(Duration), Manual. Async worker exits gracefully on drop.
- **open / open_with_policy** — Load or create; returns JsonSyncHandle (derefs to store).
- **insert, get, remove, flush, iter, path** — Unified Error type for all fallible operations.
- **Atomic writes** — Temp file then rename for crash safety.
- **Optimizations** — Pre-allocated flush/snapshot buffers; lighter async worker (fewer allocations).

See [release-notes/v0.1.0.md](release-notes/v0.1.0.md) for more detail.
