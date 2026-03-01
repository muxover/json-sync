//! Unified error type for all store operations.

/// Things that can go wrong when using the store.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// File system problem (read, write, rename).
    Io(String),
    /// Failed to serialize the map to bytes.
    Serialize(String),
    /// Failed to deserialize bytes back into the map.
    Deserialize(String),
    /// Bad configuration (invalid path, policy, etc.).
    Config(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Io(msg) => write!(f, "i/o error: {msg}"),
            Error::Serialize(msg) => write!(f, "serialization error: {msg}"),
            Error::Deserialize(msg) => write!(f, "deserialization error: {msg}"),
            Error::Config(msg) => write!(f, "config error: {msg}"),
        }
    }
}

impl std::error::Error for Error {}

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

/// Result alias using our [`Error`] type.
pub type Result<T> = std::result::Result<T, Error>;
