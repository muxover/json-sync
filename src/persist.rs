//! Persistence: load from file and atomic write.
//!
//! **load** — Reads the file (if missing or empty, returns an empty map), deserializes via the
//! given `Serializer`. Used at `open()` to populate the map.
//!
//! **atomic_write** — Writes bytes to `path.tmp` then `fs::rename` to `path` for crash safety
//! (no partial files visible to readers).

use crate::error::{Error, Result};
use crate::serializer::Serializer;
use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;

/// Load a map from a file. If the file does not exist or is empty, returns an empty map.
pub fn load<K, V, S>(path: &Path, serializer: &S) -> Result<HashMap<K, V>>
where
    K: for<'de> Deserialize<'de> + Eq + std::hash::Hash,
    V: for<'de> Deserialize<'de>,
    S: Serializer,
{
    let bytes = match std::fs::read(path) {
        Ok(b) => b,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(HashMap::new()),
        Err(e) => return Err(Error::Io(e.to_string())),
    };
    if bytes.is_empty() {
        return Ok(HashMap::new());
    }
    serializer.deserialize(&bytes)
}

/// Write bytes to path atomically: write to `path.tmp` then rename to `path`.
/// Rename overwrites the destination on all supported platforms.
pub fn atomic_write(path: &Path, bytes: &[u8]) -> Result<()> {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("json");
    let tmp = path.with_extension(format!("{}.tmp", ext));
    std::fs::write(&tmp, bytes).map_err(|e| Error::Io(e.to_string()))?;
    std::fs::rename(&tmp, path).map_err(|e| Error::Io(e.to_string()))?;
    Ok(())
}
