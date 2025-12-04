use async_trait::async_trait;
use serde_json::Value;
use uuid::Uuid;

use crate::module::ReasoningHypothesis;

/// Outcome returned by domain evaluations.
#[derive(Debug, Clone)]
pub struct DomainOutcome {
    /// Domain name.
    pub domain: String,
    /// Score 0-1 after evaluation.
    pub score: f32,
    /// Additional metadata.
    pub metadata: Value,
}

/// Trait implemented by domain-specific reasoning reviewers.
#[async_trait]
pub trait ReasoningDomain: Send + Sync {
    /// Returns the domain label.
    fn label(&self) -> &str;

    /// Evaluate a hypothesis asynchronously.
    async fn evaluate(&self, hypothesis: &ReasoningHypothesis) -> DomainOutcome;
}

/// Utility for building synthetic outcome metadata.
pub fn outcome_metadata(hypothesis_id: Uuid, rationale: &str) -> Value {
    serde_json::json!({
        "hypothesis_id": hypothesis_id,
        "rationale": rationale,
    })
}
