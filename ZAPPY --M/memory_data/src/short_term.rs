use chrono::{DateTime, Utc};
use indexmap::IndexSet;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use uuid::Uuid;

/// Importance of a memory entry. Used to choose retention and persistence strategy.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum MemoryImportance {
    /// Routine observations; lowest retention.
    Low,
    /// Default level for contextual facts.
    Medium,
    /// Highly critical memories that must be persisted.
    High,
}

/// Single memory entry stored in short-term memory.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEntry {
    /// Unique identifier.
    pub id: Uuid,
    /// Human-readable description/content.
    pub content: String,
    /// Timestamp when captured.
    pub created_at: DateTime<Utc>,
    /// Arbitrary tags for search/routing.
    pub tags: IndexSet<String>,
    /// Importance level.
    pub importance: MemoryImportance,
}

impl MemoryEntry {
    /// Creates a new entry.
    #[must_use]
    pub fn new(
        content: impl Into<String>,
        importance: MemoryImportance,
        tags: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        let mut tag_set = IndexSet::new();
        for tag in tags {
            tag_set.insert(tag.into());
        }
        Self {
            id: Uuid::new_v4(),
            content: content.into(),
            created_at: Utc::now(),
            tags: tag_set,
            importance,
        }
    }
}

/// Short-term memory implemented as a rolling buffer with tag-based queries.
#[derive(Debug)]
pub struct ShortTermMemory {
    capacity: usize,
    entries: RwLock<VecDeque<MemoryEntry>>,
}

impl Clone for ShortTermMemory {
    fn clone(&self) -> Self {
        let snapshot = self.entries.read().clone();
        Self {
            capacity: self.capacity,
            entries: RwLock::new(snapshot),
        }
    }
}

impl ShortTermMemory {
    /// Creates a new short-term memory with the given capacity.
    #[must_use]
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity: capacity.max(1),
            entries: RwLock::new(VecDeque::new()),
        }
    }

    /// Returns the number of stored entries.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.read().len()
    }

    /// Adds an entry to memory, evicting the oldest entry if capacity is exceeded.
    pub fn push(&self, entry: MemoryEntry) {
        let mut entries = self.entries.write();
        entries.push_back(entry);
        if entries.len() > self.capacity {
            entries.pop_front();
        }
    }

    /// Returns a snapshot of all entries.
    #[must_use]
    pub fn snapshot(&self) -> Vec<MemoryEntry> {
        self.entries.read().iter().cloned().collect()
    }

    /// Searches for entries containing the specified tag.
    #[must_use]
    pub fn search_by_tag(&self, tag: &str) -> Vec<MemoryEntry> {
        let tag_lower = tag.to_lowercase();
        self.entries
            .read()
            .iter()
            .filter(|entry| entry.tags.iter().any(|t| t.to_lowercase() == tag_lower))
            .cloned()
            .collect()
    }

    /// Drains entries matching the predicate (used when persisting to long-term storage).
    pub fn drain_filter<F>(&self, mut predicate: F) -> Vec<MemoryEntry>
    where
        F: FnMut(&MemoryEntry) -> bool,
    {
        let mut entries = self.entries.write();
        let mut drained = Vec::new();
        let mut retained = VecDeque::with_capacity(entries.len());
        while let Some(entry) = entries.pop_front() {
            if predicate(&entry) {
                drained.push(entry);
            } else {
                retained.push_back(entry);
            }
        }
        *entries = retained;
        drained
    }
}

impl Default for ShortTermMemory {
    fn default() -> Self {
        Self::new(256)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn short_term_memories_eviction() {
        let memory = ShortTermMemory::new(2);
        memory.push(MemoryEntry::new(
            "a",
            MemoryImportance::Low,
            Vec::<&str>::new(),
        ));
        memory.push(MemoryEntry::new(
            "b",
            MemoryImportance::Low,
            Vec::<&str>::new(),
        ));
        memory.push(MemoryEntry::new(
            "c",
            MemoryImportance::Low,
            Vec::<&str>::new(),
        ));
        let snapshot = memory.snapshot();
        assert_eq!(snapshot.len(), 2);
        assert!(snapshot.iter().any(|entry| entry.content == "b"));
        assert!(snapshot.iter().any(|entry| entry.content == "c"));
    }

    #[test]
    fn search_by_tag_finds_entries() {
        let memory = ShortTermMemory::new(4);
        memory.push(MemoryEntry::new(
            "systems update",
            MemoryImportance::Medium,
            ["ops", "infra"],
        ));
        let matches = memory.search_by_tag("Ops");
        assert_eq!(matches.len(), 1);
    }
}
