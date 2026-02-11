use crate::dirty::{DirtyStrategy, DirtyTrackerImpl};
use crate::error::JsonSyncError;
use crate::flush::{FlushScheduler, FlushStrategy};
use crate::recovery::{ensure_extension, format_from_extension, load_from_file};
use crate::serializer::{SerializationFormat, SerializerImpl};
use crate::stats::{StatsTracker, JsonSyncStats};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::hash::Hash;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Callback function type for getting all data from a map.
pub type DataGetter<K, V> = Arc<dyn Fn() -> HashMap<K, V> + Send + Sync>;

/// Callback function type for loading data into a map.
pub type DataLoader<K, V> = Arc<dyn Fn(HashMap<K, V>) + Send + Sync>;

/// High-performance JSON synchronization for map-like structures.
/// 
/// JsonSync provides asynchronous flushing, configurable dirty tracking,
/// and recovery mechanisms for any map-like structure.
/// 
/// # Example
/// 
/// ```rust,no_run
/// use json_sync::JsonSyncBuilder;
/// use std::collections::HashMap;
/// use std::sync::Arc;
/// 
/// let map = Arc::new(parking_lot::RwLock::new(HashMap::new()));
/// 
/// let get_data = {
///     let map = Arc::clone(&map);
///     Arc::new(move || {
///         map.read().clone()
///     })
/// };
/// 
/// let load_data = {
///     let map = Arc::clone(&map);
///     Arc::new(move |data: HashMap<String, i32>| {
///         *map.write() = data;
///     })
/// };
/// 
/// let sync = JsonSyncBuilder::new()
///     .path("data.json")
///     .build::<String, i32>()?;
/// 
/// sync.attach(get_data, load_data)?;
/// sync.start()?;
/// # Ok::<(), json_sync::JsonSyncError>(())
/// ```
pub struct JsonSync<K, V> {
    /// Path to the JSON file.
    path: PathBuf,
    /// Callback to get all data from the map.
    data_getter: Arc<RwLock<Option<DataGetter<K, V>>>>,
    /// Callback to load data into the map.
    data_loader: Arc<RwLock<Option<DataLoader<K, V>>>>,
    /// Dirty tracker.
    dirty_tracker: Arc<DirtyTrackerImpl<String>>,
    /// Flush scheduler.
    flush_scheduler: Arc<FlushScheduler>,
    /// Serializer.
    serializer: SerializerImpl,
    /// Statistics tracker.
    stats: Arc<StatsTracker>,
    /// Shard count (for per-shard tracking).
    #[allow(dead_code)]
    shard_count: usize,
}

impl<K, V> JsonSync<K, V>
where
    K: Hash + Eq + Send + Sync + Clone + Serialize + for<'de> Deserialize<'de> + std::fmt::Debug + 'static,
    V: Send + Sync + Serialize + for<'de> Deserialize<'de> + 'static,
{
    /// Create a new JsonSync instance.
    /// 
    /// This is typically called through `JsonSyncBuilder::build()`.
    pub(crate) fn new(
        path: PathBuf,
        flush_strategy: FlushStrategy,
        batch_size: usize,
        dirty_strategy: DirtyStrategy,
        serialization_format: SerializationFormat,
        shard_count: usize,
    ) -> Self {
        let path = ensure_extension(path, serialization_format);
        
        Self {
            path: path.clone(),
            data_getter: Arc::new(RwLock::new(None)),
            data_loader: Arc::new(RwLock::new(None)),
            dirty_tracker: Arc::new(DirtyTrackerImpl::new(dirty_strategy, shard_count)),
            flush_scheduler: Arc::new(FlushScheduler::new(flush_strategy, batch_size)),
            serializer: SerializerImpl::new(serialization_format),
            stats: Arc::new(StatsTracker::new()),
            shard_count,
        }
    }


    /// Attach callbacks for getting and loading data.
    /// 
    /// # Arguments
    /// 
    /// * `getter` - Callback to get all data from the map
    /// * `loader` - Callback to load data into the map
    pub fn attach(
        &self,
        getter: DataGetter<K, V>,
        loader: DataLoader<K, V>,
    ) -> Result<(), JsonSyncError> {
        *self.data_getter.write() = Some(getter);
        *self.data_loader.write() = Some(loader);
        Ok(())
    }

    /// Mark a key as dirty (should be called when the key is modified).
    /// 
    /// # Arguments
    /// 
    /// * `key` - The key that was modified
    pub fn mark_dirty(&self, key: &str) {
        self.dirty_tracker.mark_dirty(&key.to_string());
        self.flush_scheduler.record_change();
    }

    /// Mark a shard as dirty (for per-shard tracking).
    /// 
    /// # Arguments
    /// 
    /// * `shard_idx` - The shard index that was modified
    pub fn mark_shard_dirty(&self, shard_idx: usize) {
        self.dirty_tracker.mark_shard_dirty(shard_idx);
        self.flush_scheduler.record_change();
    }

    /// Flush all dirty entries to disk immediately.
    /// 
    /// This is a blocking operation that serializes and writes all
    /// dirty entries to the configured file path.
    pub fn flush(&self) -> Result<(), JsonSyncError> {
        let getter_guard = self.data_getter.read();
        let getter = getter_guard.as_ref().ok_or(JsonSyncError::MapNotAttached)?;

        // Collect dirty entries
        let dirty_keys = self.dirty_tracker.get_dirty_keys();
        let dirty_shards = self.dirty_tracker.get_dirty_shards();

        // Get all data from map
        let data = getter();

        // Filter to only dirty entries if per-key tracking
        let data_to_flush: HashMap<K, V> = if !dirty_keys.is_empty() {
            data.into_iter()
                .filter(|(k, _)| {
                    let key_str = format!("{:?}", k);
                    dirty_keys.contains(&key_str)
                })
                .collect()
        } else if !dirty_shards.is_empty() {
            // For per-shard, flush everything if any shard is dirty
            data
        } else {
            // No dirty entries, but flush everything anyway for safety
            data
        };

        // Serialize
        let bytes = self.serializer.serialize(&data_to_flush)?;

        // Write to file atomically (write to temp file, then rename)
        let temp_path = self.path.with_extension(format!(
            "{}.tmp",
            self.path.extension().and_then(|s| s.to_str()).unwrap_or("json")
        ));
        
        std::fs::write(&temp_path, &bytes).map_err(|e| {
            self.stats.record_io_error();
            JsonSyncError::IoError(format!("Failed to write file: {}", e))
        })?;

        std::fs::rename(&temp_path, &self.path).map_err(|e| {
            self.stats.record_io_error();
            JsonSyncError::IoError(format!("Failed to rename temp file: {}", e))
        })?;

        // Update stats
        self.stats.record_flush(bytes.len());
        self.stats.set_dirty_count(0);
        
        // Clear dirty flags
        self.dirty_tracker.clear_all();
        self.flush_scheduler.reset_change_count();

        Ok(())
    }

    /// Trigger an asynchronous flush.
    /// 
    /// This schedules a flush to happen in the background without blocking.
    pub fn flush_async(&self) -> Result<(), JsonSyncError> {
        self.flush_scheduler.trigger_flush()
    }

    /// Start the background flush scheduler.
    /// 
    /// This spawns a background task that will automatically flush
    /// data based on the configured strategy.
    pub fn start(&self) -> Result<(), JsonSyncError> {
        let handle = self.clone_for_scheduler();
        let flush_callback = move || {
            handle.flush()
        };
        self.flush_scheduler.start(flush_callback)
    }

    /// Clone this instance for use in the scheduler.
    /// 
    /// This creates a lightweight clone that shares the same internal state.
    fn clone_for_scheduler(&self) -> Arc<JsonSyncSchedulerHandle<K, V>> {
        Arc::new(JsonSyncSchedulerHandle {
            data_getter: Arc::clone(&self.data_getter),
            dirty_tracker: Arc::clone(&self.dirty_tracker),
            serializer: self.serializer.clone(),
            stats: Arc::clone(&self.stats),
            path: self.path.clone(),
        })
    }

    /// Stop the background flush scheduler.
    pub fn stop(&self) {
        self.flush_scheduler.stop();
    }

    /// Load data from file and populate the attached map.
    /// 
    /// This will overwrite any existing data in the map.
    pub fn load_from_file(&self) -> Result<(), JsonSyncError> {
        let format = format_from_extension(&self.path);
        let data: HashMap<K, V> = load_from_file(&self.path, format)?;

        let loader_guard = self.data_loader.read();
        let loader = loader_guard.as_ref().ok_or(JsonSyncError::MapNotAttached)?;

        loader(data);

        self.stats.record_recovery();
        Ok(())
    }

    /// Get statistics about sync operations.
    pub fn stats(&self) -> JsonSyncStats {
        let mut stats = self.stats.snapshot();
        stats.dirty_count = self.dirty_tracker.dirty_count();
        stats
    }

    /// Get the file path.
    pub fn path(&self) -> &Path {
        &self.path
    }
}

/// Lightweight handle for the scheduler to use.
struct JsonSyncSchedulerHandle<K, V> {
    data_getter: Arc<RwLock<Option<DataGetter<K, V>>>>,
    dirty_tracker: Arc<DirtyTrackerImpl<String>>,
    serializer: SerializerImpl,
    stats: Arc<StatsTracker>,
    path: PathBuf,
}

impl<K, V> JsonSyncSchedulerHandle<K, V>
where
    K: Hash + Eq + Send + Sync + Clone + Serialize + for<'de> Deserialize<'de> + std::fmt::Debug + 'static,
    V: Send + Sync + Serialize + for<'de> Deserialize<'de> + 'static,
{
    fn flush(self: &Arc<Self>) -> Result<(), JsonSyncError> {
        let getter_guard = self.data_getter.read();
        let getter = getter_guard.as_ref().ok_or(JsonSyncError::MapNotAttached)?;

        let data = getter();
        let bytes = self.serializer.serialize(&data)?;

        let temp_path = self.path.with_extension(format!(
            "{}.tmp",
            self.path.extension().and_then(|s| s.to_str()).unwrap_or("json")
        ));
        
        std::fs::write(&temp_path, &bytes).map_err(|e| {
            self.stats.record_io_error();
            JsonSyncError::IoError(format!("Failed to write file: {}", e))
        })?;

        std::fs::rename(&temp_path, &self.path).map_err(|e| {
            self.stats.record_io_error();
            JsonSyncError::IoError(format!("Failed to rename temp file: {}", e))
        })?;

        self.stats.record_flush(bytes.len());
        self.stats.set_dirty_count(0);
        self.dirty_tracker.clear_all();

        Ok(())
    }
}

