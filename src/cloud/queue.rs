//! Offline queue for events when cloud is unreachable
//!
//! Events are queued in memory and persisted to disk, then drained
//! when the cloud connection is restored.
//!
//! IMPORTANT: We don't queue screen events because they're large (~200KB each)
//! and ephemeral. Queuing them causes O(n) serialization on every enqueue.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::fs;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use super::events::CloudEvent;

/// Maximum number of events to keep in the queue
const MAX_QUEUE_SIZE: usize = 100;

/// Minimum interval between persists (avoid O(n) serialization storm)
const PERSIST_INTERVAL: Duration = Duration::from_secs(10);

/// Queued item with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueuedEvent {
    pub event: CloudEvent,
    pub queued_at: u64,
}

/// Offline event queue with disk persistence
pub struct OfflineQueue {
    queue: VecDeque<QueuedEvent>,
    queue_path: PathBuf,
    max_size: usize,
    /// Last time we persisted to disk (throttled to avoid O(n) storms)
    last_persist: Instant,
    /// Whether queue has been modified since last persist
    dirty: bool,
}

impl OfflineQueue {
    /// Create a new offline queue, loading from disk if exists
    pub fn new() -> Result<Self> {
        let queue_path = Self::queue_path()?;

        let queue = if queue_path.exists() {
            let content = fs::read_to_string(&queue_path).unwrap_or_default();
            serde_json::from_str(&content).unwrap_or_default()
        } else {
            VecDeque::new()
        };

        Ok(Self {
            queue,
            queue_path,
            max_size: MAX_QUEUE_SIZE,
            last_persist: Instant::now(),
            dirty: false,
        })
    }

    /// Get the queue file path
    fn queue_path() -> Result<PathBuf> {
        let home = dirs::home_dir().context("Could not determine home directory")?;
        Ok(home.join(".crabigator").join("offline_queue.json"))
    }

    /// Add an event to the queue
    ///
    /// Note: Screen and scrollback events are skipped - they're large (~200KB) and
    /// ephemeral. Queuing them causes O(n) serialization overhead.
    pub fn enqueue(&mut self, event: CloudEvent) {
        // Skip large ephemeral events - they're not worth queuing
        if matches!(event, CloudEvent::Screen { .. } | CloudEvent::Scrollback { .. }) {
            return;
        }

        // Drop oldest if at capacity
        while self.queue.len() >= self.max_size {
            self.queue.pop_front();
        }

        let queued = QueuedEvent {
            event,
            queued_at: chrono::Utc::now().timestamp_millis() as u64,
        };

        self.queue.push_back(queued);
        self.dirty = true;

        // Throttle persistence to avoid O(n) serialization storm
        if self.last_persist.elapsed() >= PERSIST_INTERVAL {
            let _ = self.persist();
            self.last_persist = Instant::now();
            self.dirty = false;
        }
    }

    /// Drain all events from the queue
    pub fn drain(&mut self) -> Vec<QueuedEvent> {
        let events: Vec<_> = self.queue.drain(..).collect();
        self.dirty = false;
        let _ = self.persist();
        events
    }

    /// Persist if dirty (call on shutdown or connection restore)
    #[allow(dead_code)]
    pub fn flush(&mut self) {
        if self.dirty {
            let _ = self.persist();
            self.dirty = false;
        }
    }

    /// Check if queue is empty
    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }

    /// Get queue length
    pub fn len(&self) -> usize {
        self.queue.len()
    }

    /// Persist queue to disk
    fn persist(&self) -> Result<()> {
        if let Some(parent) = self.queue_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let content = serde_json::to_string(&self.queue)?;
        fs::write(&self.queue_path, content)?;
        Ok(())
    }

    /// Clear the queue and remove the file (for testing)
    #[allow(dead_code)]
    pub fn clear(&mut self) {
        self.queue.clear();
        let _ = fs::remove_file(&self.queue_path);
    }
}

impl Default for OfflineQueue {
    fn default() -> Self {
        Self::new().unwrap_or_else(|_| Self {
            queue: VecDeque::new(),
            queue_path: PathBuf::from("/tmp/crabigator_offline_queue.json"),
            max_size: MAX_QUEUE_SIZE,
            last_persist: Instant::now(),
            dirty: false,
        })
    }
}
