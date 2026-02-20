//! # json-sync
//!
//! A general-purpose, persistent JSON-backed key-value store with pluggable map backends,
//! configurable flush policy, and thread-safe concurrent access.
//!
//! ## Design
//!
//! - **Pluggable backends** — Use [ShardMap](https://docs.rs/shardmap) (default), `RwLock<HashMap>`, or
//!   (with the `dashmap` feature) [DashMap](https://docs.rs/dashmap). The map lives inside the store;
//!   all reads and writes go through it.
//! - **Flush policy** — `Immediate` (flush after every insert/remove), `Async(d)` (background thread
//!   flushes on an interval and on mutation), or `Manual` (only when you call `flush()`).
//! - **Atomic writes** — Persistence uses write-to-temp-then-rename for crash safety.
//! - **Unified errors** — `insert`, `remove`, and `flush` all return `Result<_, Error>` (IO, serialize, deserialize, config).
//!
//! ## When to use json-sync
//!
//! - You need a **simple persistent key-value store** backed by a JSON file.
//! - You want **concurrent access** with a choice of map (ShardMap, DashMap, or `RwLock<HashMap>`).
//! - You want **configurable flush** — immediate, batched async, or manual.
//!
//! ## Snapshot cloning
//!
//! With backends that store `Arc<V>` (e.g. ShardMap), `get` and `iter()` clone out of the Arc.
//! For small or medium values this is fine; for very large `V`, the clone cost can be non-trivial.
//! Consider manual flush batching or smaller value types if this matters.
//!
//! ## Quick example
//!
//! ```rust,no_run
//! use json_sync::JsonSync;
//! use shardmap::ShardMap;
//!
//! fn main() -> Result<(), json_sync::Error> {
//!     let db = JsonSync::<String, String, ShardMap<String, String>>::open("db.json")?;
//!     db.insert("key1".to_string(), "value1".to_string())?;
//!     let _val = db.get(&"key1".to_string());
//!     db.remove(&"key1".to_string())?;
//!     db.flush()?;
//!     Ok(())
//! }
//! ```
//!
//! ## Features
//!
//! | Feature   | Default | Description |
//! |-----------|---------|-------------|
//! | (none)    | ✓       | ShardMap + RwLock\<HashMap\> backends, serde_json. |
//! | `dashmap` | —       | Implement `MapBackend` for `DashMap<K, V>`. |
//!
//! ## Configuration
//!
//! Use [`open`](JsonSync::open) for default (manual) flush, or [`open_with_policy`](JsonSync::open_with_policy)
//! with [`FlushPolicy::Immediate`] or [`FlushPolicy::Async`](FlushPolicy::Async) for different persistence behavior.
//!
//! ## Non-goals
//!
//! No transactions, queries, or secondary indexes; no built-in replication; JSON only (no other formats by default).

#![deny(missing_docs)]
#![warn(clippy::all)]

pub mod backend;
pub mod error;
pub mod flush;
pub mod persist;
pub mod serializer;
pub mod store;

pub use error::{Error, Result};
pub use flush::FlushPolicy;
pub use store::{JsonSync, JsonSyncHandle};

/// Default map backend: ShardMap for concurrent, introspectable storage.
pub type DefaultBackend<K, V> = shardmap::ShardMap<K, V>;
