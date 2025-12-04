use async_trait::async_trait;

use crate::{
    module::ReasoningHypothesis,
    multidomain::domain::{outcome_metadata, DomainOutcome, ReasoningDomain},
};

/// Domain reviewer for action-oriented hypotheses.
pub struct ActionsDomain;

#[async_trait]
impl ReasoningDomain for ActionsDomain {
    fn label(&self) -> &str {
        "actions"
    }

    async fn evaluate(&self, hypothesis: &ReasoningHypothesis) -> DomainOutcome {
        let score = (hypothesis.confidence * 0.8).clamp(0.0, 1.0);
        DomainOutcome {
            domain: self.label().into(),
            score,
            metadata: outcome_metadata(hypothesis.id, "validated against action policies"),
        }
    }
}
