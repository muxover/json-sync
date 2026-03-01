# Json-Sync

<div align="center">

[![Crates.io](https://img.shields.io/crates/v/json-sync.svg)](https://crates.io/crates/json-sync)
[![Documentation](https://docs.rs/json-sync/badge.svg)](https://docs.rs/json-sync)
[![CI](https://github.com/muxover/json-sync/actions/workflows/ci.yml/badge.svg)](https://github.com/muxover/json-sync/actions/workflows/ci.yml)
[![License](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)

**Persistent JSON-backed key-value store with pluggable map backends.**

[Features](#-features) • [Quick Start](#-quick-start) • [Documentation](https://docs.rs/json-sync) • [API Overview](#-api-overview) • [Backends](#-backends) • [Configuration](#️-configuration) • [Non-goals](#-non-goals) • [Contributing](#-contributing) • [License](#-license)

</div>

---

Json-Sync is a thread-safe, in-memory key-value store that persists to a JSON file. You pick the map backend (ShardMap, `RwLock<HashMap>`, or DashMap) and the flush policy (immediate, async, or manual). It handles concurrency, serialization, and crash-safe writes for you.

Good for caches, config stores, session state, CLI tool persistence — anything that fits in memory and benefits from a human-readable file on disk.

## ✨ Features

- **Pluggable backends** — ShardMap (default), `RwLock<HashMap>`, DashMap, or your own via `MapBackend`.
- **Flush policies** — `Immediate` (every write), `Async(Duration)` (background thread), or `Manual`.
- **Crash-safe writes** — temp file + rename so you never get a half-written file.
- **Builder API** — configure flush policy, pretty-print JSON, and more.
- **Rich operations** — `insert`, `get`, `remove`, `clear`, `update`, `get_or_insert`, `extend`, `keys`, `values`, and more.
- **Generic** — any `K` and `V` that implement `Serialize + DeserializeOwned + Clone + Send + Sync` (plus `Hash + Eq` for keys).

## 📦 Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
json-sync = "0.1"
shardmap = "0.1"
```

**Optional features:**

| Feature   | Description |
|-----------|-------------|
| `dashmap` | Use DashMap as the map backend (adds `dashmap` dependency). |

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

    db.insert("hello".into(), "world".into())?;
    println!("{:?}", db.get(&"hello".into()));

    db.update(&"hello".into(), |v| *v = "rust".into())?;
    db.flush()?;
    Ok(())
}
```

## ✨ When to use Json-Sync

- You need a **simple persistent key-value store** with a human-readable file format.
- You want **pluggable backends** (ShardMap, RwLock<HashMap>, DashMap) and **configurable flush** (immediate, async, manual).
- You are building **caches**, **config stores**, **session state**, or **CLI persistence** that fits in memory.
- You are fine with **single-process** and **full-snapshot** persistence (no WAL, no multi-process).

## 📋 API Overview

### Store operations

| Method | Description |
|--------|-------------|
| `open(path)` | Open or create a store with manual flush. |
| `open_with_policy(path, policy)` | Open with a specific flush policy. |
| `builder(path)` | Start a builder for full control (policy, pretty-print). |
| `insert(key, value)` | Insert; returns the previous value if any. |
| `get(&key)` | Get a value. |
| `remove(&key)` | Remove a key; returns its value. |
| `clear()` | Drop all entries. |
| `update(&key, f)` | Mutate a value in place via closure. |
| `get_or_insert(key, default)` | Return existing value or insert the default. |
| `get_or_insert_with(key, f)` | Same, but computes the default lazily. |
| `extend(iter)` | Bulk insert from an iterator (single flush). |
| `keys()` | Snapshot of all keys. |
| `values()` | Snapshot of all values. |
| `iter()` | Snapshot of all key-value pairs. |
| `contains_key(&key)` | Check existence without cloning the value. |
| `len()` / `is_empty()` | Entry count. |
| `flush()` | Persist to disk now. |
| `path()` | Path to the backing file. |

### Flush policies

| Policy | Behavior |
|--------|----------|
| `FlushPolicy::Immediate` | Writes to disk after every mutation. |
| `FlushPolicy::Async(duration)` | Background thread flushes on a timer and on mutations. Dropping the handle joins the thread. |
| `FlushPolicy::Manual` | Only flushes when you call `flush()`. |

### Builder

```rust
use json_sync::{JsonSync, FlushPolicy};
use shardmap::ShardMap;
use std::time::Duration;

let db = JsonSync::<String, i32, ShardMap<String, i32>>::builder("data.json")
    .policy(FlushPolicy::Async(Duration::from_secs(5)))
    .pretty(true)
    .build()?;
# Ok::<(), json_sync::Error>(())
```

## 🔀 Backends

**ShardMap** (default) — concurrent sharded map with low contention. Best for most workloads.

```rust,no_run
use json_sync::JsonSync;
use shardmap::ShardMap;

let db = JsonSync::<String, i32, ShardMap<String, i32>>::open("db.json").unwrap();
```

**RwLock&lt;HashMap&gt;** — no extra crate needed (uses `parking_lot` from json-sync). Single reader-writer lock.

```rust,no_run
use json_sync::JsonSync;
use parking_lot::RwLock;
use std::collections::HashMap;

let db = JsonSync::<String, i32, RwLock<HashMap<String, i32>>>::open("db.json").unwrap();
```

**DashMap** (feature `dashmap`) — fast concurrent map, no tuning needed.

```rust,no_run
# #[cfg(feature = "dashmap")]
# {
use json_sync::JsonSync;
use dashmap::DashMap;

let db = JsonSync::<String, i32, DashMap<String, i32>>::open("db.json").unwrap();
# }
```

## ⚙️ Configuration

```rust
use json_sync::{JsonSync, FlushPolicy};
use shardmap::ShardMap;
use std::time::Duration;

// full control via builder
let db = JsonSync::<String, i32, ShardMap<String, i32>>::builder("data.json")
    .policy(FlushPolicy::Async(Duration::from_secs(5)))
    .pretty(true)
    .build()?;

// convenience: open with manual flush
let db = JsonSync::<String, i32, ShardMap<String, i32>>::open("db.json")?;
# Ok::<(), json_sync::Error>(())
```

By default the JSON file is compact (one line). Use `.pretty(true)` on the builder for indented output.

## Caveats

- **Single-process only.** Multiple processes writing to the same file will corrupt it. Use file locking or a real database for multi-process scenarios.
- **Atomic writes on Windows.** The temp-file-then-rename strategy is reliable on NTFS but has no hard guarantees on FAT32 or network drives.
- **Full snapshots.** Every flush serializes the entire map. This is fine for small-to-medium datasets but won't scale to millions of entries.
- **`update()` is not atomic.** It does a get → modify → put, so there's a brief race window with concurrent writers. Good enough for single-writer setups.

## 🚫 Non-goals

Json-Sync is focused. The following are explicitly **not** goals:

- **Transactions or queries** — No transactions, secondary indexes, or query language.
- **Replication or network sync** — No built-in replication or network sync.
- **WAL or compaction** — No write-ahead log or incremental compaction.
- **Format flexibility** — JSON only by default (the `Serializer` trait exists if you want to plug something else).

## 🧪 Tests

```bash
cargo test
cargo test --features dashmap
cargo test --all-features
```

Unit tests (in `src/`), integration tests (in `tests/`), and doc tests cover all backends, flush policies, builder options, and persistence round-trips.

## Examples

```bash
cargo run --example basic
cargo run --example builder
cargo run --example with_dashmap --features dashmap
```

## 🏁 Benchmarks

```bash
cargo bench
```

Benchmarks cover insert/get/remove, flush policies, and backends (ShardMap, RwLock<HashMap>, DashMap).

## 🤝 Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md). Open an [issue](https://github.com/muxover/json-sync/issues) or [pull request](https://github.com/muxover/json-sync/pulls) on GitHub.

## 📄 License

Licensed under the Apache License, Version 2.0 ([LICENSE](LICENSE) or http://www.apache.org/licenses/LICENSE-2.0).

## 🔗 Links

- **Repository**: https://github.com/muxover/json-sync
- **Issues**: https://github.com/muxover/json-sync/issues
- **Changelog**: [CHANGELOG.md](CHANGELOG.md)
- **Documentation**: https://docs.rs/json-sync

---

<div align="center">

Made with ❤️ by Jax (@muxover)

</div>
