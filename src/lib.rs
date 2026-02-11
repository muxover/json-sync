//! # JsonSync
//!
//! A high-performance JSON synchronization library for map-like structures.
//!
//! JsonSync provides asynchronous flushing, configurable dirty tracking,
//! and recovery mechanisms for any map-like structure, with special
//! optimizations for ShardMap.
//!
//! ## Features
//!
//! - **Asynchronous Flushing**: Time-based and count-based flush strategies
//! - **Configurable Dirty Tracking**: Per-shard or per-key tracking
//! - **Multiple Serialization Formats**: JSON (default) and optional binary (bincode)
//! - **Recovery Mechanisms**: Load data from files with error handling
//! - **Thread-Safe**: All operations are safe for concurrent access
//! - **Non-Blocking**: Flush operations don't block map operations
//! - **Statistics**: Monitor flush operations and performance
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use json_sync::JsonSyncBuilder;
//! use std::collections::HashMap;
//! use std::sync::Arc;
//! use parking_lot::RwLock;
//!
//! // Create a thread-safe map
//! let map = Arc::new(RwLock::new(HashMap::new()));
//!
//! // Create callbacks for getting and loading data
//! let get_data = {
//!     let map = Arc::clone(&map);
//!     Arc::new(move || {
//!         map.read().clone()
//!     })
//! };
//!
//! let load_data = {
//!     let map = Arc::clone(&map);
//!     Arc::new(move |data: HashMap<String, i32>| {
//!         *map.write() = data;
//!     })
//! };
//!
//! // Create and configure JsonSync
//! let sync = JsonSyncBuilder::new()
//!     .path("data.json")
//!     .flush_interval(std::time::Duration::from_secs(5))
//!     .batch_size(100)
//!     .build::<String, i32>()?;
//!
//! // Attach the map
//! sync.attach(get_data, load_data)?;
//!
//! // Start automatic flushing
//! sync.start()?;
//!
//! // Mark entries as dirty when they change
//! sync.mark_dirty("key1");
//!
//! // Or flush manually
//! sync.flush()?;
//! # Ok::<(), json_sync::JsonSyncError>(())
//! ```
//!
//! ## Integration with ShardMap
//!
//! **JsonSync is optimized for and works best with [ShardMap](https://github.com/muxover/shardmap)** -
//! a high-performance concurrent sharded map. Together, they provide a complete solution
//! for managing millions of key-value entries with atomic updates, high-concurrency
//! reads/writes, and reliable JSON persistence.
//!
//! ```rust,no_run
//! use json_sync::JsonSyncBuilder;
//! use shardmap::ShardMap;
//! use std::sync::Arc;
//!
//! // Create a ShardMap instance
//! // See https://github.com/muxover/shardmap for more details
//! use serde_json::Value;
//! let map = Arc::new(ShardMap::<String, String>::new());
//!
//! let get_data: json_sync::DataGetter<String, Value> = {
//!     let map = Arc::clone(&map);
//!     Arc::new(move || {
//!         let mut data = std::collections::HashMap::new();
//!         for (key, value) in map.iter_snapshot() {
//!             data.insert(key.clone(), Value::String((*value).clone()));
//!         }
//!         data
//!     })
//! };
//!
//! let load_data: json_sync::DataLoader<String, Value> = {
//!     let map = Arc::clone(&map);
//!     Arc::new(move |data: std::collections::HashMap<String, Value>| {
//!         for (key, value) in data {
//!             if let Value::String(s) = value {
//!                 map.insert(key, s);
//!             }
//!         }
//!     })
//! };
//!
//! // Configure JsonSync to work optimally with ShardMap
//! let sync = JsonSyncBuilder::new()
//!     .path("shardmap_data.json")
//!     .dirty_strategy(json_sync::DirtyStrategy::PerShard)
//!     .shard_count(16)
//!     .build::<String, Value>()?;
//!
//! sync.attach(get_data, load_data)?;
//! # Ok::<(), json_sync::JsonSyncError>(())
//! ```

#![deny(missing_docs)]
#![warn(clippy::all)]

/// Builder for configuring JsonSync instances.
pub mod builder;
/// Dirty tracking implementations.
pub mod dirty;
/// Error types.
pub mod error;
/// Flush scheduler and strategies.
pub mod flush;
/// Recovery and file loading.
pub mod recovery;
/// Serialization implementations.
pub mod serializer;
/// Core synchronization implementation.
pub mod sync;
/// Statistics and metrics.
pub mod stats;

// Re-export main types
pub use builder::JsonSyncBuilder;
pub use dirty::DirtyStrategy;
pub use error::JsonSyncError;
pub use flush::FlushStrategy;
pub use recovery::{load_from_file, validate_json_file};
pub use serializer::SerializationFormat;
pub use stats::JsonSyncStats;
pub use sync::{DataGetter, DataLoader, JsonSync};

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::sync::Arc;
    use parking_lot::RwLock;

    #[test]
    fn test_basic_usage() {
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
            .path("test_data.json")
            .manual_flush()
            .build::<String, i32>()
            .unwrap();

        sync.attach(get_data, load_data).unwrap();
        sync.mark_dirty("test_key");
        assert!(sync.flush().is_ok());
    }

    #[test]
    fn test_dirty_tracking() {
        let sync = JsonSyncBuilder::new()
            .path("test.json")
            .manual_flush()
            .dirty_strategy(DirtyStrategy::PerKey)
            .build::<String, i32>()
            .unwrap();

        sync.mark_dirty("key1");
        sync.mark_dirty("key2");
        
        let stats = sync.stats();
        assert_eq!(stats.dirty_count, 2);
    }
}

