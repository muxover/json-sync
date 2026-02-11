use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::time::Instant;

/// Statistics for JsonSync operations.
#[derive(Debug, Clone)]
pub struct JsonSyncStats {
    /// Total number of flush operations performed.
    pub flush_count: u64,
    /// Timestamp of the last flush operation.
    pub last_flush_time: Option<Instant>,
    /// Current number of dirty entries.
    pub dirty_count: usize,
    /// Total bytes written to disk.
    pub total_bytes_written: u64,
    /// Number of recovery operations performed.
    pub recovery_count: u64,
    /// Number of serialization errors.
    pub serialize_errors: u64,
    /// Number of I/O errors.
    pub io_errors: u64,
}

impl Default for JsonSyncStats {
    fn default() -> Self {
        Self {
            flush_count: 0,
            last_flush_time: None,
            dirty_count: 0,
            total_bytes_written: 0,
            recovery_count: 0,
            serialize_errors: 0,
            io_errors: 0,
        }
    }
}

/// Thread-safe statistics tracker.
pub(crate) struct StatsTracker {
    flush_count: AtomicU64,
    last_flush_time: parking_lot::RwLock<Option<Instant>>,
    dirty_count: AtomicUsize,
    total_bytes_written: AtomicU64,
    recovery_count: AtomicU64,
    serialize_errors: AtomicU64,
    io_errors: AtomicU64,
}

impl StatsTracker {
    /// Create a new statistics tracker.
    pub fn new() -> Self {
        Self {
            flush_count: AtomicU64::new(0),
            last_flush_time: parking_lot::RwLock::new(None),
            dirty_count: AtomicUsize::new(0),
            total_bytes_written: AtomicU64::new(0),
            recovery_count: AtomicU64::new(0),
            serialize_errors: AtomicU64::new(0),
            io_errors: AtomicU64::new(0),
        }
    }

    /// Record a flush operation.
    pub fn record_flush(&self, bytes_written: usize) {
        self.flush_count.fetch_add(1, Ordering::Relaxed);
        *self.last_flush_time.write() = Some(Instant::now());
        self.total_bytes_written
            .fetch_add(bytes_written as u64, Ordering::Relaxed);
    }

    /// Update the dirty count.
    pub fn set_dirty_count(&self, count: usize) {
        self.dirty_count.store(count, Ordering::Relaxed);
    }

    /// Record a recovery operation.
    pub fn record_recovery(&self) {
        self.recovery_count.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a serialization error.
    #[allow(dead_code)] // Public API, may be used in the future
    pub fn record_serialize_error(&self) {
        self.serialize_errors.fetch_add(1, Ordering::Relaxed);
    }

    /// Record an I/O error.
    pub fn record_io_error(&self) {
        self.io_errors.fetch_add(1, Ordering::Relaxed);
    }

    /// Get a snapshot of current statistics.
    pub fn snapshot(&self) -> JsonSyncStats {
        JsonSyncStats {
            flush_count: self.flush_count.load(Ordering::Relaxed),
            last_flush_time: *self.last_flush_time.read(),
            dirty_count: self.dirty_count.load(Ordering::Relaxed),
            total_bytes_written: self.total_bytes_written.load(Ordering::Relaxed),
            recovery_count: self.recovery_count.load(Ordering::Relaxed),
            serialize_errors: self.serialize_errors.load(Ordering::Relaxed),
            io_errors: self.io_errors.load(Ordering::Relaxed),
        }
    }
}

impl Default for StatsTracker {
    fn default() -> Self {
        Self::new()
    }
}

