//! Pluggable map backends.
//!
//! Implement [`MapBackend`] to bring your own concurrent map.

use serde::de::DeserializeOwned;
use serde::Serialize;
use std::hash::Hash;

/// Trait that a concurrent map must satisfy to back a [`JsonSync`](crate::JsonSync) store.
///
/// Every method works with owned values so the public API stays uniform
/// regardless of how the backend stores things internally. Backends that keep
/// values behind an `Arc` (like ShardMap) will clone on read — cheap for small
/// values, worth knowing about for large ones.
pub trait MapBackend<K, V>: Send + Sync
where
    K: Hash + Eq + Send + Sync + Clone + Serialize + DeserializeOwned,
    V: Send + Sync + Clone + Serialize + DeserializeOwned,
{
    /// Insert a key-value pair, returning the previous value if any.
    fn insert(&self, key: K, value: V) -> Option<V>;

    /// Look up a value by key.
    fn get(&self, key: &K) -> Option<V>;

    /// Remove a key, returning its value if it was present.
    fn remove(&self, key: &K) -> Option<V>;

    /// Consistent snapshot of all entries. The returned iterator must not hold
    /// locks that would block concurrent writers.
    fn iter_snapshot(&self) -> Box<dyn Iterator<Item = (K, V)> + Send + '_>;

    /// Number of entries. Override this — the default returns 0.
    fn map_len(&self) -> usize {
        0
    }

    /// Check if a key exists without cloning the value. Override for backends
    /// that can do this cheaply (most can).
    fn contains_key(&self, key: &K) -> bool {
        self.get(key).is_some()
    }

    /// Drop all entries. The default does iter + remove which is slow; override
    /// with the backend's native clear when available.
    fn clear(&self) {
        let keys: Vec<K> = self.iter_snapshot().map(|(k, _)| k).collect();
        for k in &keys {
            self.remove(k);
        }
    }
}

// ---- ShardMap ----------------------------------------------------------------

impl<K, V> MapBackend<K, V> for shardmap::ShardMap<K, V>
where
    K: Hash + Eq + Send + Sync + Clone + Serialize + DeserializeOwned,
    V: Send + Sync + Clone + Serialize + DeserializeOwned,
{
    fn insert(&self, key: K, value: V) -> Option<V> {
        self.insert(key, value).map(|arc| (*arc).clone())
    }

    fn get(&self, key: &K) -> Option<V> {
        self.get(key).map(|arc| (*arc).clone())
    }

    fn remove(&self, key: &K) -> Option<V> {
        self.remove(key).map(|arc| (*arc).clone())
    }

    fn iter_snapshot(&self) -> Box<dyn Iterator<Item = (K, V)> + Send + '_> {
        Box::new(self.iter_snapshot().map(|(k, arc_v)| (k, (*arc_v).clone())))
    }

    fn map_len(&self) -> usize {
        self.len()
    }

    // ShardMap::get returns Arc<V>, so is_some() is just an atomic refcount bump.
    fn contains_key(&self, key: &K) -> bool {
        shardmap::ShardMap::get(self, key).is_some()
    }
}

// ---- RwLock<HashMap> ---------------------------------------------------------

impl<K, V> MapBackend<K, V> for parking_lot::RwLock<std::collections::HashMap<K, V>>
where
    K: Hash + Eq + Send + Sync + Clone + Serialize + DeserializeOwned,
    V: Send + Sync + Clone + Serialize + DeserializeOwned,
{
    fn insert(&self, key: K, value: V) -> Option<V> {
        self.write().insert(key, value)
    }

    fn get(&self, key: &K) -> Option<V> {
        self.read().get(key).cloned()
    }

    fn remove(&self, key: &K) -> Option<V> {
        self.write().remove(key)
    }

    fn iter_snapshot(&self) -> Box<dyn Iterator<Item = (K, V)> + Send + '_> {
        let snap: Vec<_> = self
            .read()
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        Box::new(snap.into_iter())
    }

    fn map_len(&self) -> usize {
        self.read().len()
    }

    fn contains_key(&self, key: &K) -> bool {
        self.read().contains_key(key)
    }

    fn clear(&self) {
        self.write().clear()
    }
}

// ---- DashMap (feature-gated) -------------------------------------------------

#[cfg(feature = "dashmap")]
impl<K, V> MapBackend<K, V> for dashmap::DashMap<K, V>
where
    K: Hash + Eq + Send + Sync + Clone + Serialize + DeserializeOwned,
    V: Send + Sync + Clone + Serialize + DeserializeOwned,
{
    fn insert(&self, key: K, value: V) -> Option<V> {
        self.insert(key, value)
    }

    fn get(&self, key: &K) -> Option<V> {
        self.get(key).map(|r| r.value().clone())
    }

    fn remove(&self, key: &K) -> Option<V> {
        self.remove(key).map(|(_, v)| v)
    }

    fn iter_snapshot(&self) -> Box<dyn Iterator<Item = (K, V)> + Send + '_> {
        let snap: Vec<_> = self
            .iter()
            .map(|r| (r.key().clone(), r.value().clone()))
            .collect();
        Box::new(snap.into_iter())
    }

    fn map_len(&self) -> usize {
        self.len()
    }

    fn contains_key(&self, key: &K) -> bool {
        dashmap::DashMap::contains_key(self, key)
    }

    fn clear(&self) {
        dashmap::DashMap::clear(self)
    }
}
