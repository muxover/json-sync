use crate::error::JsonSyncError;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::hash::Hash;

/// Serialization format to use.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SerializationFormat {
    /// JSON format (human-readable, slower).
    Json,
    /// Binary format using bincode (faster, requires binary feature).
    #[cfg(feature = "binary")]
    Binary,
}

/// Trait for serializing and deserializing map data.
pub trait Serializer {
    /// Serialize a map to bytes.
    fn serialize<K, V>(&self, data: &HashMap<K, V>) -> Result<Vec<u8>, JsonSyncError>
    where
        K: Serialize,
        V: Serialize;

    /// Deserialize bytes to a map.
    fn deserialize<K, V>(&self, bytes: &[u8]) -> Result<HashMap<K, V>, JsonSyncError>
    where
        K: for<'de> Deserialize<'de> + Hash + Eq,
        V: for<'de> Deserialize<'de>;

    /// Get the file extension for this format.
    fn file_extension(&self) -> &'static str;
}

/// JSON serializer implementation.
#[derive(Clone)]
pub struct JsonSerializer;

impl JsonSerializer {
    /// Create a new JSON serializer.
    pub fn new() -> Self {
        Self
    }
}

impl Default for JsonSerializer {
    fn default() -> Self {
        Self::new()
    }
}

impl Serializer for JsonSerializer {
    fn serialize<K, V>(&self, data: &HashMap<K, V>) -> Result<Vec<u8>, JsonSyncError>
    where
        K: Serialize,
        V: Serialize,
    {
        serde_json::to_vec(data).map_err(JsonSyncError::from)
    }

    fn deserialize<K, V>(&self, bytes: &[u8]) -> Result<HashMap<K, V>, JsonSyncError>
    where
        K: for<'de> Deserialize<'de> + Hash + Eq,
        V: for<'de> Deserialize<'de>,
    {
        serde_json::from_slice(bytes).map_err(|e| JsonSyncError::DeserializeError(e.to_string()))
    }

    fn file_extension(&self) -> &'static str {
        "json"
    }
}

/// Binary serializer implementation (bincode).
#[cfg(feature = "binary")]
pub struct BinarySerializer;

#[cfg(feature = "binary")]
impl BinarySerializer {
    /// Create a new binary serializer.
    pub fn new() -> Self {
        Self
    }
}

#[cfg(feature = "binary")]
impl Default for BinarySerializer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "binary")]
impl Serializer for BinarySerializer {
    fn serialize<K, V>(&self, data: &HashMap<K, V>) -> Result<Vec<u8>, JsonSyncError>
    where
        K: Serialize,
        V: Serialize,
    {
        bincode::serialize(data).map_err(JsonSyncError::from)
    }

    fn deserialize<K, V>(&self, bytes: &[u8]) -> Result<HashMap<K, V>, JsonSyncError>
    where
        K: for<'de> Deserialize<'de>,
        V: for<'de> Deserialize<'de>,
    {
        bincode::deserialize(bytes)
            .map_err(|e| JsonSyncError::DeserializeError(e.to_string()))
    }

    fn file_extension(&self) -> &'static str {
        "bin"
    }
}

/// Serializer implementation that can use either JSON or binary format.
#[derive(Clone)]
pub enum SerializerImpl {
    /// JSON serializer.
    Json(JsonSerializer),
    /// Binary serializer (requires binary feature).
    #[cfg(feature = "binary")]
    Binary(BinarySerializer),
}

impl SerializerImpl {
    /// Create a new serializer based on format.
    pub fn new(format: SerializationFormat) -> Self {
        match format {
            SerializationFormat::Json => SerializerImpl::Json(JsonSerializer::new()),
            #[cfg(feature = "binary")]
            SerializationFormat::Binary => SerializerImpl::Binary(BinarySerializer::new()),
        }
    }

    /// Serialize data to bytes.
    pub fn serialize<K, V>(&self, data: &HashMap<K, V>) -> Result<Vec<u8>, JsonSyncError>
    where
        K: Serialize,
        V: Serialize,
    {
        match self {
            SerializerImpl::Json(ser) => ser.serialize(data),
            #[cfg(feature = "binary")]
            SerializerImpl::Binary(ser) => ser.serialize(data),
        }
    }

    /// Deserialize bytes to data.
    pub fn deserialize<K, V>(&self, bytes: &[u8]) -> Result<HashMap<K, V>, JsonSyncError>
    where
        K: for<'de> Deserialize<'de> + Hash + Eq,
        V: for<'de> Deserialize<'de>,
    {
        match self {
            SerializerImpl::Json(ser) => ser.deserialize(bytes),
            #[cfg(feature = "binary")]
            SerializerImpl::Binary(ser) => ser.deserialize(bytes),
        }
    }

    /// Get file extension.
    pub fn file_extension(&self) -> &'static str {
        match self {
            SerializerImpl::Json(ser) => ser.file_extension(),
            #[cfg(feature = "binary")]
            SerializerImpl::Binary(ser) => ser.file_extension(),
        }
    }
}

/// Convert binary data to JSON format.
/// 
/// This is useful when using binary format internally for speed,
/// but needing JSON for external use.
#[cfg(feature = "binary")]
pub fn binary_to_json<K, V>(binary_data: &[u8]) -> Result<String, JsonSyncError>
where
    K: for<'de> Deserialize<'de> + Serialize,
    V: for<'de> Deserialize<'de> + Serialize,
{
    let binary_ser = BinarySerializer::new();
    let data: HashMap<K, V> = binary_ser.deserialize(binary_data)?;
    let json_ser = JsonSerializer::new();
    let json_bytes = json_ser.serialize(&data)?;
    String::from_utf8(json_bytes)
        .map_err(|e| JsonSyncError::SerializeError(format!("Failed to convert to JSON: {}", e)))
}

/// Convert JSON data to binary format.
#[cfg(feature = "binary")]
pub fn json_to_binary<K, V>(json_data: &[u8]) -> Result<Vec<u8>, JsonSyncError>
where
    K: for<'de> Deserialize<'de> + Serialize,
    V: for<'de> Deserialize<'de> + Serialize,
{
    let json_ser = JsonSerializer::new();
    let data: HashMap<K, V> = json_ser.deserialize(json_data)?;
    let binary_ser = BinarySerializer::new();
    binary_ser.serialize(&data)
}

