//! Error types for json-sync.
//!
//! All fallible public operations (`insert`, `remove`, `flush`, `open`) return
//! `Result<_, Error>` so callers can use a single `?` / `map_err` strategy.
//! Variants: `Io`, `Serialize`, `Deserialize`, `Config`. Implements `From`
//! for `std::io::Error` and `serde_json::Error`.

/// Errors that can occur when using json-sync.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// I/O error (file read, write, or rename).
    Io(String),
    /// Serialization error (map to bytes).
    Serialize(String),
    /// Deserialization error (bytes to map).
    Deserialize(String),
    /// Invalid configuration (path, flush policy, etc.).
    Config(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Io(msg) => write!(f, "I/O error: {}", msg),
            Error::Serialize(msg) => write!(f, "serialization error: {}", msg),
            Error::Deserialize(msg) => write!(f, "deserialization error: {}", msg),
            Error::Config(msg) => write!(f, "config error: {}", msg),
        }
    }
}

impl std::error::Error for Error {}

// Error is safe to send and share across threads (only contains Strings).
unsafe impl Send for Error {}
unsafe impl Sync for Error {}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Error::Io(err.to_string())
    }
}

impl From<serde_json::Error> for Error {
    fn from(err: serde_json::Error) -> Self {
        if err.is_io() {
            Error::Io(err.to_string())
        } else if err.is_syntax() || err.is_eof() {
            Error::Deserialize(err.to_string())
        } else {
            Error::Serialize(err.to_string())
        }
    }
}

/// Result type alias for json-sync operations.
pub type Result<T> = std::result::Result<T, Error>;
