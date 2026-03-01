//! Persistent JSON-backed key-value store with pluggable backends.
//!
//! Pick a map (ShardMap, `RwLock<HashMap>`, DashMap), a flush policy
//! (immediate / async / manual), and you're good to go.
//!
//! ```rust,no_run
//! use json_sync::JsonSync;
//! use shardmap::ShardMap;
//!
//! let db = JsonSync::<String, String, ShardMap<String, String>>::open("db.json").unwrap();
//! db.insert("hello".into(), "world".into()).unwrap();
//! db.flush().unwrap();
//! ```
//!
//! **Single-process only.** If multiple processes open the same file they will
//! clobber each other. Use advisory file locking or a real database for
//! multi-process access.

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
pub use store::{JsonSync, JsonSyncBuilder, JsonSyncHandle};

/// Default backend: ShardMap.
pub type DefaultBackend<K, V> = shardmap::ShardMap<K, V>;
