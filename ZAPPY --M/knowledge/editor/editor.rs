use serde::{Deserialize, Serialize};

use super::{
    helper::{KnowledgeDiff, SummaryBuilder},
    reviewer::EditReviewer,
};

use crate::saver::{KnowledgeRecord, KnowledgeStore};

/// Edit operation requested for a record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditOperation {
    /// Record identifier.
    pub record_id: uuid::Uuid,
    /// Desired new body text.
    pub new_body: String,
    /// Rationale provided by the editor.
    pub rationale: String,
}

/// Applies edits with review.
#[derive(Debug, Clone)]
pub struct KnowledgeEditor {
    store: KnowledgeStore,
    summarizer: SummaryBuilder,
    reviewer: EditReviewer,
}

impl KnowledgeEditor {
    /// Creates a new editor.
    #[must_use]
    pub fn new(store: KnowledgeStore) -> Self {
        Self {
            store,
            summarizer: SummaryBuilder::default(),
            reviewer: EditReviewer::default(),
        }
    }

    /// Produces a summary for the record.
    pub fn summarize(&self, record: &KnowledgeRecord) -> String {
        self.summarizer.summarize(&record.body, 80)
    }

    /// Applies the edit if it passes review.
    pub fn apply(&self, operation: EditOperation) -> anyhow::Result<KnowledgeRecord> {
        let mut record = self
            .store
            .get(&operation.record_id)
            .ok_or_else(|| anyhow::anyhow!("record not found"))?;

        let diff = KnowledgeDiff {
            before: record.body.clone(),
            after: operation.new_body.clone(),
            rationale: operation.rationale.clone(),
        };
        let decision = self.reviewer.review(&diff);
        if !decision.approved {
            anyhow::bail!("edit rejected: {}", decision.notes);
        }

        record.body = operation.new_body;
        record.metadata.insert(
            "last_edit".into(),
            serde_json::json!({
                "rationale": operation.rationale,
                "timestamp": chrono::Utc::now().to_rfc3339(),
            }),
        );

        self.store.upsert(record.clone());
        Ok(record)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::saver::KnowledgeRecord;

    #[test]
    fn editor_applies_edit() {
        let store = KnowledgeStore::default();
        let record = KnowledgeRecord::new("src", "title", "original content with enough length");
        let id = record.id;
        store.insert(record.clone());
        let editor = KnowledgeEditor::new(store);
        let updated = editor
            .apply(EditOperation {
                record_id: id,
                new_body: "updated body with content".into(),
                rationale: "clarity".into(),
            })
            .unwrap();
        assert_eq!(updated.body, "updated body with content");
    }
}
