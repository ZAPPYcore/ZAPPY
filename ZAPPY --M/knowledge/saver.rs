use chrono::{DateTime, Utc};
use indexmap::IndexMap;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Persistent record stored in the knowledge base.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeRecord {
    /// Unique identifier.
    pub id: Uuid,
    /// Optional external reference identifier (e.g., experience id).
    pub external_ref: Option<String>,
    /// Source system or URL.
    pub source: String,
    /// Canonical title.
    pub title: String,
    /// Body text.
    pub body: String,
    /// Structured metadata for analytics.
    pub metadata: IndexMap<String, serde_json::Value>,
    /// Creation timestamp.
    pub created_at: DateTime<Utc>,
}

impl KnowledgeRecord {
    /// Creates a new record.
    #[must_use]
    pub fn new(
        source: impl Into<String>,
        title: impl Into<String>,
        body: impl Into<String>,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            external_ref: None,
            source: source.into(),
            title: title.into(),
            body: body.into(),
            metadata: IndexMap::new(),
            created_at: Utc::now(),
        }
    }

    /// Attaches an external reference identifier.
    #[must_use]
    pub fn with_external_ref(mut self, reference: impl Into<String>) -> Self {
        self.external_ref = Some(reference.into());
        self
    }

    /// Adds metadata entry.
    #[must_use]
    pub fn with_metadata(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.metadata.insert(key.into(), value);
        self
    }
}

/// Thread-safe knowledge store used by the AGI.
#[derive(Debug, Default, Clone)]
pub struct KnowledgeStore {
    records: std::sync::Arc<RwLock<Vec<KnowledgeRecord>>>,
}

impl KnowledgeStore {
    /// Inserts a record into the store.
    pub fn insert(&self, record: KnowledgeRecord) {
        self.records.write().push(record);
    }

    /// Returns the number of stored records.
    #[must_use]
    pub fn len(&self) -> usize {
        self.records.read().len()
    }

    /// Finds records containing the provided keyword.
    #[must_use]
    pub fn find_by_keyword(&self, keyword: &str) -> Vec<KnowledgeRecord> {
        let keyword = keyword.to_lowercase();
        self.records
            .read()
            .iter()
            .filter(|record| {
                record.title.to_lowercase().contains(&keyword)
                    || record.body.to_lowercase().contains(&keyword)
            })
            .cloned()
            .collect()
    }

    /// Returns the most recent `n` records.
    #[must_use]
    pub fn latest(&self, n: usize) -> Vec<KnowledgeRecord> {
        let mut records = self.records.read().clone();
        records.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        records.truncate(n);
        records
    }

    /// Retrieves a record by id.
    #[must_use]
    pub fn get(&self, id: &Uuid) -> Option<KnowledgeRecord> {
        self.records
            .read()
            .iter()
            .find(|rec| &rec.id == id)
            .cloned()
    }

    /// Updates or inserts a record.
    pub fn upsert(&self, record: KnowledgeRecord) {
        let mut guard = self.records.write();
        if let Some(existing) = guard.iter_mut().find(|rec| rec.id == record.id) {
            *existing = record;
        } else {
            guard.push(record);
        }
    }

    /// Returns true if a record with the given external reference exists.
    #[must_use]
    pub fn contains_external_ref(&self, external_ref: &str) -> bool {
        self.records
            .read()
            .iter()
            .any(|rec| rec.external_ref.as_deref() == Some(external_ref))
    }

    /// Snapshot of all records.
    #[must_use]
    pub fn all(&self) -> Vec<KnowledgeRecord> {
        self.records.read().clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn store_filters_by_keyword() {
        let store = KnowledgeStore::default();
        store.insert(KnowledgeRecord::new("source", "Rust", "Ownership model"));
        store.insert(KnowledgeRecord::new("source", "Python", "Interpreter"));
        let results = store.find_by_keyword("rust");
        assert_eq!(results.len(), 1);
    }
}
