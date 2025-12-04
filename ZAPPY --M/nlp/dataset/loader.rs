use std::{fs, path::Path};

use anyhow::{Context, Result};
use rand::{seq::SliceRandom, SeedableRng};
use rand_chacha::ChaCha8Rng;

use super::{DatasetIndex, DatasetShard};

/// Bytes loaded from a dataset shard.
#[derive(Debug)]
pub struct LoadedShard {
    /// Shard metadata.
    pub shard: DatasetShard,
    /// Raw bytes.
    pub data: Vec<u8>,
}

/// Loader capable of sampling shards with priority weighting.
pub struct DatasetLoader {
    base_path: String,
    rng: ChaCha8Rng,
}

impl DatasetLoader {
    /// Creates a loader root at the dataset path.
    #[must_use]
    pub fn new(base_path: impl Into<String>) -> Self {
        Self {
            base_path: base_path.into(),
            rng: ChaCha8Rng::from_entropy(),
        }
    }

    /// Loads the dataset index from disk.
    pub fn load_index(&self, path: &Path) -> Result<DatasetIndex> {
        let data = fs::read_to_string(path).with_context(|| format!("reading {:?}", path))?;
        let index: DatasetIndex = serde_json::from_str(&data)?;
        index.validate()?;
        Ok(index)
    }

    /// Samples shards honoring `priority`.
    pub fn sample_shards(
        &mut self,
        index: &DatasetIndex,
        count: usize,
    ) -> Result<Vec<LoadedShard>> {
        let mut shards = index.shards.clone();
        shards.sort_by(|a, b| b.priority.cmp(&a.priority));
        let limit = count.min(shards.len());
        let candidate_window = (limit * 2).max(limit);
        let mut candidates: Vec<_> = shards.into_iter().take(candidate_window).collect();
        candidates.shuffle(&mut self.rng);
        let mut selection = Vec::new();

        for shard in candidates.into_iter().take(limit) {
            let path = Path::new(&self.base_path).join(&shard.path);
            let bytes = fs::read(&path)
                .with_context(|| format!("reading shard {:?} (dataset {})", path, index.name))?;
            selection.push(LoadedShard { shard, data: bytes });
        }
        Ok(selection)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use tempfile::tempdir;
    use uuid::Uuid;

    #[test]
    fn loader_reads_index_and_shards() {
        let dir = tempdir().unwrap();
        let shard_path = dir.path().join("shard-1.bin");
        fs::write(&shard_path, b"hello world").unwrap();
        let index_path = dir.path().join("index.json");
        let shard_id = Uuid::new_v4();
        let index = json!({
            "dataset_id": Uuid::new_v4(),
            "name": "test",
            "version": "1.0",
            "schema": null,
            "shards": [{
                "id": shard_id,
                "path": shard_path
                    .file_name()
                    .unwrap()
                    .to_string_lossy(),
                "bytes": 11,
                "priority": 90
            }]
        });
        fs::write(&index_path, serde_json::to_vec(&index).unwrap()).unwrap();

        let loader = DatasetLoader::new(dir.path().to_string_lossy().to_string());
        let ds_index = loader.load_index(&index_path).unwrap();
        let mut loader = loader;
        let shards = loader.sample_shards(&ds_index, 1).unwrap();
        assert_eq!(shards.len(), 1);
        assert_eq!(shards[0].data, b"hello world");
    }
}
