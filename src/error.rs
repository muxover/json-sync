/// Errors that can occur when operating on JsonSync.
#[derive(Debug, Clone)]
pub enum JsonSyncError {
    /// An I/O error occurred (e.g., file read/write failure).
    IoError(String),
    /// A serialization error occurred.
    SerializeError(String),
    /// A deserialization error occurred.
    DeserializeError(String),
    /// The provided path is invalid.
    InvalidPath(String),
    /// An error occurred during recovery.
    RecoveryError(String),
    /// An error occurred during flush operation.
    FlushError(String),
    /// The map is not attached to the sync instance.
    MapNotAttached,
    /// Invalid configuration provided.
    InvalidConfiguration(String),
}

impl std::fmt::Display for JsonSyncError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            JsonSyncError::IoError(msg) => write!(f, "I/O error: {}", msg),
            JsonSyncError::SerializeError(msg) => write!(f, "Serialization error: {}", msg),
            JsonSyncError::DeserializeError(msg) => write!(f, "Deserialization error: {}", msg),
            JsonSyncError::InvalidPath(msg) => write!(f, "Invalid path: {}", msg),
            JsonSyncError::RecoveryError(msg) => write!(f, "Recovery error: {}", msg),
            JsonSyncError::FlushError(msg) => write!(f, "Flush error: {}", msg),
            JsonSyncError::MapNotAttached => write!(f, "Map is not attached to sync instance"),
            JsonSyncError::InvalidConfiguration(msg) => {
                write!(f, "Invalid configuration: {}", msg)
            }
        }
    }
}

impl std::error::Error for JsonSyncError {}

impl From<std::io::Error> for JsonSyncError {
    fn from(err: std::io::Error) -> Self {
        JsonSyncError::IoError(err.to_string())
    }
}

impl From<serde_json::Error> for JsonSyncError {
    fn from(err: serde_json::Error) -> Self {
        JsonSyncError::SerializeError(err.to_string())
    }
}

#[cfg(feature = "binary")]
impl From<bincode::Error> for JsonSyncError {
    fn from(err: bincode::Error) -> Self {
        JsonSyncError::SerializeError(err.to_string())
    }
}

