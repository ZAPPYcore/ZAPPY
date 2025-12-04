use std::{
    collections::VecDeque,
    fs,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Entry describing a shard in the dataset index.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShardRecord {
    /// File name relative to the dataset directory (e.g., `shards/shard-00000.data`).
    pub shard: String,
    /// Number of samples recorded in the shard.
    pub samples: u64,
    /// Importance weight used for prioritised loading.
    pub importance: f32,
}

impl ShardRecord {
    fn effective_priority(&self) -> f32 {
        if self.importance.is_finite() && self.importance > 0.0 {
            self.importance
        } else {
            self.samples as f32
        }
    }
}

/// JSON index describing the dataset shards.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatasetIndex {
    /// Dataset name (matches schema metadata).
    pub name: String,
    /// Path to the dataset root.
    #[serde(skip)]
    pub base_path: PathBuf,
    /// Shard records in arbitrary order.
    pub records: Vec<ShardRecord>,
}

impl DatasetIndex {
    /// Loads an index JSON file from disk.
    pub fn load(index_path: impl AsRef<Path>) -> Result<Self, DataLoaderError> {
        let path = index_path.as_ref();
        let contents = fs::read_to_string(path)?;
        let mut index: DatasetIndex = serde_json::from_str(&contents)?;
        index.base_path = path
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from("."));
        if index.records.is_empty() {
            return Err(DataLoaderError::EmptyIndex);
        }
        Ok(index)
    }
}

/// Batch of shard data loaded into memory.
#[derive(Debug, Clone)]
pub struct ShardBatch {
    /// Metadata describing the shard.
    pub record: ShardRecord,
    /// Binary payload read from disk.
    pub payload: Vec<u8>,
}

/// Prefetching shard loader with importance-aware ordering.
#[derive(Debug)]
pub struct ShardLoader {
    index: DatasetIndex,
    pending: VecDeque<ShardRecord>,
    prefetch: usize,
    queue: VecDeque<ShardBatch>,
}

impl ShardLoader {
    /// Creates a loader from an existing dataset index.
    pub fn from_index(index: DatasetIndex, prefetch: usize) -> Result<Self, DataLoaderError> {
        let mut ordered = index.records.clone();
        ordered.sort_by(|a, b| {
            b.effective_priority()
                .partial_cmp(&a.effective_priority())
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        let pending = VecDeque::from(ordered);
        Ok(Self {
            index,
            pending,
            prefetch: prefetch.max(1),
            queue: VecDeque::new(),
        })
    }

    /// Loads the next batch if available.
    pub fn next(&mut self) -> Result<Option<ShardBatch>, DataLoaderError> {
        self.fill_prefetch()?;
        Ok(self.queue.pop_front())
    }

    fn fill_prefetch(&mut self) -> Result<(), DataLoaderError> {
        while self.queue.len() < self.prefetch {
            if let Some(record) = self.pending.pop_front() {
                let shard_path = self.index.base_path.join(&record.shard);
                let payload = fs::read(&shard_path)?;
                self.queue.push_back(ShardBatch { record, payload });
            } else {
                break;
            }
        }
        Ok(())
    }
}

/// Errors raised by the dataset loader.
#[derive(Debug, Error)]
pub enum DataLoaderError {
    /// Dataset index missing shards.
    #[error("dataset index contains no records")]
    EmptyIndex,
    /// I/O error (filesystem).
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    /// JSON parsing failure.
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use tempfile::tempdir;

    #[test]
    fn loads_shards_in_priority_order() {
        let tmp = tempdir().unwrap();
        let dataset_dir = tmp.path().join("dataset");
        let shards_dir = dataset_dir.join("shards");
        fs::create_dir_all(&shards_dir).unwrap();

        fs::write(shards_dir.join("shard-00000.data"), b"aaaa").unwrap();
        fs::write(shards_dir.join("shard-00001.data"), b"bbbbbb").unwrap();

        let index = json!({
            "name": "sample",
            "records": [
                {"shard": "shards/shard-00000.data", "samples": 10, "importance": 0.2},
                {"shard": "shards/shard-00001.data", "samples": 5, "importance": 0.9}
            ]
        });
        fs::create_dir_all(&dataset_dir).unwrap();
        fs::write(dataset_dir.join("index.json"), index.to_string()).unwrap();

        let mut loader = ShardLoader::from_index(
            DatasetIndex::load(dataset_dir.join("index.json")).unwrap(),
            2,
        )
        .unwrap();

        let first = loader.next().unwrap().unwrap();
        assert_eq!(first.record.shard, "shards/shard-00001.data");
        let second = loader.next().unwrap().unwrap();
        assert_eq!(second.record.shard, "shards/shard-00000.data");
        assert!(loader.next().unwrap().is_none());
    }
}
