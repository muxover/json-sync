//! Serialization for persisting the map to disk.
//!
//! **Serializer** — Trait with `serialize(HashMap) -> Vec<u8>` and `deserialize(&[u8]) -> HashMap`.
//! Default implementation is **JsonSerializer** (serde_json). Custom serializers can implement
//! the trait for alternative formats (e.g. ron, simd_json) as a future extension.

use crate::error::{Error, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Trait for serializing and deserializing map data to/from bytes.
pub trait Serializer: Send + Sync {
    /// Serialize a map to bytes.
    fn serialize<K, V>(&self, data: &HashMap<K, V>) -> Result<Vec<u8>>
    where
        K: Serialize,
        V: Serialize;

    /// Deserialize bytes into a map.
    fn deserialize<K, V>(&self, bytes: &[u8]) -> Result<HashMap<K, V>>
    where
        K: for<'de> Deserialize<'de> + Eq + std::hash::Hash,
        V: for<'de> Deserialize<'de>;
}

/// Default JSON serializer using serde_json.
#[derive(Clone, Default)]
pub struct JsonSerializer;

impl JsonSerializer {
    /// Create a new JSON serializer.
    pub fn new() -> Self {
        Self
    }
}

impl Serializer for JsonSerializer {
    fn serialize<K, V>(&self, data: &HashMap<K, V>) -> Result<Vec<u8>>
    where
        K: Serialize,
        V: Serialize,
    {
        serde_json::to_vec(data).map_err(Error::from)
    }

    fn deserialize<K, V>(&self, bytes: &[u8]) -> Result<HashMap<K, V>>
    where
        K: for<'de> Deserialize<'de> + Eq + std::hash::Hash,
        V: for<'de> Deserialize<'de>,
    {
        serde_json::from_slice(bytes).map_err(Error::from)
    }
}
