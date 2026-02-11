# JsonSync

<div align="center">

[![Crates.io](https://img.shields.io/crates/v/json-sync.svg)](https://crates.io/crates/json-sync)
[![Documentation](https://docs.rs/json-sync/badge.svg)](https://docs.rs/json-sync)
[![License](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)

**High-performance JSON synchronization for map-like structures**

[Features](#-features) • [Quick Start](#-quick-start) • [Documentation](https://docs.rs/json-sync) • [Examples](#-examples) • [Integration](#integration-with-shardmap) • [Performance](#-performance)

</div>

---

JsonSync is a production-ready Rust library that provides JSON persistence and synchronization for map-like structures. It offers asynchronous flushing, configurable dirty tracking, and recovery mechanisms, making it ideal for high-throughput applications that need reliable data persistence.

**Optimized for [ShardMap](https://github.com/muxover/shardmap)**: JsonSync works best with [ShardMap](https://github.com/muxover/shardmap), a high-performance concurrent sharded map. Together, they provide a powerful solution for managing millions of key-value entries with atomic updates, high-concurrency reads/writes, and reliable JSON persistence.

## ✨ Features

- 🚀 **Asynchronous Flushing**: Time-based and count-based flush strategies
- 🔄 **Configurable Dirty Tracking**: Per-shard or per-key tracking for optimal performance
- 📦 **Multiple Serialization Formats**: JSON (default) and optional binary (bincode) for speed
- 🔒 **Thread-Safe**: All operations are safe for concurrent access
- ⚡ **Non-Blocking**: Flush operations don't block map operations
- 📊 **Statistics**: Monitor flush operations and performance metrics
- 🛡️ **Recovery Mechanisms**: Robust file loading with error handling
- 🎯 **Optimized for ShardMap**: Best performance when used with [ShardMap](https://github.com/muxover/shardmap)
- 🔗 **Standalone**: Works with any map-like structure (HashMap, ShardMap, DashMap, etc.)

## 📦 Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
json-sync = "0.1"
```

Or with the optional `binary` feature for faster serialization:

```toml
[dependencies]
json-sync = { version = "0.1", features = ["binary"] }
```

## 🚀 Quick Start

```rust
use json_sync::JsonSyncBuilder;
use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;
use std::time::Duration;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create a thread-safe map
    let map = Arc::new(RwLock::new(HashMap::new()));

    // Create callbacks for getting and loading data
    let get_data = {
        let map = Arc::clone(&map);
        Arc::new(move || {
            map.read().clone()
        })
    };

    let load_data = {
        let map = Arc::clone(&map);
        Arc::new(move |data: HashMap<String, i32>| {
            *map.write() = data;
        })
    };

    // Create and configure JsonSync
    let sync = JsonSyncBuilder::new()
        .path("data.json")
        .flush_interval(Duration::from_secs(5))
        .batch_size(100)
        .build::<String, i32>()?;

    // Attach the map
    sync.attach(get_data, load_data)?;

    // Insert data
    map.write().insert("key1".to_string(), 1);
    sync.mark_dirty("key1");

    // Flush manually or let automatic flushing handle it
    sync.flush()?;

    Ok(())
}
```

## 📖 Table of Contents

- [Features](#-features)
- [Installation](#-installation)
- [Quick Start](#-quick-start)
- [Examples](#-examples)
- [Configuration](#-configuration)
- [Integration with ShardMap](#integration-with-shardmap)
- [Performance](#-performance)
- [API Reference](#-api-reference)
- [Design Decisions](#-design-decisions)
- [Contributing](#-contributing)
- [License](#-license)

## 💡 Examples

### Basic Usage

```rust
use json_sync::{JsonSyncBuilder, DirtyStrategy};
use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;
use std::time::Duration;

let map = Arc::new(RwLock::new(HashMap::new()));

let get_data = {
    let map = Arc::clone(&map);
    Arc::new(move || {
        map.read().clone()
    })
};

let load_data = {
    let map = Arc::clone(&map);
    Arc::new(move |data: HashMap<String, i32>| {
        *map.write() = data;
    })
};

let sync = JsonSyncBuilder::new()
    .path("data.json")
    .flush_interval(Duration::from_secs(10))
    .dirty_strategy(DirtyStrategy::PerKey)
    .build::<String, i32>()?;

sync.attach(get_data, load_data)?;
sync.start()?;

// Mark entries as dirty when they change
sync.mark_dirty("key1");
```

### High-Throughput Workload

```rust
use json_sync::{JsonSyncBuilder, DirtyStrategy};

// Configure for high-throughput workloads
let sync = JsonSyncBuilder::new()
    .path("high_throughput_data.json")
    .flush_threshold(10000)  // Flush after 10K changes
    .batch_size(1000)
    .dirty_strategy(DirtyStrategy::PerShard)
    .shard_count(64)
    .build::<String, i32>()?;

// Handle high-frequency updates efficiently
```

### Recovery

```rust
use json_sync::{load_from_file, SerializationFormat};
use std::collections::HashMap;

// Load data from file
let data: HashMap<String, i32> = load_from_file(
    "data.json",
    SerializationFormat::Json
)?;

// Or use JsonSync's built-in recovery
sync.load_from_file()?;
```

## ⚙️ Configuration

### Flush Strategies

**Time-Based Flushing:**
```rust
let sync = JsonSyncBuilder::new()
    .path("data.json")
    .flush_interval(Duration::from_secs(5))  // Flush every 5 seconds
    .build::<String, i32>()?;
```

**Count-Based Flushing:**
```rust
let sync = JsonSyncBuilder::new()
    .path("data.json")
    .flush_threshold(1000)  // Flush after 1000 changes
    .build::<String, i32>()?;
```

**Manual Flushing:**
```rust
let sync = JsonSyncBuilder::new()
    .path("data.json")
    .manual_flush()  // Only flush on explicit calls
    .build::<String, i32>()?;
```

### Dirty Tracking Strategies

**Per-Key Tracking** (more granular, better for sparse updates):
```rust
let sync = JsonSyncBuilder::new()
    .path("data.json")
    .dirty_strategy(DirtyStrategy::PerKey)
    .build::<String, i32>()?;
```

**Per-Shard Tracking** (faster, better for dense updates):
```rust
let sync = JsonSyncBuilder::new()
    .path("data.json")
    .dirty_strategy(DirtyStrategy::PerShard)
    .shard_count(16)
    .build::<String, i32>()?;
```

### Serialization Formats

**JSON Format** (default, human-readable):
```rust
let sync = JsonSyncBuilder::new()
    .path("data.json")
    .serialization_format(SerializationFormat::Json)
    .build::<String, i32>()?;
```

**Binary Format** (faster, requires `binary` feature):
```toml
[dependencies]
json-sync = { version = "0.1", features = ["binary"] }
```

```rust
let sync = JsonSyncBuilder::new()
    .path("data.bin")
    .use_binary_format(true)
    .build::<String, i32>()?;
```

## 🔗 Integration with ShardMap

**JsonSync is optimized for and works best with [ShardMap](https://github.com/muxover/shardmap)** - a high-performance concurrent sharded map designed for extreme workloads. Together, they provide a complete solution for managing millions of key-value entries with atomic updates, high-concurrency reads/writes, and reliable JSON persistence.

### Why ShardMap + JsonSync?

- **Perfect Match**: ShardMap's per-shard design aligns perfectly with JsonSync's per-shard dirty tracking
- **High Performance**: Both libraries are designed for extreme workloads and millions of operations per second
- **Production Ready**: Battle-tested combination for high-throughput applications
- **Observability**: ShardMap's per-shard statistics combined with JsonSync's flush statistics provide complete visibility

JsonSync works seamlessly with ShardMap:

```rust
use json_sync::{JsonSyncBuilder, DirtyStrategy};
use shardmap::ShardMap;
use std::sync::Arc;

// Create a ShardMap instance
// See https://github.com/muxover/shardmap for more details
let map = Arc::new(ShardMap::new());

// Create callbacks that work with ShardMap
let get_data = {
    let map = Arc::clone(&map);
    Arc::new(move || {
        let mut data = std::collections::HashMap::new();
        for (key, value) in map.iter_snapshot() {
            // Convert ShardMap entries to HashMap
            data.insert(key.clone(), serde_json::Value::String((*value).clone()));
        }
        data
    })
};

// Configure JsonSync to work optimally with ShardMap
// Use PerShard dirty tracking to match ShardMap's architecture
let sync = JsonSyncBuilder::new()
    .path("shardmap_data.json")
    .dirty_strategy(DirtyStrategy::PerShard)
    .shard_count(16)  // Match ShardMap's shard count
    .build::<String, serde_json::Value>()?;

sync.attach(get_data, /* load_data */)?;

// Mark shards as dirty when they change
// This aligns perfectly with ShardMap's per-shard operations
sync.mark_shard_dirty(0);
```

**Learn more about ShardMap**: Visit [https://github.com/muxover/shardmap](https://github.com/muxover/shardmap) to see how ShardMap provides high-performance concurrent map operations with per-shard statistics and observability.

## ⚡ Performance

### Performance Characteristics

- **Minimal Overhead**: Dirty tracking adds minimal overhead to map operations
- **Non-Blocking**: Flush operations run asynchronously and don't block map access
- **Efficient Serialization**: Binary format provides 2-3x faster serialization than JSON
- **Scalable**: Handles millions of entries and high-frequency updates

### Expected Performance

| Operation | Performance |
|-----------|-------------|
| **Mark Dirty** | < 1µs (per-key), < 0.5µs (per-shard) |
| **Flush (10K entries)** | ~10-50ms (JSON), ~5-20ms (binary) |
| **Load from File** | ~20-100ms (10K entries) |
| **Concurrent Flush** | Thread-safe, no blocking |

### Tuning Recommendations

1. **For High-Throughput Workloads**: Use `PerShard` tracking with count-based flushing (especially with [ShardMap](https://github.com/muxover/shardmap))
2. **For Sparse Updates**: Use `PerKey` tracking for more granular control
3. **For Large Datasets**: Enable binary format for faster serialization
4. **For Real-time Sync**: Use time-based flushing with short intervals
5. **For Batch Processing**: Use count-based flushing with large thresholds

## 📚 API Reference

### Main Types

- `JsonSync<K, V>`: The main synchronization instance
- `JsonSyncBuilder`: Builder for configuring JsonSync
- `DirtyStrategy`: Strategy for tracking dirty entries
- `FlushStrategy`: Strategy for automatic flushing
- `SerializationFormat`: Format for serialization
- `JsonSyncStats`: Statistics about sync operations

### Key Methods

| Method | Description |
|--------|-------------|
| `JsonSyncBuilder::new()` | Create a new builder |
| `attach(getter, loader)` | Attach callbacks for getting/loading data |
| `mark_dirty(key)` | Mark a key as dirty |
| `mark_shard_dirty(idx)` | Mark a shard as dirty |
| `flush()` | Flush data to disk immediately |
| `flush_async()` | Trigger asynchronous flush |
| `start()` | Start automatic flushing |
| `stop()` | Stop automatic flushing |
| `load_from_file()` | Load data from file |
| `stats()` | Get synchronization statistics |

For detailed API documentation, see [docs.rs/json-sync](https://docs.rs/json-sync).

## 🏗️ Design Decisions

### Why Callback-Based Architecture?

JsonSync uses callbacks instead of traits to remain truly standalone and work with any map-like structure. This design:

- **Flexibility**: Works with HashMap, ShardMap, DashMap, or any custom map
- **No Dependencies**: Doesn't require map types to implement specific traits
- **Simple Integration**: Easy to integrate with existing codebases

### Why Multiple Dirty Tracking Strategies?

Different workloads benefit from different tracking strategies:

- **Per-Key**: Better for sparse updates, more memory overhead
- **Per-Shard**: Better for dense updates, less memory overhead

### Why Optional Binary Format?

Binary format provides significant performance improvements (2-3x faster) but:

- Requires the `binary` feature
- Files are not human-readable
- Can still convert to JSON for external use

## 🎯 Use Cases

JsonSync is ideal for:

- 🏦 **Session Stores**: Persist user sessions with automatic flushing
- 🌐 **Web Servers**: Cache synchronization with recovery
- 📈 **Real-time Analytics**: High-frequency data updates with persistence
- 🗄️ **State Management**: Application state with reliable persistence
- 🔍 **Monitoring Tools**: Track metrics with automatic synchronization
- ⚡ **High-Throughput Systems**: Handle high-frequency updates with minimal overhead (especially with [ShardMap](https://github.com/muxover/shardmap))

## 🚫 Non-Goals

JsonSync is designed to be a focused synchronization library. The following are explicitly **not** goals:

- ❌ **Database Features**: No transactions, queries, or complex operations
- ❌ **Network Sync**: No built-in network synchronization (use with network libraries)
- ❌ **Conflict Resolution**: No automatic conflict resolution (application-level)
- ❌ **Compression**: No built-in compression (use file system compression)

## 🤝 Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

### Development Setup

```bash
# Clone the repository
git clone https://github.com/muxover/json-sync.git
cd json-sync

# Run tests
cargo test

# Run benchmarks
cargo bench

# Check documentation
cargo doc --open
```

### Code Style

- Follow Rust standard formatting (`cargo fmt`)
- Run clippy (`cargo clippy`)
- Ensure all tests pass
- Update documentation for API changes

## 📄 License

Licensed under the Apache License, Version 2.0 ([LICENSE](LICENSE) or http://www.apache.org/licenses/LICENSE-2.0)

## 🔗 Links

- **Crates.io**: https://crates.io/crates/json-sync
- **Documentation**: https://docs.rs/json-sync
- **Repository**: https://github.com/muxover/json-sync
- **Issues**: https://github.com/muxover/json-sync/issues

---

<div align="center">

Made with ❤️ by Jax (@muxover)

[⭐ Star us on GitHub](https://github.com/muxover/json-sync) if you find this project useful!

</div>

