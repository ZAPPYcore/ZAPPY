#![deny(
    clippy::all,
    clippy::pedantic,
    clippy::nursery,
    missing_docs,
    rust_2018_idioms
)]

//! Event bus abstractions for module-to-module communication.

use anyhow::Result;
use async_trait::async_trait;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::{fs::OpenOptions, io::AsyncWriteExt, sync::broadcast};

/// Generic event record encoded as JSON.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventRecord {
    /// Unique identifier (uuid).
    pub id: String,
    /// Module producing the event.
    pub source: String,
    /// Event type (e.g., `training.progress`).
    pub event_type: String,
    /// ISO timestamp.
    pub timestamp: String,
    /// Arbitrary JSON payload.
    #[serde(default)]
    pub payload: serde_json::Value,
}

/// Event publisher interface.
#[async_trait]
pub trait EventPublisher: Send + Sync {
    /// Publishes an event to the bus.
    async fn publish(&self, event: EventRecord) -> Result<()>;
}

/// Event subscriber interface.
#[async_trait]
pub trait EventSubscriber: Send + Sync {
    /// Starts consuming events. Implementations should block or stream until channel closes.
    async fn subscribe(&self) -> Result<broadcast::Receiver<EventRecord>>;
}

/// In-memory broadcast bus (for local development and tests).
#[derive(Debug, Clone)]
pub struct MemoryEventBus {
    sender: broadcast::Sender<EventRecord>,
    backlog: Arc<Mutex<VecDeque<EventRecord>>>,
}

impl MemoryEventBus {
    /// Creates a new bus with the given capacity.
    #[must_use]
    pub fn new(capacity: usize) -> Self {
        let (sender, _) = broadcast::channel(capacity);
        Self {
            sender,
            backlog: Arc::new(Mutex::new(VecDeque::with_capacity(capacity))),
        }
    }

    /// Snapshot of recent events retained in memory.
    #[must_use]
    pub fn snapshot(&self) -> Vec<EventRecord> {
        self.backlog.lock().iter().cloned().collect()
    }
}

/// File-backed publisher useful for durable event logs.
#[derive(Debug, Clone)]
pub struct FileEventPublisher {
    path: PathBuf,
}

impl FileEventPublisher {
    /// Creates a publisher that appends JSON lines to the given path.
    pub fn new(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref().to_path_buf();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        Ok(Self { path })
    }
}

#[async_trait]
impl EventPublisher for MemoryEventBus {
    async fn publish(&self, event: EventRecord) -> Result<()> {
        {
            let mut backlog = self.backlog.lock();
            backlog.push_back(event.clone());
            if backlog.len() > backlog.capacity() {
                backlog.pop_front();
            }
        }
        let _ = self.sender.send(event);
        Ok(())
    }
}

#[async_trait]
impl EventSubscriber for MemoryEventBus {
    async fn subscribe(&self) -> Result<broadcast::Receiver<EventRecord>> {
        Ok(self.sender.subscribe())
    }
}

#[async_trait]
impl EventPublisher for FileEventPublisher {
    async fn publish(&self, event: EventRecord) -> Result<()> {
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
            .await?;
        let data = serde_json::to_vec(&event)?;
        file.write_all(&data).await?;
        file.write_all(b"\n").await?;
        file.flush().await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use tokio::runtime::Runtime;

    fn sample_event() -> EventRecord {
        EventRecord {
            id: "event-1".into(),
            source: "tester".into(),
            event_type: "unit.test".into(),
            timestamp: "2025-11-20T00:00:00Z".into(),
            payload: serde_json::json!({"value": 1}),
        }
    }

    #[test]
    fn publishes_and_receives() {
        let rt = Runtime::new().unwrap();
        rt.block_on(async {
            let bus = MemoryEventBus::new(16);
            let mut rx = bus.subscribe().await.unwrap();
            bus.publish(sample_event()).await.unwrap();
            let event = rx.recv().await.unwrap();
            assert_eq!(event.event_type, "unit.test");
        });
    }

    #[test]
    fn file_publisher_writes_events() {
        let rt = Runtime::new().unwrap();
        rt.block_on(async {
            let dir = tempdir().unwrap();
            let path = dir.path().join("events.log");
            let publisher = FileEventPublisher::new(&path).unwrap();
            publisher.publish(sample_event()).await.unwrap();
            let content = std::fs::read_to_string(path).unwrap();
            assert!(content.contains("unit.test"));
        });
    }
}
