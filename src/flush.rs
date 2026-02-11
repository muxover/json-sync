use crate::error::JsonSyncError;
use parking_lot::Mutex;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::time::interval;

/// Flush strategy for determining when to flush data.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlushStrategy {
    /// Flush every N seconds.
    TimeBased(Duration),
    /// Flush after N changes.
    CountBased(usize),
    /// Only flush on explicit calls (manual).
    Manual,
}

/// Flush scheduler that manages automatic flushing.
pub struct FlushScheduler {
    /// Strategy for flushing.
    strategy: FlushStrategy,
    /// Batch size for collecting dirty entries.
    batch_size: usize,
    /// Change counter (for count-based strategy).
    change_count: Arc<AtomicU64>,
    /// Flag to stop the scheduler.
    stop_flag: Arc<AtomicBool>,
    /// Channel for triggering flushes.
    flush_tx: Arc<Mutex<Option<mpsc::UnboundedSender<()>>>>,
    /// Handle for the background task.
    handle: Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>,
}

impl FlushScheduler {
    /// Create a new flush scheduler.
    pub fn new(strategy: FlushStrategy, batch_size: usize) -> Self {
        Self {
            strategy,
            batch_size,
            change_count: Arc::new(AtomicU64::new(0)),
            stop_flag: Arc::new(AtomicBool::new(false)),
            flush_tx: Arc::new(Mutex::new(None)),
            handle: Arc::new(Mutex::new(None)),
        }
    }

    /// Record a change (for count-based strategy).
    pub fn record_change(&self) {
        if matches!(self.strategy, FlushStrategy::CountBased(_)) {
            self.change_count.fetch_add(1, Ordering::Relaxed);
        }
    }

    /// Get the current change count.
    pub fn change_count(&self) -> u64 {
        self.change_count.load(Ordering::Relaxed)
    }

    /// Reset the change counter.
    pub fn reset_change_count(&self) {
        self.change_count.store(0, Ordering::Relaxed);
    }

    /// Start the background flush scheduler.
    /// 
    /// This spawns a background task that will trigger flushes
    /// based on the configured strategy.
    pub fn start<F>(&self, flush_callback: F) -> Result<(), JsonSyncError>
    where
        F: Fn() -> Result<(), JsonSyncError> + Send + Sync + 'static,
    {
        if matches!(self.strategy, FlushStrategy::Manual) {
            return Ok(());
        }

        let (tx, mut rx) = mpsc::unbounded_channel();
        *self.flush_tx.lock() = Some(tx.clone());

        let stop_flag = Arc::clone(&self.stop_flag);
        let change_count = Arc::clone(&self.change_count);
        let strategy = self.strategy;

        let handle = tokio::spawn(async move {
            match strategy {
                FlushStrategy::TimeBased(duration) => {
                    let mut interval = interval(duration);
                    loop {
                        tokio::select! {
                            _ = interval.tick() => {
                                if let Err(e) = flush_callback() {
                                    eprintln!("Flush error: {}", e);
                                }
                            }
                            _ = rx.recv() => {
                                // Manual flush triggered
                                if let Err(e) = flush_callback() {
                                    eprintln!("Flush error: {}", e);
                                }
                            }
                        }
                        if stop_flag.load(Ordering::Relaxed) {
                            break;
                        }
                    }
                }
                FlushStrategy::CountBased(threshold) => {
                    loop {
                        tokio::select! {
                            _ = rx.recv() => {
                                // Manual flush triggered
                                if let Err(e) = flush_callback() {
                                    eprintln!("Flush error: {}", e);
                                }
                            }
                            _ = tokio::time::sleep(Duration::from_millis(100)) => {
                                // Check change count periodically
                                let count = change_count.load(Ordering::Relaxed);
                                if count >= threshold as u64 {
                                    change_count.store(0, Ordering::Relaxed);
                                    if let Err(e) = flush_callback() {
                                        eprintln!("Flush error: {}", e);
                                    }
                                }
                            }
                        }
                        if stop_flag.load(Ordering::Relaxed) {
                            break;
                        }
                    }
                }
                FlushStrategy::Manual => {
                    // Manual mode - just wait for manual triggers
                    while let Some(_) = rx.recv().await {
                        if let Err(e) = flush_callback() {
                            eprintln!("Flush error: {}", e);
                        }
                        if stop_flag.load(Ordering::Relaxed) {
                            break;
                        }
                    }
                }
            }
        });

        *self.handle.lock() = Some(handle);
        Ok(())
    }

    /// Stop the background scheduler.
    pub fn stop(&self) {
        self.stop_flag.store(true, Ordering::Relaxed);
        
        // Send a final message to wake up the task
        if let Some(tx) = self.flush_tx.lock().as_ref() {
            let _ = tx.send(());
        }
        
        // Wait for the task to finish
        if let Some(handle) = self.handle.lock().take() {
            // Try to wait for the task if we're in an async context
            let rt = tokio::runtime::Handle::try_current();
            if let Ok(rt) = rt {
                // We're in an async context, spawn a task to wait
                rt.spawn(async move {
                    let _ = handle.await;
                });
            } else {
                // Not in an async context, create a new runtime to wait
                let rt = tokio::runtime::Runtime::new().unwrap();
                rt.block_on(async {
                    let _ = handle.await;
                });
            }
        }
    }

    /// Trigger an async flush.
    pub fn trigger_flush(&self) -> Result<(), JsonSyncError> {
        if let Some(tx) = self.flush_tx.lock().as_ref() {
            tx.send(()).map_err(|_| {
                JsonSyncError::FlushError("Failed to send flush signal".to_string())
            })?;
        }
        Ok(())
    }

    /// Get the batch size.
    pub fn batch_size(&self) -> usize {
        self.batch_size
    }

    /// Get the flush strategy.
    pub fn strategy(&self) -> FlushStrategy {
        self.strategy
    }
}

impl Drop for FlushScheduler {
    fn drop(&mut self) {
        self.stop();
    }
}

/// Default flush interval (5 seconds).
pub const DEFAULT_FLUSH_INTERVAL: Duration = Duration::from_secs(5);

/// Default flush threshold (1000 changes).
pub const DEFAULT_FLUSH_THRESHOLD: usize = 1000;

/// Default batch size (100 entries).
pub const DEFAULT_BATCH_SIZE: usize = 100;

