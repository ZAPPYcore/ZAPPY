//! Decision orchestration for the autonomy kernel.

/// Core decision engine primitives.
pub mod decisionmaking;
/// Governance reviewer implementations.
pub mod reviewer;
/// Additional resilience reviewers.
pub mod reviewerno2;

use std::{fmt, sync::Arc};

use decisionmaking::{build_input, DecisionEngine, DecisionHypothesis, DecisionInput};
use reviewer::{DecisionReviewer, GovernanceReviewer, ReviewFinding};
use reviewerno2::ContinuityReviewer;
use serde::{Deserialize, Serialize};
use serde_json::json;
use shared_logging::LogLevel;

use crate::{
    module::{AutonomyError, AutonomySignal, ModuleBroker, ModuleRegistry},
    telemetry::AutonomyTelemetry,
};

/// Outcome produced after all reviewers have run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionVerdict {
    /// Hypothesis that survived review.
    pub hypothesis: DecisionHypothesis,
    /// Approved directives.
    pub directives: Vec<crate::module::ControlDirective>,
    /// Individual reviewer notes.
    pub findings: Vec<ReviewFinding>,
    /// Confidence adjusted by reviewer outcomes.
    pub confidence: f32,
}

/// Directs the decision engine and reviewers.
#[derive(Clone)]
pub struct DecisionDirector {
    engine: DecisionEngine,
    reviewers: Vec<Arc<dyn DecisionReviewer>>,
    registry: ModuleRegistry,
    telemetry: Option<AutonomyTelemetry>,
}

impl fmt::Debug for DecisionDirector {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DecisionDirector")
            .field("reviewers", &self.reviewers.len())
            .finish()
    }
}

impl DecisionDirector {
    /// Creates a director with default reviewers.
    #[must_use]
    pub fn new(registry: ModuleRegistry) -> Self {
        Self {
            engine: DecisionEngine::default(),
            reviewers: vec![
                Arc::new(GovernanceReviewer::new(0.55)),
                Arc::new(ContinuityReviewer),
            ],
            registry,
            telemetry: None,
        }
    }

    /// Adds an additional reviewer.
    pub fn with_reviewer(mut self, reviewer: Arc<dyn DecisionReviewer>) -> Self {
        self.reviewers.push(reviewer);
        self
    }

    /// Attaches telemetry sinks.
    #[must_use]
    pub fn with_telemetry(mut self, telemetry: AutonomyTelemetry) -> Self {
        self.telemetry = Some(telemetry);
        self
    }

    /// Evaluates a signal end-to-end, returning a verdict.
    pub async fn decide_signal(
        &self,
        signal: AutonomySignal,
    ) -> Result<DecisionVerdict, AutonomyError> {
        let input = build_input(signal, &self.registry);
        if let Some(tel) = &self.telemetry {
            let scope = format!("{:?}", input.signal.scope);
            let metrics = input.signal.metrics.clone();
            let _ = tel.log(
                LogLevel::Info,
                "autonomy.signal.received",
                json!({ "scope": scope, "metrics": metrics }),
            );
            let _ = tel.event(
                "autonomy.signal.received",
                json!({ "scope": scope, "metrics": metrics }),
            );
        }
        if let Some(tel) = &self.telemetry {
            let scope = format!("{:?}", input.signal.scope);
            let metrics = input.signal.metrics.clone();
            let _ = tel.log(
                LogLevel::Info,
                "autonomy.signal.received",
                json!({ "scope": scope, "metrics": metrics }),
            );
            let _ = tel.event(
                "autonomy.signal.received",
                json!({ "scope": scope, "metrics": metrics }),
            );
        }
        self.decide(input).await
    }

    /// Evaluates the provided input.
    pub async fn decide(&self, input: DecisionInput) -> Result<DecisionVerdict, AutonomyError> {
        let draft = self.engine.evaluate(&input)?;
        if let Some(tel) = &self.telemetry {
            let _ = tel.log(
                LogLevel::Info,
                "autonomy.decision.draft",
                json!({
                    "hypothesis": draft.hypothesis.summary,
                    "directives": draft.directives.len(),
                    "confidence": draft.confidence
                }),
            );
        }
        let mut findings = Vec::new();
        for reviewer in &self.reviewers {
            findings.push(reviewer.review(&draft).await);
        }

        let approved = findings.iter().all(|finding| finding.passed);
        if !approved {
            if let Some(tel) = &self.telemetry {
                let _ = tel.log(
                    LogLevel::Warn,
                    "autonomy.decision.rejected",
                    json!({ "findings": findings }),
                );
                let _ = tel.event(
                    "autonomy.decision.rejected",
                    json!({ "findings": findings }),
                );
            }
            return Err(AutonomyError::Internal(
                "decision rejected by reviewers".into(),
            ));
        }

        let confidence_penalty = findings
            .iter()
            .filter(|finding| !finding.notes.contains("acceptable"))
            .count() as f32
            * 0.05;

        let verdict = DecisionVerdict {
            hypothesis: draft.hypothesis,
            directives: draft.directives,
            findings,
            confidence: (draft.confidence - confidence_penalty).clamp(0.0, 1.0),
        };
        if let Some(tel) = &self.telemetry {
            let _ = tel.log(
                LogLevel::Info,
                "autonomy.decision.approved",
                json!({
                    "hypothesis": verdict.hypothesis.summary,
                    "directives": verdict.directives.len(),
                    "confidence": verdict.confidence
                }),
            );
            let _ = tel.event(
                "autonomy.decision.approved",
                json!({
                    "hypothesis": verdict.hypothesis.summary,
                    "directives": verdict.directives.len(),
                    "confidence": verdict.confidence
                }),
            );
        }
        Ok(verdict)
    }
}

/// Convenience constructor bundling the broker registry.
#[must_use]
pub fn build_director(broker: &ModuleBroker) -> DecisionDirector {
    DecisionDirector::new(broker.registry())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::module::{AutonomySignal, ModuleKind, ModuleSpec, SignalScope};

    #[tokio::test]
    async fn director_returns_verdict() {
        let registry = ModuleRegistry::default();
        registry.upsert(ModuleSpec::new("planner", ModuleKind::Planner));
        let signal = AutonomySignal::new(SignalScope::Global, "steady").with_metric("load", 0.3);
        let director = DecisionDirector::new(registry);
        let verdict = director.decide_signal(signal).await.unwrap();
        assert!(verdict.confidence > 0.0);
    }
}
