use std::{
    collections::VecDeque,
    fs::{self, OpenOptions},
    io::{BufRead, BufReader, Write},
    path::{Path, PathBuf},
    sync::Arc,
};

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use parking_lot::{Mutex, RwLock};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

/// Envelope used for sharing cross-module learning experiences.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineEnvelope {
    /// Unique identifier for the record.
    pub id: Uuid,
    /// Module originating the event.
    pub module: String,
    /// Signal or topic name.
    pub signal: String,
    /// Payload serialized as JSON.
    pub payload: Value,
    /// Creation timestamp.
    pub timestamp: DateTime<Utc>,
}

/// Experience hub storing recent records for replay and online learning.
pub struct ExperienceHub {
    records: RwLock<VecDeque<PipelineEnvelope>>,
    capacity: usize,
    recorder: Option<Arc<ExperienceRecorder>>,
}

impl ExperienceHub {
    /// Creates a new hub with the desired capacity.
    #[must_use]
    pub fn new(capacity: usize) -> Self {
        Self {
            records: RwLock::new(VecDeque::with_capacity(capacity)),
            capacity: capacity.max(1),
            recorder: None,
        }
    }

    /// Attaches a recorder that persists envelopes.
    #[must_use]
    pub fn with_recorder(mut self, recorder: Arc<ExperienceRecorder>) -> Self {
        self.recorder = Some(recorder);
        self
    }

    /// Sets/overrides recorder after construction.
    pub fn set_recorder(&mut self, recorder: Arc<ExperienceRecorder>) {
        self.recorder = Some(recorder);
    }

    /// Publishes a record and returns the stored envelope.
    pub fn publish(
        &self,
        module: impl Into<String>,
        signal: impl Into<String>,
        payload: Value,
    ) -> PipelineEnvelope {
        let envelope = PipelineEnvelope {
            id: Uuid::new_v4(),
            module: module.into(),
            signal: signal.into(),
            payload,
            timestamp: Utc::now(),
        };
        let mut records = self.records.write();
        if records.len() == self.capacity {
            records.pop_front();
        }
        records.push_back(envelope.clone());
        if let Some(recorder) = &self.recorder {
            if let Err(err) = recorder.persist(&envelope) {
                eprintln!("experience recorder failed: {err:?}");
            }
        }
        envelope
    }

    /// Returns the most recent `limit` envelopes (newest first).
    #[must_use]
    pub fn snapshot(&self, limit: usize) -> Vec<PipelineEnvelope> {
        let records = self.records.read();
        records.iter().rev().take(limit).cloned().collect()
    }

    /// Returns envelopes created since the provided timestamp.
    #[must_use]
    pub fn since(&self, since: DateTime<Utc>) -> Vec<PipelineEnvelope> {
        let records = self.records.read();
        records
            .iter()
            .filter(|record| record.timestamp >= since)
            .cloned()
            .collect()
    }
}

/// Durable recorder that writes envelopes to JSONL.
#[derive(Debug)]
pub struct ExperienceRecorder {
    path: PathBuf,
    writer: Mutex<std::fs::File>,
}

impl ExperienceRecorder {
    /// Opens or creates a recorder at the provided path.
    pub fn new(path: impl Into<PathBuf>) -> Result<Self> {
        let path = path.into();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).with_context(|| {
                format!("creating experience recorder dir {}", parent.display())
            })?;
        }
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .with_context(|| format!("opening experience recorder {}", path.display()))?;
        Ok(Self {
            path,
            writer: Mutex::new(file),
        })
    }

    /// Persists an envelope.
    pub fn persist(&self, envelope: &PipelineEnvelope) -> Result<()> {
        let mut writer = self.writer.lock();
        serde_json::to_writer(&mut *writer, envelope)?;
        writer.write_all(b"\n")?;
        writer.flush()?;
        Ok(())
    }

    /// Returns recorder path.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }
}

/// Reader for archived experiences stored on disk.
#[derive(Debug, Clone)]
pub struct ExperienceArchive {
    path: PathBuf,
}

impl ExperienceArchive {
    /// Creates an archive reader for the given path.
    #[must_use]
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    /// Returns the latest `limit` envelopes from disk.
    pub fn tail(&self, limit: usize) -> Result<Vec<PipelineEnvelope>> {
        let mut events = self.read_all()?;
        if events.len() > limit {
            events.drain(0..events.len() - limit);
        }
        Ok(events)
    }

    /// Returns envelopes recorded since the provided timestamp.
    pub fn since(&self, since: DateTime<Utc>) -> Result<Vec<PipelineEnvelope>> {
        let events = self.read_all()?;
        Ok(events
            .into_iter()
            .filter(|env| env.timestamp >= since)
            .collect())
    }

    fn read_all(&self) -> Result<Vec<PipelineEnvelope>> {
        if !self.path.exists() {
            return Ok(Vec::new());
        }
        let file = std::fs::File::open(&self.path)
            .with_context(|| format!("opening experience archive {}", self.path.display()))?;
        let reader = BufReader::new(file);
        let mut events = Vec::new();
        for line in reader.lines() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }
            let env = serde_json::from_str::<PipelineEnvelope>(&line)
                .with_context(|| "failed to deserialize pipeline envelope")?;
            events.push(env);
        }
        Ok(events)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use tempfile::tempdir;

    #[test]
    fn hub_drops_old_records() {
        let hub = ExperienceHub::new(2);
        hub.publish("a", "sig", json!({}));
        hub.publish("b", "sig", json!({}));
        hub.publish("c", "sig", json!({}));
        let recent = hub.snapshot(3);
        assert_eq!(recent.len(), 2);
        assert_eq!(recent[0].module, "c");
        assert_eq!(recent[1].module, "b");
    }

    #[test]
    fn hub_filters_by_timestamp() {
        let hub = ExperienceHub::new(4);
        hub.publish("a", "sig", json!({}));
        let boundary = Utc::now();
        hub.publish("b", "sig", json!({}));
        let events = hub.since(boundary);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].module, "b");
    }

    #[test]
    fn recorder_persists_and_archive_replays() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("experience.log");
        let recorder = Arc::new(ExperienceRecorder::new(&path).unwrap());
        let hub = ExperienceHub::new(8).with_recorder(recorder.clone());
        hub.publish("alpha", "event.one", json!({ "value": 1 }));
        hub.publish("beta", "event.two", json!({ "value": 2 }));
        let archive = ExperienceArchive::new(&path);
        let tail = archive.tail(10).unwrap();
        assert_eq!(tail.len(), 2);
        assert_eq!(tail[0].module, "alpha");
        assert_eq!(tail[1].module, "beta");
        let since = tail[1].timestamp;
        let recent = archive.since(since).unwrap();
        assert_eq!(recent.len(), 1);
        assert_eq!(recent[0].signal, "event.two");
    }
}
