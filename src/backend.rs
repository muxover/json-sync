//! Pluggable map backends for JsonSync.
//!
//! Implement `MapBackend<K, V>` to use a custom concurrent map. JsonSync's
//! public API always returns owned `V`; backends that store `Arc<V>` (e.g.
//! ShardMap) clone out — document that `get`/`iter()` may clone for large values.

use serde::de::DeserializeOwned;
use serde::Serialize;
use std::hash::Hash;

/// Trait that any concurrent map backend must implement for use with JsonSync.
///
/// All methods normalize to owned values so that JsonSync's API is uniform
/// regardless of backend. Backends that store `Arc<V>` (e.g. ShardMap) will
/// clone out of the Arc; for very large `V` this can be non-trivial — see
/// crate docs for optional future APIs (e.g. `get_arc`) if you need to avoid cloning.
pub trait MapBackend<K, V>: Send + Sync
where
    K: Hash + Eq + Send + Sync + Clone + Serialize + DeserializeOwned,
    V: Send + Sync + Clone + Serialize + DeserializeOwned,
{
    /// Insert a key-value pair. Returns the previous value if the key was present.
    fn insert(&self, key: K, value: V) -> Option<V>;

    /// Get the value for a key. May clone `V` for backends that store `Arc<V>`.
    fn get(&self, key: &K) -> Option<V>;

    /// Remove the key and return its value if present.
    fn remove(&self, key: &K) -> Option<V>;

    /// Produce a snapshot of all entries. Consistent view; may clone values.
    fn iter_snapshot(&self) -> Box<dyn Iterator<Item = (K, V)> + Send + '_>;

    /// Number of entries. Used to pre-allocate buffers; default 0.
    fn map_len(&self) -> usize {
        0
    }
}

// ----------------------------------------------------------------------------
// ShardMap backend
// ----------------------------------------------------------------------------

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
        Box::new(
            self.iter_snapshot()
                .map(|(k, arc_v)| (k, (*arc_v).clone())),
        )
    }

    // map_len() uses default 0
}

// ----------------------------------------------------------------------------
// RwLock<HashMap> backend (no extra deps beyond parking_lot)
// ----------------------------------------------------------------------------

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
        let guard = self.read();
        let len = guard.len();
        let mut snap = Vec::with_capacity(len);
        for (k, v) in guard.iter() {
            snap.push((k.clone(), v.clone()));
        }
        Box::new(snap.into_iter())
    }

    fn map_len(&self) -> usize {
        self.read().len()
    }
}

// ----------------------------------------------------------------------------
// DashMap backend (optional feature)
// ----------------------------------------------------------------------------

#[cfg_attr(docsrs, doc(cfg(feature = "dashmap")))]
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
        self.get(key).map(|r| (*r).clone())
    }

    fn remove(&self, key: &K) -> Option<V> {
        self.remove(key).map(|(_, v)| v)
    }

    fn iter_snapshot(&self) -> Box<dyn Iterator<Item = (K, V)> + Send + '_> {
        let len = self.len();
        let mut snap = Vec::with_capacity(len);
        for r in self.iter() {
            snap.push((r.key().clone(), r.value().clone()));
        }
        Box::new(snap.into_iter())
    }

    fn map_len(&self) -> usize {
        self.len()
    }
}
