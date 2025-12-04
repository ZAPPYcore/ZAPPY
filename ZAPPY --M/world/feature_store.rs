use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use chrono::Utc;
use parking_lot::Mutex;
use serde_json::json;
use uuid::Uuid;

use crate::{infoseeker::InfoSignal, learning::AssimilationJob};

/// File-backed feature store for replaying world signals.
#[derive(Debug)]
pub struct FeatureStore {
    path: Option<PathBuf>,
    writer: Option<Mutex<std::fs::File>>,
}

impl FeatureStore {
    /// Opens (or creates) a feature store at the given path.
    pub fn open(path: impl Into<PathBuf>) -> Result<Self> {
        let path = path.into();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("creating feature store dir {}", parent.display()))?;
        }
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .with_context(|| format!("opening feature store {}", path.display()))?;
        Ok(Self {
            path: Some(path),
            writer: Some(Mutex::new(file)),
        })
    }

    /// Returns a disabled store (no-op writer).
    #[must_use]
    pub fn disabled() -> Self {
        Self {
            path: None,
            writer: None,
        }
    }

    /// Persists raw signals for future learning.
    pub fn persist_signals(&self, batch_id: &Uuid, signals: &[InfoSignal]) -> Result<()> {
        if let Some(writer) = &self.writer {
            let mut guard = writer.lock();
            for signal in signals {
                let record = json!({
                    "batch_id": batch_id,
                    "ts": Utc::now(),
                    "region": signal.region_id,
                    "severity": signal.severity,
                    "metrics": signal.metrics,
                });
                serde_json::to_writer(&mut *guard, &record)?;
                guard.write_all(b"\n")?;
            }
            guard.flush()?;
        }
        Ok(())
    }

    /// Persists the aggregation job metadata.
    pub fn persist_job(&self, job: &AssimilationJob) -> Result<()> {
        if let Some(writer) = &self.writer {
            let mut guard = writer.lock();
            let record = json!({
                "batch_id": job.batch_id,
                "ts": Utc::now(),
                "regions": job.region_metrics.keys().cloned().collect::<Vec<_>>(),
            });
            serde_json::to_writer(&mut *guard, &record)?;
            guard.write_all(b"\n")?;
            guard.flush()?;
        }
        Ok(())
    }

    /// Returns the configured path, if enabled.
    #[must_use]
    pub fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use indexmap::IndexMap;
    use serde_json::json;
    use tempfile::tempdir;

    #[test]
    fn writes_records() {
        let dir = tempdir().unwrap();
        let store_path = dir.path().join("features.jsonl");
        let store = FeatureStore::open(&store_path).unwrap();
        let signals = vec![InfoSignal {
            region_id: "alpha".into(),
            metrics: json!({ "load": 0.9 }),
            severity: 0.91,
        }];
        let batch_id = Uuid::new_v4();
        store.persist_signals(&batch_id, &signals).unwrap();
        let mut job_regions = IndexMap::new();
        job_regions.insert("alpha".into(), json!({ "load": 0.9 }));
        store
            .persist_job(&AssimilationJob {
                batch_id,
                region_metrics: job_regions,
            })
            .unwrap();
        let content = fs::read_to_string(store_path).unwrap();
        assert!(content.contains("alpha"));
    }
}
