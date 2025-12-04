use anyhow::Result;
use serde_json::json;
use tokio::sync::RwLock;

use crate::{
    engine::{InferenceEngine, InferenceResult},
    module::{ReasoningDirective, ReasoningHypothesis, SignalPacket, Verdict},
    multidomain::MultiDomainCoordinator,
    telemetry::ReasoningTelemetry,
};

/// Orchestrates inference + multi-domain review.
pub struct ReasoningRuntime {
    engine: RwLock<InferenceEngine>,
    coordinator: MultiDomainCoordinator,
    telemetry: Option<ReasoningTelemetry>,
}

impl ReasoningRuntime {
    /// Creates a runtime with default components.
    #[must_use]
    pub fn new(telemetry: Option<ReasoningTelemetry>) -> Self {
        let engine = RwLock::new(InferenceEngine::default());
        let coordinator = MultiDomainCoordinator::with_defaults(telemetry.clone());
        Self {
            engine,
            coordinator,
            telemetry,
        }
    }

    /// Runs full reasoning flow.
    pub async fn reason(
        &self,
        directive: ReasoningDirective,
        signals: Vec<SignalPacket>,
    ) -> Result<Verdict> {
        self.log(
            "reasoning.directive.received",
            json!({ "priority": directive.priority.score() }),
        );
        let inference = {
            let mut engine = self.engine.write().await;
            engine.infer(directive, signals)
        };
        let best = self.select_best(inference).await?;
        Ok(best)
    }

    async fn select_best(&self, inference: InferenceResult) -> Result<Verdict> {
        let mut best_hypothesis: Option<ReasoningHypothesis> = None;
        let mut best_score = 0.0;
        for hypothesis in &inference.hypotheses {
            let score = self.coordinator.review(hypothesis).await?;
            if score > best_score {
                best_score = score;
                best_hypothesis = Some(ReasoningHypothesis {
                    confidence: score,
                    ..hypothesis.clone()
                });
            }
        }
        if let Some(h) = &best_hypothesis {
            self.event(
                "reasoning.verdict.hypothesis_selected",
                json!({ "hypothesis_id": h.id, "confidence": h.confidence }),
            );
        } else {
            self.event(
                "reasoning.verdict.none",
                json!({ "directive_id": inference.directive.id }),
            );
        }
        Ok(Verdict {
            directive_id: inference.directive.id,
            hypothesis: best_hypothesis,
            notes: if best_score >= 0.5 {
                "hypothesis accepted".into()
            } else {
                "insufficient confidence".into()
            },
            decided_at: chrono::Utc::now(),
        })
    }

    fn log(&self, message: &str, metadata: serde_json::Value) {
        if let Some(tel) = &self.telemetry {
            let _ = tel.log(shared_logging::LogLevel::Info, message, metadata);
        }
    }

    fn event(&self, event_type: &str, payload: serde_json::Value) {
        if let Some(tel) = &self.telemetry {
            let _ = tel.event(event_type, payload);
        }
    }
}

impl Default for ReasoningRuntime {
    fn default() -> Self {
        Self::new(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::module::{DirectivePriority, ReasoningDirective};

    #[tokio::test]
    async fn runtime_produces_verdict() {
        let runtime = ReasoningRuntime::default();
        let directive = ReasoningDirective::new("Assess anomaly", DirectivePriority::High);
        let signals = vec![
            SignalPacket::new("sensor spike", json!({ "value": 12 })),
            SignalPacket::new("latency jump", json!({ "ms": 300 })),
        ];
        let verdict = runtime.reason(directive, signals).await.unwrap();
        assert_eq!(verdict.hypothesis.is_some(), true);
    }
}
