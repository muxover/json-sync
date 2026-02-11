use crate::dirty::DirtyStrategy;
use crate::error::JsonSyncError;
use crate::flush::{FlushStrategy, DEFAULT_BATCH_SIZE, DEFAULT_FLUSH_INTERVAL};
use crate::serializer::SerializationFormat;
use std::path::PathBuf;

/// Builder for creating a JsonSync instance with custom configuration.
/// 
/// # Example
/// 
/// ```rust
/// use json_sync::JsonSyncBuilder;
/// use json_sync::dirty::DirtyStrategy;
/// use std::time::Duration;
/// 
/// let sync = JsonSyncBuilder::new()
///     .path("data.json")
///     .flush_interval(Duration::from_secs(10))
///     .batch_size(500)
///     .dirty_strategy(DirtyStrategy::PerKey)
///     .build::<String, i32>()?;
/// # Ok::<(), json_sync::JsonSyncError>(())
/// ```
pub struct JsonSyncBuilder {
    path: Option<PathBuf>,
    flush_strategy: FlushStrategy,
    batch_size: usize,
    dirty_strategy: DirtyStrategy,
    serialization_format: SerializationFormat,
    shard_count: Option<usize>,
}

impl JsonSyncBuilder {
    /// Create a new builder with default configuration.
    pub fn new() -> Self {
        Self {
            path: None,
            flush_strategy: FlushStrategy::TimeBased(DEFAULT_FLUSH_INTERVAL),
            batch_size: DEFAULT_BATCH_SIZE,
            dirty_strategy: DirtyStrategy::PerShard,
            serialization_format: SerializationFormat::Json,
            shard_count: None,
        }
    }

    /// Set the file path for JSON persistence.
    /// 
    /// # Arguments
    /// 
    /// * `path` - Path to the JSON file (will be created if it doesn't exist)
    pub fn path(mut self, path: impl Into<PathBuf>) -> Self {
        self.path = Some(path.into());
        self
    }

    /// Set the flush interval for time-based flushing.
    /// 
    /// This sets the strategy to `FlushStrategy::TimeBased` with the given duration.
    /// 
    /// # Arguments
    /// 
    /// * `duration` - How often to flush data to disk
    pub fn flush_interval(mut self, duration: std::time::Duration) -> Self {
        self.flush_strategy = FlushStrategy::TimeBased(duration);
        self
    }

    /// Set the flush threshold for count-based flushing.
    /// 
    /// This sets the strategy to `FlushStrategy::CountBased` with the given threshold.
    /// 
    /// # Arguments
    /// 
    /// * `count` - Number of changes after which to flush
    pub fn flush_threshold(mut self, count: usize) -> Self {
        self.flush_strategy = FlushStrategy::CountBased(count);
        self
    }

    /// Set the flush strategy to manual (only flush on explicit calls).
    pub fn manual_flush(mut self) -> Self {
        self.flush_strategy = FlushStrategy::Manual;
        self
    }

    /// Set the batch size for collecting dirty entries before flushing.
    /// 
    /// # Arguments
    /// 
    /// * `size` - Number of entries to collect in a batch
    pub fn batch_size(mut self, size: usize) -> Self {
        self.batch_size = size;
        self
    }

    /// Set the dirty tracking strategy.
    /// 
    /// # Arguments
    /// 
    /// * `strategy` - Either `DirtyStrategy::PerShard` or `DirtyStrategy::PerKey`
    pub fn dirty_strategy(mut self, strategy: DirtyStrategy) -> Self {
        self.dirty_strategy = strategy;
        self
    }

    /// Enable binary format for serialization (requires `binary` feature).
    /// 
    /// When enabled, data is serialized using bincode internally for speed,
    /// but can still be converted to JSON for external use.
    #[cfg(feature = "binary")]
    pub fn use_binary_format(mut self, enable: bool) -> Self {
        if enable {
            self.serialization_format = SerializationFormat::Binary;
        } else {
            self.serialization_format = SerializationFormat::Json;
        }
        self
    }

    /// Set the serialization format explicitly.
    pub fn serialization_format(mut self, format: SerializationFormat) -> Self {
        self.serialization_format = format;
        self
    }

    /// Set the shard count (for per-shard dirty tracking).
    /// 
    /// This is only used when `dirty_strategy` is `PerShard`.
    /// 
    /// # Arguments
    /// 
    /// * `count` - Number of shards (must be power of two)
    pub fn shard_count(mut self, count: usize) -> Self {
        self.shard_count = Some(count);
        self
    }

    /// Build a JsonSync instance.
    /// 
    /// # Returns
    /// 
    /// A configured `JsonSync` instance, or an error if configuration is invalid.
    pub fn build<K, V>(self) -> Result<crate::sync::JsonSync<K, V>, JsonSyncError>
    where
        K: std::hash::Hash + Eq + Send + Sync + Clone + serde::Serialize + for<'de> serde::Deserialize<'de> + std::fmt::Debug + 'static,
        V: Send + Sync + serde::Serialize + for<'de> serde::Deserialize<'de> + 'static,
    {
        let path = self.path.ok_or_else(|| {
            JsonSyncError::InvalidConfiguration("Path is required".to_string())
        })?;

        // Validate batch size
        if self.batch_size == 0 {
            return Err(JsonSyncError::InvalidConfiguration(
                "Batch size must be greater than 0".to_string(),
            ));
        }

        // Validate shard count if provided
        if let Some(shard_count) = self.shard_count {
            if shard_count == 0 || !shard_count.is_power_of_two() {
                return Err(JsonSyncError::InvalidConfiguration(format!(
                    "Shard count must be a power of two and greater than 0, got {}",
                    shard_count
                )));
            }
        }

        Ok(crate::sync::JsonSync::new(
            path,
            self.flush_strategy,
            self.batch_size,
            self.dirty_strategy,
            self.serialization_format,
            self.shard_count.unwrap_or(16),
        ))
    }
}

impl Default for JsonSyncBuilder {
    fn default() -> Self {
        Self::new()
    }
}

