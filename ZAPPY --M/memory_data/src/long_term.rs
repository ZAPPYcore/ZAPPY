use std::{fs, path::PathBuf};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

use crate::short_term::{MemoryEntry, MemoryImportance};

/// Long-term memory level. Higher levels imply stronger durability.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum MemoryLevel {
    /// Low-importance retention (level1).
    Level1,
    /// Mid-tier retention.
    Level2,
    /// Strategic retention.
    Level3,
    /// High-value retention.
    Level4,
    /// Mission-critical retention.
    Level5,
}

impl MemoryLevel {
    /// Directory name for the level.
    #[must_use]
    pub fn dir_name(&self) -> &'static str {
        match self {
            Self::Level1 => "level1",
            Self::Level2 => "level2",
            Self::Level3 => "level3",
            Self::Level4 => "level4",
            Self::Level5 => "level5",
        }
    }

    /// Maps from importance to recommended persistence level.
    #[must_use]
    pub fn from_importance(importance: MemoryImportance) -> Self {
        match importance {
            MemoryImportance::Low => Self::Level1,
            MemoryImportance::Medium => Self::Level3,
            MemoryImportance::High => Self::Level5,
        }
    }
}

/// Serializable long-term memory record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredMemory {
    /// Memory identifier.
    pub id: Uuid,
    /// Content.
    pub content: String,
    /// Tags.
    pub tags: Vec<String>,
    /// Importance.
    pub importance: MemoryImportance,
    /// Created timestamp.
    pub created_at: DateTime<Utc>,
    /// Persisted timestamp.
    pub persisted_at: DateTime<Utc>,
}

impl From<MemoryEntry> for StoredMemory {
    fn from(entry: MemoryEntry) -> Self {
        Self {
            id: entry.id,
            content: entry.content,
            tags: entry.tags.iter().cloned().collect(),
            importance: entry.importance,
            created_at: entry.created_at,
            persisted_at: Utc::now(),
        }
    }
}

/// Errors emitted by the long-term storage subsystem.
#[derive(Debug, Error)]
pub enum MemoryStorageError {
    /// Filesystem I/O error.
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    /// Serialization failure.
    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

/// File-system backed long-term memory.
#[derive(Debug, Clone)]
pub struct LongTermMemory {
    base_path: PathBuf,
}

impl LongTermMemory {
    /// Creates a new repository rooted at the provided base directory.
    #[must_use]
    pub fn new(base_path: impl Into<PathBuf>) -> Self {
        Self {
            base_path: base_path.into(),
        }
    }

    /// Persists a memory entry at a specific level.
    pub fn persist(
        &self,
        entry: MemoryEntry,
        level: MemoryLevel,
    ) -> Result<PathBuf, MemoryStorageError> {
        let stored: StoredMemory = entry.into();
        let dir = self.base_path.join("long_term").join(level.dir_name());
        fs::create_dir_all(&dir)?;
        let path = dir.join(format!("{}.json", stored.id));
        let data = serde_json::to_vec_pretty(&stored)?;
        fs::write(&path, data)?;
        Ok(path)
    }

    /// Loads the most recent `limit` memories for the given level.
    #[must_use]
    pub fn load_recent(&self, level: MemoryLevel, limit: usize) -> Vec<StoredMemory> {
        let dir = self.base_path.join("long_term").join(level.dir_name());
        let mut entries = Vec::new();
        if let Ok(read_dir) = fs::read_dir(dir) {
            for entry in read_dir
                .flatten()
                .filter(|e| e.path().extension().map_or(false, |ext| ext == "json"))
            {
                if let Ok(data) = fs::read(entry.path()) {
                    if let Ok(memory) = serde_json::from_slice::<StoredMemory>(&data) {
                        entries.push(memory);
                    }
                }
            }
        }
        entries.sort_by(|a, b| b.persisted_at.cmp(&a.persisted_at));
        entries.truncate(limit);
        entries
    }

    /// Clears all stored memories (primarily used in tests).
    pub fn clear(&self) -> std::io::Result<()> {
        if self.base_path.exists() {
            for entry in fs::read_dir(&self.base_path)? {
                let path = entry?.path();
                if path.is_dir() {
                    fs::remove_dir_all(path)?;
                } else {
                    fs::remove_file(path)?;
                }
            }
        }
        Ok(())
    }
}

impl Default for LongTermMemory {
    fn default() -> Self {
        Self::new(env!("CARGO_MANIFEST_DIR"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::short_term::{MemoryEntry, MemoryImportance};
    use tempfile::tempdir;

    #[test]
    fn persists_and_loads_memories() {
        let dir = tempdir().unwrap();
        let repo = LongTermMemory::new(dir.path());
        let entry = MemoryEntry::new("critical insight", MemoryImportance::High, ["core"]);
        repo.persist(entry, MemoryLevel::Level5).unwrap();
        let memories = repo.load_recent(MemoryLevel::Level5, 10);
        assert_eq!(memories.len(), 1);
        assert_eq!(memories[0].content, "critical insight");
    }
}
