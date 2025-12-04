use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Metadata describing a dataset shard.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatasetShard {
    /// Name of the shard.
    pub name: String,
    /// Number of samples.
    pub samples: usize,
    /// Location reference.
    pub location: String,
    /// Last updated timestamp.
    pub updated_at: DateTime<Utc>,
}

/// Seeker that indexes dataset shards.
#[derive(Debug, Default)]
pub struct DatasetSeeker {
    shards: Vec<DatasetShard>,
}

impl DatasetSeeker {
    /// Registers a shard.
    pub fn register(&mut self, shard: DatasetShard) {
        self.shards.push(shard);
    }

    /// Finds shards that satisfy minimum sample count.
    #[must_use]
    pub fn find(&self, min_samples: usize) -> Vec<DatasetShard> {
        self.shards
            .iter()
            .filter(|shard| shard.samples >= min_samples)
            .cloned()
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seeker_filters_shards() {
        let mut seeker = DatasetSeeker::default();
        seeker.register(DatasetShard {
            name: "A".into(),
            samples: 1_000,
            location: "s3://bucket/a".into(),
            updated_at: Utc::now(),
        });
        assert_eq!(seeker.find(500).len(), 1);
    }
}
