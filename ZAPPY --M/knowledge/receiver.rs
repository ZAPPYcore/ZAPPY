use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

use crate::{
    saver::{KnowledgeRecord, KnowledgeStore},
    security::KnowledgeGuard,
};

/// Incoming artifact before normalization.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeArtifact {
    /// Artifact identifier from source system.
    pub external_id: String,
    /// Source label or URL.
    pub source: String,
    /// Title.
    pub title: String,
    /// Raw content.
    pub content: String,
    /// Optional category.
    pub category: Option<String>,
    /// Collected timestamp.
    pub collected_at: DateTime<Utc>,
}

impl KnowledgeArtifact {
    /// Creates a new artifact.
    #[must_use]
    pub fn new(
        source: impl Into<String>,
        title: impl Into<String>,
        content: impl Into<String>,
    ) -> Self {
        Self {
            external_id: Uuid::new_v4().to_string(),
            source: source.into(),
            title: title.into(),
            content: content.into(),
            category: None,
            collected_at: Utc::now(),
        }
    }
}

/// Errors emitted while receiving artifacts.
#[derive(Debug, Error)]
pub enum KnowledgeReceiverError {
    /// Artifact failed validation.
    #[error("validation error: {0}")]
    Validation(String),
    /// Security policy rejected the artifact.
    #[error("security rejection: {0}")]
    Security(String),
}

/// Receives artifacts, validates, and persists them.
#[derive(Debug, Clone)]
pub struct KnowledgeReceiver {
    store: KnowledgeStore,
    guard: KnowledgeGuard,
}

impl KnowledgeReceiver {
    /// Creates a new receiver.
    #[must_use]
    pub fn new(store: KnowledgeStore, guard: KnowledgeGuard) -> Self {
        Self { store, guard }
    }

    /// Processes the artifact, returning persisted record.
    pub fn receive(
        &self,
        artifact: KnowledgeArtifact,
    ) -> Result<KnowledgeRecord, KnowledgeReceiverError> {
        self.validate(&artifact)?;
        self.guard
            .enforce(&artifact)
            .map_err(KnowledgeReceiverError::Security)?;

        let record = KnowledgeRecord::new(&artifact.source, &artifact.title, &artifact.content)
            .with_metadata(
                "collected_at",
                serde_json::json!(artifact.collected_at.to_rfc3339()),
            )
            .with_metadata("category", serde_json::json!(artifact.category))
            .with_external_ref(&artifact.external_id);

        self.store.insert(record.clone());
        Ok(record)
    }

    fn validate(&self, artifact: &KnowledgeArtifact) -> Result<(), KnowledgeReceiverError> {
        if artifact.title.trim().is_empty() {
            return Err(KnowledgeReceiverError::Validation(
                "title cannot be empty".into(),
            ));
        }
        if artifact.content.trim().len() < 15 {
            return Err(KnowledgeReceiverError::Validation(
                "content too short".into(),
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::security::SecurityPolicy;

    #[test]
    fn receiver_persists_record() {
        let store = KnowledgeStore::default();
        let guard = KnowledgeGuard::new(SecurityPolicy::default());
        let receiver = KnowledgeReceiver::new(store.clone(), guard);
        let artifact =
            KnowledgeArtifact::new("web", "Test Title", "This is a sufficiently long body.");
        let record = receiver.receive(artifact).unwrap();
        assert_eq!(record.title, "Test Title");
        assert_eq!(store.len(), 1);
    }
}
