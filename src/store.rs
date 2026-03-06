//! Core store type, handle, and builder.

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
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Persistent JSON-backed key-value store.
///
/// Generic over key `K`, value `V`, and map backend `M`. Use [`open`](Self::open)
/// for a quick start or [`builder`](Self::builder) for full control over flush
/// policy, pretty-printing, etc.
///
/// All operations are thread-safe — the concurrency guarantees come from
/// whichever backend you pick.
pub struct JsonSync<K, V, M> {
    pub(crate) map: Arc<M>,
    pub(crate) path: PathBuf,
    pub(crate) serializer: JsonSerializer,
    pub(crate) policy: FlushPolicy,
    pub(crate) trigger: Option<Arc<std::sync::mpsc::SyncSender<()>>>,
    pub(crate) _marker: PhantomData<(K, V)>,
}

impl<K, V, M> JsonSync<K, V, M>
where
    K: Hash + Eq + Send + Sync + Clone + Serialize + DeserializeOwned + 'static,
    V: Send + Sync + Clone + Serialize + DeserializeOwned + 'static,
    M: MapBackend<K, V> + 'static,
{
    /// Open (or create) a store at `path` with manual flush and compact JSON.
    pub fn open(path: impl AsRef<Path>) -> Result<JsonSyncHandle<K, V, M>>
    where
        M: Default,
    {
        Self::builder(path).build()
    }

    /// Open with a specific flush policy. Shorthand for
    /// `builder(path).policy(p).build()`.
    pub fn open_with_policy(
        path: impl AsRef<Path>,
        policy: FlushPolicy,
    ) -> Result<JsonSyncHandle<K, V, M>>
    where
        M: Default,
    {
        Self::builder(path).policy(policy).build()
    }

    /// Start configuring a new store. Call [`.build()`](JsonSyncBuilder::build)
    /// when ready.
    pub fn builder(path: impl AsRef<Path>) -> JsonSyncBuilder<K, V, M>
    where
        M: Default,
    {
        JsonSyncBuilder::new(path)
    }

    // ---- reads ----

    /// Get the value for `key`, or `None` if absent.
    #[must_use]
    pub fn get(&self, key: &K) -> Option<V> {
        self.map.get(key)
    }

    /// `true` if the key exists. Avoids cloning the value when the backend
    /// supports it.
    #[must_use]
    pub fn contains_key(&self, key: &K) -> bool {
        self.map.contains_key(key)
    }

    /// Number of entries.
    #[must_use]
    pub fn len(&self) -> usize {
        self.map.map_len()
    }

    /// `true` when the store has no entries.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Snapshot of all key-value pairs.
    #[must_use]
    pub fn iter(&self) -> Vec<(K, V)> {
        self.map.iter_snapshot().collect()
    }

    /// Snapshot of all keys.
    #[must_use]
    pub fn keys(&self) -> Vec<K> {
        self.map.iter_snapshot().map(|(k, _)| k).collect()
    }

    /// Snapshot of all values.
    #[must_use]
    pub fn values(&self) -> Vec<V> {
        self.map.iter_snapshot().map(|(_, v)| v).collect()
    }

    /// Path to the backing JSON file.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    // ---- writes ----

    /// Insert a key-value pair, returning the previous value if the key existed.
    pub fn insert(&self, key: K, value: V) -> Result<Option<V>> {
        let prev = self.map.insert(key, value);
        self.notify_mutation()?;
        Ok(prev)
    }

    /// Remove a key, returning its value if it was present.
    pub fn remove(&self, key: &K) -> Result<Option<V>> {
        let prev = self.map.remove(key);
        self.notify_mutation()?;
        Ok(prev)
    }

    /// Drop all entries from the store.
    pub fn clear(&self) -> Result<()> {
        self.map.clear();
        self.notify_mutation()
    }

    /// Bulk-insert from an iterator. Only triggers one flush at the end, not
    /// one per entry.
    pub fn extend<I>(&self, iter: I) -> Result<()>
    where
        I: IntoIterator<Item = (K, V)>,
    {
        for (k, v) in iter {
            self.map.insert(k, v);
        }
        self.notify_mutation()
    }

    /// Mutate the value at `key` in place. Returns `false` if the key doesn't
    /// exist (nothing happens in that case).
    ///
    /// Heads up: this does a get-then-put under the hood, so there's a small
    /// race window with concurrent writers. Fine for single-writer setups.
    pub fn update<F>(&self, key: &K, f: F) -> Result<bool>
    where
        F: FnOnce(&mut V),
    {
        match self.map.get(key) {
            Some(mut v) => {
                f(&mut v);
                self.map.insert(key.clone(), v);
                self.notify_mutation()?;
                Ok(true)
            }
            None => Ok(false),
        }
    }

    /// Return the existing value for `key`, or insert `default` and return it.
    pub fn get_or_insert(&self, key: K, default: V) -> Result<V> {
        if let Some(v) = self.map.get(&key) {
            return Ok(v);
        }
        let ret = default.clone();
        self.map.insert(key, default);
        self.notify_mutation()?;
        Ok(ret)
    }

    /// Like [`get_or_insert`](Self::get_or_insert) but only computes the
    /// default when the key is actually missing.
    pub fn get_or_insert_with<F>(&self, key: K, f: F) -> Result<V>
    where
        F: FnOnce() -> V,
    {
        if let Some(v) = self.map.get(&key) {
            return Ok(v);
        }
        let val = f();
        let ret = val.clone();
        self.map.insert(key, val);
        self.notify_mutation()?;
        Ok(ret)
    }

    // ---- persistence ----

    /// Write the current map contents to disk (atomic temp-file + rename).
    pub fn flush(&self) -> Result<()> {
        do_flush(self.map.as_ref(), &self.path, &self.serializer)
    }

    // ---- internal ----

    fn notify_mutation(&self) -> Result<()> {
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
        Ok(())
    }
}

impl<K, V, M> std::fmt::Debug for JsonSync<K, V, M> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("JsonSync")
            .field("path", &self.path)
            .field("policy", &self.policy)
            .finish_non_exhaustive()
    }
}

fn do_flush<K, V, M>(map: &M, path: &Path, serializer: &JsonSerializer) -> Result<()>
where
    K: Hash + Eq + Send + Sync + Clone + Serialize + DeserializeOwned,
    V: Send + Sync + Clone + Serialize + DeserializeOwned,
    M: MapBackend<K, V>,
{
    let mut data = HashMap::with_capacity(map.map_len());
    for (k, v) in map.iter_snapshot() {
        data.insert(k, v);
    }
    let bytes = serializer.serialize(&data)?;
    atomic_write(path, &bytes)
}

// ---------------------------------------------------------------------------
// Builder
// ---------------------------------------------------------------------------

/// Configures and opens a [`JsonSync`] store.
///
/// ```rust,no_run
/// use json_sync::JsonSync;
/// use shardmap::ShardMap;
///
/// let db = JsonSync::<String, i32, ShardMap<String, i32>>::builder("db.json")
///     .pretty(true)
///     .build()
///     .unwrap();
/// ```
pub struct JsonSyncBuilder<K, V, M> {
    path: PathBuf,
    policy: FlushPolicy,
    pretty: bool,
    _marker: PhantomData<(K, V, M)>,
}

impl<K, V, M> JsonSyncBuilder<K, V, M>
where
    K: Hash + Eq + Send + Sync + Clone + Serialize + DeserializeOwned + 'static,
    V: Send + Sync + Clone + Serialize + DeserializeOwned + 'static,
    M: MapBackend<K, V> + Default + 'static,
{
    fn new(path: impl AsRef<Path>) -> Self {
        Self {
            path: path.as_ref().to_path_buf(),
            policy: FlushPolicy::Manual,
            pretty: false,
            _marker: PhantomData,
        }
    }

    /// Set the flush policy (default: [`FlushPolicy::Manual`]).
    pub fn policy(mut self, policy: FlushPolicy) -> Self {
        self.policy = policy;
        self
    }

    /// Write human-readable JSON with indentation (default: compact).
    pub fn pretty(mut self, yes: bool) -> Self {
        self.pretty = yes;
        self
    }

    /// Load (or create) the store and return a handle.
    pub fn build(self) -> Result<JsonSyncHandle<K, V, M>> {
        let serializer = if self.pretty {
            JsonSerializer::pretty()
        } else {
            JsonSerializer::new()
        };

        let map = Arc::new(M::default());

        let data = load::<K, V, _>(&self.path, &serializer)?;
        for (k, v) in data {
            map.insert(k, v);
        }

        let (worker, trigger) = match &self.policy {
            FlushPolicy::Async(interval) => {
                let (tx, rx) = std::sync::mpsc::sync_channel(0);
                let map_ref = Arc::clone(&map);
                let path = self.path.clone();
                let ser = serializer.clone();
                let interval = *interval;
                let w = AsyncFlushWorker::start_with_receiver(
                    interval,
                    move || {
                        let _ = do_flush(map_ref.as_ref(), &path, &ser);
                    },
                    rx,
                );
                (Some(w), Some(Arc::new(tx)))
            }
            _ => (None, None),
        };

        let store = JsonSync {
            map,
            path: self.path,
            serializer,
            policy: self.policy,
            trigger,
            _marker: PhantomData,
        };

        Ok(JsonSyncHandle {
            inner: Arc::new(store),
            worker,
        })
    }
}

impl<K, V, M> std::fmt::Debug for JsonSyncBuilder<K, V, M> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("JsonSyncBuilder")
            .field("path", &self.path)
            .field("policy", &self.policy)
            .field("pretty", &self.pretty)
            .finish()
    }
}

// ---------------------------------------------------------------------------
// Handle
// ---------------------------------------------------------------------------

/// Owns the store and (for async policy) the background flush thread.
///
/// Derefs to [`JsonSync`] so you can call store methods directly on it.
/// Dropping this will join the background thread if one is running, which may
/// block for up to one flush interval.
pub struct JsonSyncHandle<K, V, M> {
    pub(crate) inner: Arc<JsonSync<K, V, M>>,
    #[allow(dead_code)]
    pub(crate) worker: Option<AsyncFlushWorker>,
}

impl<K, V, M> std::ops::Deref for JsonSyncHandle<K, V, M> {
    type Target = JsonSync<K, V, M>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<K, V, M> std::fmt::Debug for JsonSyncHandle<K, V, M> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(&*self.inner, f)
    }
}
