//! Core store: `JsonSync` and `JsonSyncHandle`.
//!
//! When the flush policy is `Async`, the handle owns the background worker.
//! Dropping the handle joins the worker (may block briefly).

use crate::backend::MapBackend;
use crate::error::Result;
use crate::flush::{AsyncFlushWorker, FlushPolicy};
use crate::persist::{atomic_write, load};
use crate::serializer::{JsonSerializer, Serializer};
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::collections::HashMap;
use std::hash::Hash;
use std::marker::PhantomData;
use std::path::Path;
use std::sync::Arc;

/// Persistent JSON-backed key-value store with a pluggable map backend.
///
/// Uses [ShardMap](https://docs.rs/shardmap) by default for concurrent access;
/// you can use `RwLock<HashMap>` or (with the `dashmap` feature) `DashMap` instead.
///
/// **Flush policy:** `Immediate` = flush after every insert/remove; `Async(d)` = background
/// thread flushes every `d`; `Manual` = only when you call `flush()`.
///
/// **Note:** `get` and `iter()` may clone `V` for backends that store `Arc<V>` (e.g. ShardMap).
/// For very large values this can be costly; consider manual flush batching or smaller values.
pub struct JsonSync<K, V, M> {
    pub(crate) map: Arc<M>,
    pub(crate) path: std::path::PathBuf,
    pub(crate) serializer: JsonSerializer,
    pub(crate) policy: FlushPolicy,
    /// When Async, trigger channel to request a flush from the background worker.
    pub(crate) trigger: Option<Arc<std::sync::mpsc::SyncSender<()>>>,
    pub(crate) _marker: PhantomData<(K, V)>,
}

impl<K, V, M> JsonSync<K, V, M>
where
    K: Hash + Eq + Send + Sync + Clone + Serialize + DeserializeOwned + 'static,
    V: Send + Sync + Clone + Serialize + DeserializeOwned + 'static,
    M: MapBackend<K, V> + 'static,
{
    /// Open or create the database at `path`. If the file exists, loads it into the map.
    /// Uses `Manual` flush policy by default. Requires `M: Default`.
    pub fn open(path: impl AsRef<Path>) -> Result<JsonSyncHandle<K, V, M>>
    where
        M: Default,
    {
        Self::open_with_policy(path, FlushPolicy::Manual)
    }

    /// Open or create the database with a custom flush policy.
    pub fn open_with_policy(
        path: impl AsRef<Path>,
        policy: FlushPolicy,
    ) -> Result<JsonSyncHandle<K, V, M>>
    where
        M: Default,
    {
        let path = path.as_ref().to_path_buf();
        let map: M = M::default();
        let map = Arc::new(map);
        let serializer = JsonSerializer::new();

        let data = load::<K, V, _>(&path, &serializer)?;
        for (k, v) in data {
            map.insert(k, v);
        }

        let (worker, trigger) = match &policy {
            FlushPolicy::Async(interval) => {
                let (tx, rx) = std::sync::mpsc::sync_channel(0);
                let sync_for_worker: JsonSync<K, V, M> = JsonSync {
                    map: Arc::clone(&map),
                    path: path.clone(),
                    serializer: JsonSerializer::new(),
                    policy: policy.clone(),
                    trigger: None,
                    _marker: PhantomData,
                };
                let arc = Arc::new(sync_for_worker);
                let inner = Arc::clone(&arc);
                let interval = *interval;
                let w = AsyncFlushWorker::start_with_receiver(
                    interval,
                    move || {
                        let _ = inner.flush();
                    },
                    rx,
                );
                (Some(w), Some(Arc::new(tx)))
            }
            _ => (None, None),
        };

        let sync = JsonSync {
            map,
            path,
            serializer,
            policy,
            trigger,
            _marker: PhantomData,
        };

        Ok(JsonSyncHandle {
            inner: Arc::new(sync),
            worker,
        })
    }

    /// Insert a key-value pair. Returns the previous value if present.
    /// On `Immediate` policy this flushes to disk; on `Async` it notifies the background flusher.
    pub fn insert(&self, key: K, value: V) -> Result<Option<V>> {
        let prev = self.map.insert(key, value);
        self.after_mut(prev)
    }

    /// Get the value for a key. May clone `V` for backends that store `Arc<V>`.
    pub fn get(&self, key: &K) -> Option<V> {
        self.map.get(key)
    }

    /// Remove the key and return its value if present.
    pub fn remove(&self, key: &K) -> Result<Option<V>> {
        let prev = self.map.remove(key);
        self.after_mut(prev)
    }

    /// Flush the current map state to disk (atomic write).
    pub fn flush(&self) -> Result<()> {
        do_flush(self.map.as_ref(), &self.path, &self.serializer)
    }

    /// Return a snapshot of all entries. Consistent view; may clone values.
    pub fn iter(&self) -> Vec<(K, V)> {
        self.map.iter_snapshot().collect()
    }

    /// Path to the backing JSON file.
    pub fn path(&self) -> &std::path::Path {
        &self.path
    }

    fn after_mut(&self, prev: Option<V>) -> Result<Option<V>> {
        match &self.policy {
            FlushPolicy::Immediate => {
                do_flush(self.map.as_ref(), &self.path, &self.serializer)?;
            }
            FlushPolicy::Async(_) => {
                if let Some(t) = &self.trigger {
                    let _ = t.try_send(());
                }
            }
            FlushPolicy::Manual => {}
        }
        Ok(prev)
    }
}

fn do_flush<K, V, M>(
    map: &M,
    path: &std::path::Path,
    serializer: &JsonSerializer,
) -> Result<()>
where
    K: Hash + Eq + Send + Sync + Clone + Serialize + DeserializeOwned,
    V: Send + Sync + Clone + Serialize + DeserializeOwned,
    M: MapBackend<K, V>,
{
    let data: HashMap<K, V> = map.iter_snapshot().collect();
    let bytes = serializer.serialize(&data)?;
    atomic_write(path, &bytes)
}

/// Handle that owns the store and (when using `Async` policy) the background flush worker.
/// Derefs to `JsonSync`. Dropping may block briefly while the worker exits.
/// To share across threads, wrap in `Arc`: `Arc::new(handle)` or clone an `Arc<JsonSyncHandle<...>>`.
/// Field order: inner dropped first so that the trigger sender is dropped and the worker thread sees Disconnected before we join.
pub struct JsonSyncHandle<K, V, M> {
    pub(crate) inner: Arc<JsonSync<K, V, M>>,
    /// Held for its `Drop` impl (joins the thread); never read otherwise.
    #[allow(dead_code)]
    pub(crate) worker: Option<AsyncFlushWorker>,
}

impl<K, V, M> std::ops::Deref for JsonSyncHandle<K, V, M> {
    type Target = JsonSync<K, V, M>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}
