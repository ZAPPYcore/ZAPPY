use std::sync::Arc;

use anyhow::Result;

/// Domain implementations for action pipelines.
pub mod actions;
/// Advanced/causal reasoning reviewers.
pub mod advanced;
/// Shared trait definitions for domains.
pub mod domain;
/// Helper functions for multi-domain aggregation.
pub mod helper;
/// Hypothesis review orchestration.
pub mod reviewer;

use crate::{module::ReasoningHypothesis, telemetry::ReasoningTelemetry};
use actions::ActionsDomain;
use advanced::CausalDomain;
use domain::ReasoningDomain;
use reviewer::HypothesisReviewer;

/// Coordinates multi-domain reasoning reviews.
pub struct MultiDomainCoordinator {
    reviewer: HypothesisReviewer,
}

impl MultiDomainCoordinator {
    /// Builds a coordinator with default domains.
    #[must_use]
    pub fn with_defaults(telemetry: Option<ReasoningTelemetry>) -> Self {
        let domains: Vec<Arc<dyn ReasoningDomain>> = vec![
            Arc::new(ActionsDomain),
            Arc::new(CausalDomain::new("causal")),
        ];
        Self {
            reviewer: HypothesisReviewer::new(domains, telemetry),
        }
    }

    /// Reviews a hypothesis and returns aggregate confidence.
    pub async fn review(&self, hypothesis: &ReasoningHypothesis) -> Result<f32> {
        self.reviewer.review(hypothesis).await
    }
}
