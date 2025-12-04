use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Dataset index describing shards and schema metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatasetIndex {
    /// Unique dataset id aligned with AGI standard JSON schema.
    pub dataset_id: Uuid,
    /// Dataset name.
    pub name: String,
    /// Version string.
    pub version: String,
    /// List of shards.
    pub shards: Vec<DatasetShard>,
    /// Optional schema path for validation.
    pub schema: Option<PathBuf>,
}

impl DatasetIndex {
    /// Validates index invariants.
    pub fn validate(&self) -> anyhow::Result<()> {
        anyhow::ensure!(
            !self.shards.is_empty(),
            "dataset {} has no shards",
            self.name
        );
        Ok(())
    }
}

/// Individual dataset shard entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatasetShard {
    /// Shard id.
    pub id: Uuid,
    /// Relative path for the shard.
    pub path: PathBuf,
    /// Byte size.
    pub bytes: u64,
    /// Priority weight for sampling.
    pub priority: u8,
}
