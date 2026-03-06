//! Serialization layer. Defaults to JSON via serde_json.
//!
//! Implement [`Serializer`] if you need a different format (RON, MessagePack, etc.).

use crate::error::{Error, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Converts map snapshots to/from bytes for persistence.
pub trait Serializer: Send + Sync {
    /// Encode a map to bytes.
    fn serialize<K, V>(&self, data: &HashMap<K, V>) -> Result<Vec<u8>>
    where
        K: Serialize,
        V: Serialize;

    /// Decode bytes back into a map.
    fn deserialize<K, V>(&self, bytes: &[u8]) -> Result<HashMap<K, V>>
    where
        K: for<'de> Deserialize<'de> + Eq + std::hash::Hash,
        V: for<'de> Deserialize<'de>;
}

/// JSON serializer with optional pretty-printing.
#[derive(Clone, Default)]
pub struct JsonSerializer {
    pretty: bool,
}

impl JsonSerializer {
    /// Compact JSON (single line, no extra whitespace).
    pub fn new() -> Self {
        Self::default()
    }

    /// Pretty-printed JSON with indentation — easier to read by hand.
    pub fn pretty() -> Self {
        Self { pretty: true }
    }
}

impl Serializer for JsonSerializer {
    fn serialize<K, V>(&self, data: &HashMap<K, V>) -> Result<Vec<u8>>
    where
        K: Serialize,
        V: Serialize,
    {
        let bytes = if self.pretty {
            serde_json::to_vec_pretty(data)
        } else {
            serde_json::to_vec(data)
        };
        bytes.map_err(Error::from)
    }

    fn deserialize<K, V>(&self, bytes: &[u8]) -> Result<HashMap<K, V>>
    where
        K: for<'de> Deserialize<'de> + Eq + std::hash::Hash,
        V: for<'de> Deserialize<'de>,
    {
        serde_json::from_slice(bytes).map_err(Error::from)
    }
}
