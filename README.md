# json-sync

<div align="center">

[![Crates.io](https://img.shields.io/crates/v/json-sync.svg)](https://crates.io/crates/json-sync)
[![Documentation](https://docs.rs/json-sync/badge.svg)](https://docs.rs/json-sync)
[![License](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)

**Persistent JSON-backed key-value store with pluggable map backends.**

[Features](#-features) • [Quick Start](#-quick-start) • [Documentation](https://docs.rs/json-sync) • [Configuration](#️-configuration) • [API Overview](#-api-overview) • [Backends](#-backends) • [Non-goals](#-non-goals) • [Contributing](#-contributing) • [License](#-license)

</div>

---

json-sync is a general-purpose, thread-safe key-value store that persists to a JSON file. You choose the map backend (default: [ShardMap](https://github.com/muxover/shardmap)) and the flush policy (immediate, async, or manual). The store owns the map and handles concurrency and persistence for you.

## ✨ Features

- **Pluggable backends** — Use ShardMap (default), `RwLock<HashMap>`, or DashMap (optional feature). Implement `MapBackend` for custom maps.
- **Configurable flush** — `Immediate` (every insert/remove), `Async(Duration)` (background thread), or `Manual` (only when you call `flush()`).
- **Atomic writes** — Write to a temp file then rename for crash safety; no partial files.
- **Thread-safe** — All operations are safe for concurrent access; concurrency is delegated to the map backend.
- **Unified errors** — `insert`, `remove`, and `flush` return the same `Error` type (IO, serialize, deserialize, config).
- **Generic over K and V** — Any `K: Serialize + DeserializeOwned + Hash + Eq`, `V: Serialize + DeserializeOwned` (plus `Send + Sync + Clone`).

## 📦 Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
json-sync = "0.1"
shardmap = "0.2"
```

**Optional features:**

| Feature    | Description |
|-----------|-------------|
| `dashmap` | Use `DashMap<K, V>` as a backend: `JsonSync<K, V, DashMap<K, V>>`. |

```toml
# With DashMap backend
json-sync = { version = "0.1", features = ["dashmap"] }
dashmap = "6"
```

## 🚀 Quick Start

```rust
use json_sync::JsonSync;
use shardmap::ShardMap;

fn main() -> Result<(), json_sync::Error> {
    let db = JsonSync::<String, String, ShardMap<String, String>>::open("db.json")?;
    db.insert("key1".to_string(), "value1".to_string())?;
    let _val = db.get(&"key1".to_string());
    db.remove(&"key1".to_string())?;
    db.flush()?;
    Ok(())
}
```

## ✨ When to use json-sync

- You need a **simple persistent key-value store** backed by a JSON file.
- You want **concurrent reads and writes** with a choice of map (ShardMap for introspection and tuning, DashMap for simplicity, or `RwLock<HashMap>` for minimal deps).
- You want **configurable flush** — immediate consistency, batched async, or manual control.
- You are building **caches**, **session stores**, or **config stores** that fit in memory and benefit from JSON on disk.

## 📋 API Overview

### Store operations

| Method        | Description |
|---------------|-------------|
| `open(path)`  | Load or create the database. Returns a handle (derefs to the store). |
| `open_with_policy(path, policy)` | Same as `open` with a custom `FlushPolicy`. |
| `insert(key, value)` | Insert; returns previous value. Flushes or notifies according to policy. |
| `get(&key)`   | Get value. May clone `V` for backends that store `Arc<V>`. |
| `remove(&key)` | Remove; returns previous value. |
| `flush()`     | Persist current map to disk (atomic write). |
| `iter()`      | Snapshot of all entries `Vec<(K, V)>`. |
| `path()`      | Path to the backing JSON file. |

### Flush policy

| Policy            | Description |
|-------------------|-------------|
| `FlushPolicy::Immediate` | Flush after every insert/remove. |
| `FlushPolicy::Async(d)`  | Background thread flushes every `d` and on mutation; dropping the handle joins the thread (may block up to one interval). |
| `FlushPolicy::Manual`   | Flush only when you call `flush()`. |

## ⚙️ Configuration

```rust,no_run
use json_sync::{JsonSync, FlushPolicy};
use shardmap::ShardMap;
use std::time::Duration;

fn main() -> Result<(), json_sync::Error> {
    // Manual flush (default)
    let _db = JsonSync::<String, i32, ShardMap<String, i32>>::open("data.json")?;

    // Async flush every 5 seconds
    let _db = JsonSync::<String, i32, ShardMap<String, i32>>::open_with_policy(
        "data_async.json",
        FlushPolicy::Async(Duration::from_secs(5)),
    )?;

    // Immediate flush on every write
    let _db = JsonSync::<String, i32, ShardMap<String, i32>>::open_with_policy(
        "data_immediate.json",
        FlushPolicy::Immediate,
    )?;
    Ok(())
}
```

## 🔀 Backends

**Default: ShardMap** — Low resource use, predictable behavior, highly concurrent. Optional per-shard diagnostics. Best fit for most workloads.

```rust,no_run
use json_sync::JsonSync;
use shardmap::ShardMap;

fn main() -> Result<(), json_sync::Error> {
    let _db = JsonSync::<String, i32, ShardMap<String, i32>>::open("db.json")?;
    Ok(())
}
```

**RwLock&lt;HashMap&gt;** — No extra crate (uses `parking_lot` from json-sync); use when you want minimal dependencies. Single lock.

```rust,no_run
use json_sync::JsonSync;
use std::collections::HashMap;
use parking_lot::RwLock;

fn main() -> Result<(), json_sync::Error> {
    let _db = JsonSync::<String, i32, RwLock<HashMap<String, i32>>>::open("db.json")?;
    Ok(())
}
```

**DashMap** (optional feature) — Simple concurrent map; use when you want speed without tuning or introspection.

```toml
json-sync = { version = "0.1", features = ["dashmap"] }
dashmap = "6"
```

```rust,no_run
use json_sync::JsonSync;
use dashmap::DashMap;

fn main() -> Result<(), json_sync::Error> {
    let _db = JsonSync::<String, i32, DashMap<String, i32>>::open("db.json")?;
    Ok(())
}
```

## 📊 Snapshot and cloning

With ShardMap (and any backend that stores `Arc<V>`), `get` and `iter()` clone out of the Arc. For small or medium values this is fine; for **very large V**, the clone cost can be non-trivial. Documented in the crate and on the relevant methods.

## 🏗️ Design

- **Storage** — The map backend (e.g. ShardMap) lives inside the store; all reads/writes go through it. No global lock beyond the backend’s own locking.
- **Persistence** — Full snapshot to JSON; write to `path.tmp` then `fs::rename` to `path` for crash safety.
- **Async worker** — For `FlushPolicy::Async`, a background thread runs on an interval and on trigger; dropping the handle drops the trigger sender then joins the thread (graceful shutdown; may block up to one flush interval).

## 🚫 Non-goals

json-sync is focused. The following are explicitly **not** goals:

- **Database features** — No transactions, queries, or secondary indexes.
- **Network sync** — No built-in replication; use with other crates if needed.
- **Alternative formats** — Default is JSON only; custom serializers are a possible future extension.
- **WAL or compaction** — Atomic write is always on; optional compaction may be added later.

## 🤝 Contributing

Contributions are welcome. Please open an [issue](https://github.com/muxover/json-sync/issues) or [pull request](https://github.com/muxover/json-sync) on GitHub.

## 📄 License

Licensed under the Apache License, Version 2.0 ([LICENSE](LICENSE) or http://www.apache.org/licenses/LICENSE-2.0)

## 🔗 Links

- **Crates.io**: https://crates.io/crates/json-sync
- **Documentation**: https://docs.rs/json-sync
- **Repository**: https://github.com/muxover/json-sync
- **Issues**: https://github.com/muxover/json-sync/issues
- **Changelog**: [CHANGELOG.md](CHANGELOG.md)

---

<div align="center">

Made with ❤️ by Jax (@muxover)

[⭐ Star us on GitHub](https://github.com/muxover/json-sync) if you find this project useful!

</div>
