use async_trait::async_trait;

use super::{
    decisionmaking::DecisionDraft,
    reviewer::{DecisionReviewer, ReviewFinding},
};

/// Reviewer that ensures redundancy and continuity considerations.
#[derive(Debug, Clone)]
pub struct ContinuityReviewer;

#[async_trait]
impl DecisionReviewer for ContinuityReviewer {
    fn name(&self) -> &str {
        "continuity"
    }

    async fn review(&self, draft: &DecisionDraft) -> ReviewFinding {
        let sufficient_directives = draft.directives.len() >= 1;
        ReviewFinding {
            reviewer: self.name().into(),
            passed: sufficient_directives && draft.confidence >= 0.4,
            notes: format!(
                "directives={} confidence={:.2}",
                draft.directives.len(),
                draft.confidence
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::decision::decisionmaking::DecisionHypothesis;
    use crate::module::{ControlDirective, DirectivePriority, ModuleKind, ModuleTarget};
    use chrono::Utc;

    fn draft(confidence: f32) -> DecisionDraft {
        DecisionDraft {
            hypothesis: DecisionHypothesis {
                summary: "summary".into(),
                rationale: "rationale".into(),
                risk: 0.2,
            },
            directives: vec![ControlDirective::new(
                ModuleTarget::Kind(ModuleKind::Executor),
                "act",
            )
            .with_priority(DirectivePriority::Routine)],
            confidence,
            generated_at: Utc::now(),
        }
    }

    #[tokio::test]
    async fn fails_low_confidence() {
        let reviewer = ContinuityReviewer;
        let finding = reviewer.review(&draft(0.2)).await;
        assert!(!finding.passed);
    }
}
