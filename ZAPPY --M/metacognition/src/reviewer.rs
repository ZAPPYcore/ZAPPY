use serde::{Deserialize, Serialize};

use crate::{
    executor::{CommandOutcome, ExecutionInsight},
    metacognition::ReflectionOutcome,
};

/// Reviewer that validates the outcome of metacognitive reflections.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct MetaReviewer;

impl MetaReviewer {
    /// Ensures the outcome meets minimal standards.
    pub fn review(
        &self,
        outcome: &ReflectionOutcome,
        insight: &ExecutionInsight,
    ) -> anyhow::Result<()> {
        if outcome.summary.trim().len() < 16 {
            anyhow::bail!("reflection summary too short");
        }
        if insight.resiliency_score < -0.2 {
            anyhow::bail!("resiliency score indicates regression risk");
        }
        let failures = insight
            .diagnostics
            .iter()
            .filter(|diag| matches!(diag.outcome, CommandOutcome::Failure))
            .count();
        if failures > 2 {
            anyhow::bail!("multiple metacognition commands failed validation");
        }
        Ok(())
    }
}
