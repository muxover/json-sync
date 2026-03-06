//! Flush policies and the background flush worker.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

/// Controls when the map gets written to disk.
#[non_exhaustive]
#[derive(Debug, Clone)]
pub enum FlushPolicy {
    /// Write after every insert/remove. Safest, but most I/O.
    Immediate,
    /// Background thread writes on a timer and whenever the map changes.
    Async(Duration),
    /// Only write when you call `flush()` yourself.
    Manual,
}

/// Background thread that calls a flush closure on a timer or when poked.
/// Joins the thread on drop so nothing leaks.
pub struct AsyncFlushWorker {
    stop: Arc<AtomicBool>,
    tx: Option<mpsc::SyncSender<()>>,
    join_handle: Option<thread::JoinHandle<()>>,
}

impl AsyncFlushWorker {
    /// Spawn a worker using an externally-created channel.  The caller keeps the
    /// sender side and drops it when the store is done — that signals the worker
    /// to exit.
    pub fn start_with_receiver<F>(interval: Duration, flush_fn: F, rx: mpsc::Receiver<()>) -> Self
    where
        F: Fn() + Send + 'static,
    {
        let stop = Arc::new(AtomicBool::new(false));
        let stop_flag = Arc::clone(&stop);

        let join_handle = thread::spawn(move || loop {
            if stop_flag.load(Ordering::Relaxed) {
                break;
            }
            match rx.recv_timeout(interval) {
                Ok(()) | Err(mpsc::RecvTimeoutError::Timeout) => flush_fn(),
                Err(mpsc::RecvTimeoutError::Disconnected) => break,
            }
        });

        Self {
            stop,
            tx: None,
            join_handle: Some(join_handle),
        }
    }

    /// Spawn a worker that owns both ends of the channel.
    pub fn start<F>(interval: Duration, flush_fn: F) -> Self
    where
        F: Fn() + Send + 'static,
    {
        let stop = Arc::new(AtomicBool::new(false));
        let stop_flag = Arc::clone(&stop);
        let (tx, rx) = mpsc::sync_channel::<()>(0);

        let join_handle = thread::spawn(move || loop {
            if stop_flag.load(Ordering::Relaxed) {
                break;
            }
            match rx.recv_timeout(interval) {
                Ok(()) | Err(mpsc::RecvTimeoutError::Timeout) => flush_fn(),
                Err(mpsc::RecvTimeoutError::Disconnected) => break,
            }
        });

        Self {
            stop,
            tx: Some(tx),
            join_handle: Some(join_handle),
        }
    }

    /// Non-blocking nudge to flush now. If the worker is busy the nudge is
    /// silently dropped (the next timer tick will catch up).
    pub fn trigger(&self) {
        if let Some(ref t) = self.tx {
            let _ = t.try_send(());
        }
    }
}

impl Drop for AsyncFlushWorker {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        drop(self.tx.take());
        if let Some(h) = self.join_handle.take() {
            let _ = h.join();
        }
    }
}
