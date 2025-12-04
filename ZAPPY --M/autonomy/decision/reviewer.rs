use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use super::decisionmaking::DecisionDraft;

/// Outcome from a reviewer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewFinding {
    /// Reviewer name.
    pub reviewer: String,
    /// Whether the reviewer approved.
    pub passed: bool,
    /// Notes for audit logs.
    pub notes: String,
}

/// Contract implemented by every reviewer.
#[async_trait]
pub trait DecisionReviewer: Send + Sync {
    /// Reviewer human-readable name.
    fn name(&self) -> &str;

    /// Performs the review producing a finding.
    async fn review(&self, draft: &DecisionDraft) -> ReviewFinding;
}

/// Reviewer that enforces governance risk thresholds.
#[derive(Debug, Clone)]
pub struct GovernanceReviewer {
    max_risk: f32,
}

impl GovernanceReviewer {
    /// Creates a new reviewer.
    #[must_use]
    pub fn new(max_risk: f32) -> Self {
        Self { max_risk }
    }
}

#[async_trait]
impl DecisionReviewer for GovernanceReviewer {
    fn name(&self) -> &str {
        "governance"
    }

    async fn review(&self, draft: &DecisionDraft) -> ReviewFinding {
        let passed = draft.hypothesis.risk <= self.max_risk;
        ReviewFinding {
            reviewer: self.name().into(),
            passed,
            notes: if passed {
                "risk acceptable".into()
            } else {
                format!("risk {:.2} above {}", draft.hypothesis.risk, self.max_risk)
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::decisionmaking::{DecisionDraft, DecisionHypothesis};
    use super::*;
    use crate::module::{ControlDirective, DirectivePriority, ModuleKind, ModuleTarget};
    use chrono::Utc;

    fn sample_draft(risk: f32) -> DecisionDraft {
        DecisionDraft {
            hypothesis: DecisionHypothesis {
                summary: "test".into(),
                rationale: "rationale".into(),
                risk,
            },
            directives: vec![ControlDirective::new(
                ModuleTarget::Kind(ModuleKind::Planner),
                "noop",
            )
            .with_priority(DirectivePriority::Routine)],
            confidence: 0.9,
            generated_at: Utc::now(),
        }
    }

    #[tokio::test]
    async fn reviewer_detects_high_risk() {
        let reviewer = GovernanceReviewer::new(0.4);
        let finding = reviewer.review(&sample_draft(0.6)).await;
        assert!(!finding.passed);
    }
}
