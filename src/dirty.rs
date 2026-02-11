use parking_lot::RwLock;
use std::collections::HashSet;
use std::hash::Hash;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering;

/// Strategy for tracking dirty entries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DirtyStrategy {
    /// Track dirty state per shard (faster, less granular).
    PerShard,
    /// Track dirty state per key (more granular, can flush only changed keys).
    PerKey,
}

/// Trait for tracking dirty state of entries.
pub trait DirtyTracker<K> {
    /// Mark an entry as dirty.
    fn mark_dirty(&self, key: &K);
    
    /// Mark a shard as dirty (for per-shard tracking).
    fn mark_shard_dirty(&self, shard_idx: usize);
    
    /// Clear dirty flags for all entries.
    fn clear_all(&self);
    
    /// Get all dirty keys (for per-key tracking).
    fn get_dirty_keys(&self) -> Vec<K>
    where
        K: Clone;
    
    /// Get all dirty shard indices (for per-shard tracking).
    fn get_dirty_shards(&self) -> Vec<usize>;
    
    /// Check if any entries are dirty.
    fn has_dirty(&self) -> bool;
    
    /// Get the count of dirty entries.
    fn dirty_count(&self) -> usize;
}

/// Per-shard dirty tracker.
/// 
/// Tracks which shards have changes, but not individual keys.
/// This is faster but less granular - when flushing, entire shards
/// must be written even if only one key changed.
pub struct PerShardTracker {
    /// Bitmap of dirty shards (up to 64 shards supported).
    dirty_shards: AtomicU64,
    /// Number of shards being tracked.
    shard_count: usize,
}

impl PerShardTracker {
    /// Create a new per-shard tracker.
    pub fn new(shard_count: usize) -> Self {
        Self {
            dirty_shards: AtomicU64::new(0),
            shard_count,
        }
    }
}

impl<K> DirtyTracker<K> for PerShardTracker {
    fn mark_dirty(&self, _key: &K) {
        // For per-shard tracking, we need the shard index
        // This will be called with mark_shard_dirty instead
    }
    
    fn mark_shard_dirty(&self, shard_idx: usize) {
        if shard_idx < self.shard_count && shard_idx < 64 {
            let mask = 1u64 << shard_idx;
            self.dirty_shards.fetch_or(mask, Ordering::Relaxed);
        }
    }
    
    fn clear_all(&self) {
        self.dirty_shards.store(0, Ordering::Relaxed);
    }
    
    fn get_dirty_keys(&self) -> Vec<K>
    where
        K: Clone,
    {
        // Per-shard tracker doesn't track individual keys
        Vec::new()
    }
    
    fn get_dirty_shards(&self) -> Vec<usize> {
        let bits = self.dirty_shards.load(Ordering::Relaxed);
        let mut shards = Vec::new();
        for i in 0..self.shard_count.min(64) {
            if (bits >> i) & 1 != 0 {
                shards.push(i);
            }
        }
        shards
    }
    
    fn has_dirty(&self) -> bool {
        self.dirty_shards.load(Ordering::Relaxed) != 0
    }
    
    fn dirty_count(&self) -> usize {
        <PerShardTracker as DirtyTracker<String>>::get_dirty_shards(self).len()
    }
}

/// Per-key dirty tracker.
/// 
/// Tracks individual keys that have changed, allowing granular
/// flushing of only changed entries. More memory overhead but
/// more efficient for sparse updates.
pub struct PerKeyTracker<K> {
    /// Set of dirty keys.
    dirty_keys: RwLock<HashSet<K>>,
}

impl<K> PerKeyTracker<K>
where
    K: Hash + Eq,
{
    /// Create a new per-key tracker.
    pub fn new() -> Self {
        Self {
            dirty_keys: RwLock::new(HashSet::new()),
        }
    }
}

impl<K> Default for PerKeyTracker<K>
where
    K: Hash + Eq,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<K> DirtyTracker<K> for PerKeyTracker<K>
where
    K: Hash + Eq + Clone,
{
    fn mark_dirty(&self, key: &K) {
        self.dirty_keys.write().insert(key.clone());
    }
    
    fn mark_shard_dirty(&self, _shard_idx: usize) {
        // For per-key tracking, we need individual keys
        // This should be called with mark_dirty instead
    }
    
    fn clear_all(&self) {
        self.dirty_keys.write().clear();
    }
    
    fn get_dirty_keys(&self) -> Vec<K>
    where
        K: Clone,
    {
        self.dirty_keys.read().iter().cloned().collect()
    }
    
    fn get_dirty_shards(&self) -> Vec<usize> {
        // Per-key tracker doesn't track shards
        Vec::new()
    }
    
    fn has_dirty(&self) -> bool {
        !self.dirty_keys.read().is_empty()
    }
    
    fn dirty_count(&self) -> usize {
        self.dirty_keys.read().len()
    }
}

/// Hybrid tracker that supports both strategies.
/// 
/// This is used internally by JsonSync to support both
/// per-shard and per-key tracking based on configuration.
pub enum DirtyTrackerImpl<K> {
    /// Per-shard tracking implementation.
    PerShard(PerShardTracker),
    /// Per-key tracking implementation.
    PerKey(PerKeyTracker<K>),
}

impl<K> DirtyTrackerImpl<K>
where
    K: Hash + Eq + Clone,
{
    /// Create a new tracker based on strategy.
    pub fn new(strategy: DirtyStrategy, shard_count: usize) -> Self {
        match strategy {
            DirtyStrategy::PerShard => {
                DirtyTrackerImpl::PerShard(PerShardTracker::new(shard_count))
            }
            DirtyStrategy::PerKey => {
                DirtyTrackerImpl::PerKey(PerKeyTracker::new())
            }
        }
    }
    
    /// Mark a key as dirty.
    pub fn mark_dirty(&self, key: &K) {
        match self {
            DirtyTrackerImpl::PerShard(_) => {
                // For per-shard, we need shard index - this will be handled by mark_shard_dirty
            }
            DirtyTrackerImpl::PerKey(tracker) => {
                tracker.mark_dirty(key);
            }
        }
    }
    
    /// Mark a shard as dirty.
    pub fn mark_shard_dirty(&self, shard_idx: usize) {
        match self {
            DirtyTrackerImpl::PerShard(tracker) => {
                <PerShardTracker as DirtyTracker<String>>::mark_shard_dirty(tracker, shard_idx);
            }
            DirtyTrackerImpl::PerKey(_) => {
                // For per-key, we need individual keys - this will be handled by mark_dirty
            }
        }
    }
    
    /// Clear all dirty flags.
    pub fn clear_all(&self) {
        match self {
            DirtyTrackerImpl::PerShard(tracker) => {
                <PerShardTracker as DirtyTracker<String>>::clear_all(tracker);
            }
            DirtyTrackerImpl::PerKey(tracker) => tracker.clear_all(),
        }
    }
    
    /// Get dirty keys.
    pub fn get_dirty_keys(&self) -> Vec<K> {
        match self {
            DirtyTrackerImpl::PerShard(_) => Vec::new(),
            DirtyTrackerImpl::PerKey(tracker) => tracker.get_dirty_keys(),
        }
    }
    
    /// Get dirty shards.
    pub fn get_dirty_shards(&self) -> Vec<usize> {
        match self {
            DirtyTrackerImpl::PerShard(tracker) => {
                <PerShardTracker as DirtyTracker<String>>::get_dirty_shards(tracker)
            }
            DirtyTrackerImpl::PerKey(_) => Vec::new(),
        }
    }
    
    /// Check if any entries are dirty.
    pub fn has_dirty(&self) -> bool {
        match self {
            DirtyTrackerImpl::PerShard(tracker) => {
                <PerShardTracker as DirtyTracker<String>>::has_dirty(tracker)
            }
            DirtyTrackerImpl::PerKey(tracker) => tracker.has_dirty(),
        }
    }
    
    /// Get dirty count.
    pub fn dirty_count(&self) -> usize {
        match self {
            DirtyTrackerImpl::PerShard(tracker) => {
                <PerShardTracker as DirtyTracker<String>>::dirty_count(tracker)
            }
            DirtyTrackerImpl::PerKey(tracker) => tracker.dirty_count(),
        }
    }
}

