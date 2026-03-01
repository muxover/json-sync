//! Disk I/O helpers: load from file and atomic write.
//!
//! The rename-over approach is close to atomic on most platforms. On NTFS
//! (Windows) it's reliable; on FAT32 or network shares there are no hard
//! guarantees. If that matters to you, keep backups or use a real database.

use crate::error::{Error, Result};
use crate::serializer::Serializer;
use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;

/// Reads and deserializes the file at `path`. Returns an empty map if the file
/// is missing or empty (not an error).
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

/// Write `bytes` to `<path>.tmp` and then rename over `path`. This avoids
/// leaving a half-written file if the process crashes mid-write.
pub fn atomic_write(path: &Path, bytes: &[u8]) -> Result<()> {
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("json");
    let tmp = path.with_extension(format!("{ext}.tmp"));
    std::fs::write(&tmp, bytes).map_err(|e| Error::Io(e.to_string()))?;
    std::fs::rename(&tmp, path).map_err(|e| Error::Io(e.to_string()))?;
    Ok(())
}
