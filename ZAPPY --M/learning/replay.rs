use std::{collections::HashSet, sync::Arc};

use anyhow::Result;
use chrono::{DateTime, Utc};

use crate::pipeline::{ExperienceArchive, ExperienceHub, PipelineEnvelope};

/// High-level service that exposes replay APIs to downstream modules.
pub struct ExperienceReplayService {
    hub: Arc<ExperienceHub>,
    archive: Option<ExperienceArchive>,
}

impl ExperienceReplayService {
    /// Creates a new service backed by an `ExperienceHub` and optional archive.
    #[must_use]
    pub fn new(hub: Arc<ExperienceHub>, archive: Option<ExperienceArchive>) -> Self {
        Self { hub, archive }
    }

    /// Returns the most recent envelopes merged from archive + in-memory hub.
    pub fn latest(&self, limit: usize) -> Result<Vec<PipelineEnvelope>> {
        let limit = limit.max(1);
        let mut merged = Vec::new();
        if let Some(archive) = &self.archive {
            merged.extend(archive.tail(limit)?);
        }
        merged.extend(self.hub.snapshot(limit));
        Ok(truncate_unique(sort_descending(merged), limit))
    }

    /// Returns envelopes created since the given timestamp.
    pub fn since(&self, since: DateTime<Utc>) -> Result<Vec<PipelineEnvelope>> {
        let mut merged = Vec::new();
        if let Some(archive) = &self.archive {
            merged.extend(archive.since(since)?);
        }
        merged.extend(self.hub.since(since));
        Ok(unique_preserve(sort_ascending(merged)))
    }

    /// Returns the underlying archive, if configured.
    #[must_use]
    pub fn archive(&self) -> Option<&ExperienceArchive> {
        self.archive.as_ref()
    }
}

fn sort_descending(mut events: Vec<PipelineEnvelope>) -> Vec<PipelineEnvelope> {
    events.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
    events
}

fn sort_ascending(mut events: Vec<PipelineEnvelope>) -> Vec<PipelineEnvelope> {
    events.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));
    events
}

fn truncate_unique(events: Vec<PipelineEnvelope>, limit: usize) -> Vec<PipelineEnvelope> {
    let mut seen = HashSet::new();
    let mut deduped = Vec::with_capacity(events.len());
    for event in events {
        if seen.insert(event.id) {
            deduped.push(event);
        }
        if deduped.len() == limit {
            break;
        }
    }
    deduped
}

fn unique_preserve(events: Vec<PipelineEnvelope>) -> Vec<PipelineEnvelope> {
    let mut seen = HashSet::new();
    events
        .into_iter()
        .filter(|event| seen.insert(event.id))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use tempfile::tempdir;

    #[test]
    fn latest_merges_archive_and_hub() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("exp.log");
        let recorder = Arc::new(ExperienceRecorder::new(&path).unwrap());
        let hub = Arc::new(ExperienceHub::new(2).with_recorder(recorder.clone()));
        // Publish 4 entries; hub retains last 2, archive retains all.
        hub.publish("alpha", "event.one", json!({}));
        hub.publish("beta", "event.two", json!({}));
        hub.publish("gamma", "event.three", json!({}));
        hub.publish("delta", "event.four", json!({}));
        let archive = ExperienceArchive::new(&path);
        let service = ExperienceReplayService::new(hub.clone(), Some(archive));
        let latest = service.latest(4).unwrap();
        assert_eq!(latest.len(), 4);
        assert_eq!(latest[0].module, "delta");
        assert_eq!(latest[3].module, "alpha");
    }

    #[test]
    fn since_returns_chronological_unique_records() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("exp.log");
        let recorder = Arc::new(ExperienceRecorder::new(&path).unwrap());
        let hub = Arc::new(ExperienceHub::new(8).with_recorder(recorder.clone()));
        hub.publish("alpha", "event.one", json!({}));
        let boundary = Utc::now();
        hub.publish("beta", "event.two", json!({}));
        let archive = ExperienceArchive::new(&path);
        let service = ExperienceReplayService::new(hub, Some(archive));
        let events = service.since(boundary).unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].module, "beta");
        assert!(events[0].timestamp >= boundary);
    }

    use crate::pipeline::ExperienceRecorder;
}
