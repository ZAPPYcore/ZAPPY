use serde::{Deserialize, Serialize};

use super::helper::KnowledgeDiff;

/// Result from reviewing an edit.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewDecision {
    /// Approved flag.
    pub approved: bool,
    /// Notes for auditing.
    pub notes: String,
}

/// Reviewer that enforces quality checks.
#[derive(Debug, Default, Clone)]
pub struct EditReviewer;

impl EditReviewer {
    /// Reviews the diff ensuring the edited content is not empty.
    #[must_use]
    pub fn review(&self, diff: &KnowledgeDiff) -> ReviewDecision {
        if diff.after.trim().is_empty() {
            return ReviewDecision {
                approved: false,
                notes: "edited content empty".into(),
            };
        }
        if diff.after.len() < diff.before.len() / 4 {
            return ReviewDecision {
                approved: false,
                notes: "excessive truncation detected".into(),
            };
        }
        ReviewDecision {
            approved: true,
            notes: "changes approved".into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reviewer_rejects_empty() {
        let reviewer = EditReviewer::default();
        let decision = reviewer.review(&KnowledgeDiff {
            before: "text".into(),
            after: "".into(),
            rationale: "test".into(),
        });
        assert!(!decision.approved);
    }
}
