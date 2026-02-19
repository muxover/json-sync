//! Flush policy and background flush worker.
//!
//! **FlushPolicy** — `Immediate` (flush after every mutating op), `Async(Duration)` (background
//! thread on interval and on trigger), or `Manual` (only on explicit `flush()`).
//!
//! **Async worker** — Exits gracefully on drop: stop flag is set and the sender is dropped so the
//! receiver sees disconnection; then the worker thread is joined. No dangling threads. Dropping
//! a handle that uses `Async` may block briefly while the worker flushes and exits.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

/// When to persist the map to disk.
#[derive(Debug, Clone)]
pub enum FlushPolicy {
    /// Flush after every mutating operation (insert/remove). Highest consistency, higher IO.
    Immediate,
    /// Flush on a fixed interval in a background thread. Coalesces writes.
    Async(Duration),
    /// Only flush when the user calls `flush()`. No background thread.
    Manual,
}

/// Handles scheduling and running the background flush worker for `Async(Duration)`.
/// On drop, sends stop and joins the worker thread (graceful shutdown).
pub struct AsyncFlushWorker {
    stop: Arc<AtomicBool>,
    tx: Option<mpsc::SyncSender<()>>,
    join_handle: Option<thread::JoinHandle<()>>,
}

impl AsyncFlushWorker {
    /// Start a background thread that calls `flush_fn` every `interval` or when a message is received on `rx`.
    /// Caller creates the channel and holds the sender (e.g. in the store as trigger); when the sender is dropped, the worker exits.
    pub fn start_with_receiver<F>(
        interval: Duration,
        flush_fn: F,
        rx: mpsc::Receiver<()>,
    ) -> Self
    where
        F: Fn() -> () + Send + 'static,
    {
        let stop = Arc::new(AtomicBool::new(false));
        let stop_worker = Arc::clone(&stop);

        let join_handle = thread::spawn(move || {
            loop {
                if stop_worker.load(Ordering::Relaxed) {
                    break;
                }
                match rx.recv_timeout(interval) {
                    Ok(()) => flush_fn(),
                    Err(mpsc::RecvTimeoutError::Timeout) => flush_fn(),
                    Err(mpsc::RecvTimeoutError::Disconnected) => break,
                }
            }
        });

        Self {
            stop,
            tx: None, // caller holds sender; when dropped, worker sees Disconnected
            join_handle: Some(join_handle),
        }
    }

    /// Start a background thread that calls `flush_fn` every `interval`. Worker owns the channel.
    pub fn start<F>(interval: Duration, flush_fn: F) -> Self
    where
        F: Fn() -> () + Send + 'static,
    {
        let stop = Arc::new(AtomicBool::new(false));
        let stop_worker = Arc::clone(&stop);
        let (tx, rx) = mpsc::sync_channel::<()>(0);

        let join_handle = thread::spawn(move || {
            loop {
                if stop_worker.load(Ordering::Relaxed) {
                    break;
                }
                match rx.recv_timeout(interval) {
                    Ok(()) => flush_fn(),
                    Err(mpsc::RecvTimeoutError::Timeout) => flush_fn(),
                    Err(mpsc::RecvTimeoutError::Disconnected) => break,
                }
            }
        });

        Self {
            stop,
            tx: Some(tx),
            join_handle: Some(join_handle),
        }
    }

    /// Request one flush (e.g. after a mutating op when policy is Async).
    pub fn trigger(&self) {
        if let Some(ref t) = self.tx {
            let _ = t.try_send(());
        }
    }
}

impl Drop for AsyncFlushWorker {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        let _ = self.tx.take(); // disconnect receiver so worker thread exits
        if let Some(handle) = self.join_handle.take() {
            let _ = handle.join();
        }
    }
}
