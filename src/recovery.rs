use crate::error::JsonSyncError;
use crate::serializer::{SerializationFormat, SerializerImpl};
use serde::Deserialize;
use std::collections::HashMap;
use std::hash::Hash;
use std::path::{Path, PathBuf};

/// Load data from a file and populate a map.
/// 
/// This function handles file reading, deserialization, and error recovery.
/// It supports both JSON and binary formats based on the file extension.
/// 
/// # Arguments
/// 
/// * `path` - Path to the file to load
/// * `format` - Serialization format to use
/// 
/// # Returns
/// 
/// A `HashMap` containing the loaded data, or an error if loading fails.
/// 
/// # Example
/// 
/// ```rust,no_run
/// use json_sync::recovery::load_from_file;
/// use json_sync::serializer::SerializationFormat;
/// use std::collections::HashMap;
/// 
/// let data: HashMap<String, i32> = load_from_file(
///     "data.json",
///     SerializationFormat::Json
/// )?;
/// # Ok::<(), json_sync::JsonSyncError>(())
/// ```
pub fn load_from_file<K, V>(
    path: impl AsRef<Path>,
    format: SerializationFormat,
) -> Result<HashMap<K, V>, JsonSyncError>
where
    K: for<'de> Deserialize<'de> + Hash + Eq,
    V: for<'de> Deserialize<'de>,
{
    let path = path.as_ref();
    
    // Validate path
    if !path.exists() {
        return Err(JsonSyncError::InvalidPath(format!(
            "File does not exist: {}",
            path.display()
        )));
    }
    
    // Read file
    let bytes = std::fs::read(path).map_err(|e| {
        JsonSyncError::RecoveryError(format!("Failed to read file {}: {}", path.display(), e))
    })?;
    
    // Handle empty files
    if bytes.is_empty() {
        return Ok(HashMap::new());
    }
    
    // Deserialize based on format
    let serializer = SerializerImpl::new(format);
    match serializer.deserialize(&bytes) {
        Ok(data) => Ok(data),
        Err(e) => {
            // Try to recover by attempting JSON format if binary failed
            if matches!(format, SerializationFormat::Json) {
                return Err(JsonSyncError::RecoveryError(format!(
                    "Failed to deserialize JSON from {}: {}",
                    path.display(),
                    e
                )));
            }
            
            // Try JSON as fallback
            let json_serializer = SerializerImpl::new(SerializationFormat::Json);
            json_serializer.deserialize(&bytes).map_err(|json_err| {
                JsonSyncError::RecoveryError(format!(
                    "Failed to deserialize from {} (tried both formats): original={}, json={}",
                    path.display(),
                    e,
                    json_err
                ))
            })
        }
    }
}

/// Validate JSON structure without deserializing.
/// 
/// This is useful for checking if a file is valid JSON before attempting
/// to load it into a typed structure.
pub fn validate_json_file(path: impl AsRef<Path>) -> Result<(), JsonSyncError> {
    let path = path.as_ref();
    let bytes = std::fs::read(path).map_err(|e| {
        JsonSyncError::RecoveryError(format!("Failed to read file {}: {}", path.display(), e))
    })?;
    
    serde_json::from_slice::<serde_json::Value>(&bytes).map_err(|e| {
        JsonSyncError::RecoveryError(format!(
            "Invalid JSON in file {}: {}",
            path.display(),
            e
        ))
    })?;
    
    Ok(())
}

/// Attempt to recover data from a corrupted file.
/// 
/// This function tries multiple strategies:
/// 1. Try to parse as JSON
/// 2. Try to parse as binary (if feature enabled)
/// 3. Try to extract partial data if possible
pub fn recover_from_corrupted_file<K, V>(
    path: impl AsRef<Path>,
) -> Result<Option<HashMap<K, V>>, JsonSyncError>
where
    K: for<'de> Deserialize<'de> + Hash + Eq,
    V: for<'de> Deserialize<'de>,
{
    let path = path.as_ref();
    
    if !path.exists() {
        return Ok(None);
    }
    
    let bytes = std::fs::read(path).map_err(|e| {
        JsonSyncError::RecoveryError(format!("Failed to read file {}: {}", path.display(), e))
    })?;
    
    if bytes.is_empty() {
        return Ok(Some(HashMap::new()));
    }
    
    // Try JSON first
    let json_serializer = SerializerImpl::new(SerializationFormat::Json);
    if let Ok(data) = json_serializer.deserialize(&bytes) {
        return Ok(Some(data));
    }
    
    // Try binary if feature enabled
    #[cfg(feature = "binary")]
    {
        let binary_serializer = SerializerImpl::new(SerializationFormat::Binary);
        if let Ok(data) = binary_serializer.deserialize(&bytes) {
            return Ok(Some(data));
        }
    }
    
    // Could not recover
    Ok(None)
}

/// Get the appropriate serialization format based on file extension.
pub fn format_from_extension(path: impl AsRef<Path>) -> SerializationFormat {
    let path = path.as_ref();
    match path.extension().and_then(|ext| ext.to_str()) {
        Some("json") => SerializationFormat::Json,
        #[cfg(feature = "binary")]
        Some("bin") => SerializationFormat::Binary,
        _ => SerializationFormat::Json, // Default to JSON
    }
}

/// Determine the file path with appropriate extension based on format.
pub fn ensure_extension(path: impl AsRef<Path>, format: SerializationFormat) -> PathBuf {
    let path = path.as_ref();
    let serializer = SerializerImpl::new(format);
    let ext = serializer.file_extension();
    
    if path.extension().is_some() {
        path.to_path_buf()
    } else {
        path.with_extension(ext)
    }
}

