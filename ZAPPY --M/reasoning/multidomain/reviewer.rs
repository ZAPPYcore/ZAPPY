use std::sync::Arc;

use anyhow::Result;
use futures::future::join_all;

use crate::{
    module::ReasoningHypothesis,
    multidomain::{
        domain::{DomainOutcome, ReasoningDomain},
        helper::{aggregate_confidence, telemetry_payload},
    },
    telemetry::ReasoningTelemetry,
};

/// Reviews hypotheses by dispatching to domains and aggregating scores.
pub struct HypothesisReviewer {
    domains: Vec<Arc<dyn ReasoningDomain>>,
    telemetry: Option<ReasoningTelemetry>,
}

impl HypothesisReviewer {
    /// Creates a new reviewer from domain implementations.
    #[must_use]
    pub fn new(
        domains: Vec<Arc<dyn ReasoningDomain>>,
        telemetry: Option<ReasoningTelemetry>,
    ) -> Self {
        Self { domains, telemetry }
    }

    /// Runs the review pipeline.
    pub async fn review(&self, hypothesis: &ReasoningHypothesis) -> Result<f32> {
        let futures = self
            .domains
            .iter()
            .map(|domain| {
                let domain = Arc::clone(domain);
                async move { domain.evaluate(hypothesis).await }
            })
            .collect::<Vec<_>>();
        let outcomes: Vec<DomainOutcome> = join_all(futures).await;
        let scores: Vec<f32> = outcomes.iter().map(|o| o.score).collect();
        let aggregate = aggregate_confidence(hypothesis, &scores);
        if let Some(tel) = &self.telemetry {
            let payload = telemetry_payload(hypothesis, aggregate);
            let _ = tel.log(
                shared_logging::LogLevel::Info,
                "reasoning.hypothesis.reviewed",
                payload.clone(),
            );
            let _ = tel.event("reasoning.hypothesis.reviewed", payload);
        }
        Ok(aggregate)
    }
}
