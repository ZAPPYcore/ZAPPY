use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::module::{
    AutonomyError, AutonomySignal, ControlDirective, DirectivePriority, ModuleKind, ModuleRegistry,
    ModuleSpec, ModuleTarget,
};

/// Context for a decision cycle.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionContext {
    /// Unique cycle identifier.
    pub cycle_id: Uuid,
    /// Optional operator supplied hint.
    pub operator_hint: Option<String>,
}

impl Default for DecisionContext {
    fn default() -> Self {
        Self {
            cycle_id: Uuid::new_v4(),
            operator_hint: None,
        }
    }
}

/// Input provided to the decision engine.
#[derive(Debug, Clone)]
pub struct DecisionInput {
    /// Latest autonomy signal.
    pub signal: AutonomySignal,
    /// Registry snapshot.
    pub registry_snapshot: Vec<ModuleSpec>,
    /// Execution context.
    pub context: DecisionContext,
}

/// Statement describing the recommended course of action.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionHypothesis {
    /// Narrative summary.
    pub summary: String,
    /// Supporting rationale.
    pub rationale: String,
    /// Estimated risk between 0 and 1.
    pub risk: f32,
}

/// Draft produced by the decision engine prior to review.
#[derive(Debug, Clone)]
pub struct DecisionDraft {
    /// Hypothesis being proposed.
    pub hypothesis: DecisionHypothesis,
    /// Proposed directives.
    pub directives: Vec<ControlDirective>,
    /// Confidence level between 0 and 1.
    pub confidence: f32,
    /// Timestamp of generation.
    pub generated_at: DateTime<Utc>,
}

/// Deterministic engine that transforms signals into drafts.
#[derive(Debug, Clone)]
pub struct DecisionEngine {
    threshold: f64,
}

impl DecisionEngine {
    /// Creates a new engine with the provided load threshold.
    #[must_use]
    pub fn new(threshold: f64) -> Self {
        Self { threshold }
    }

    /// Evaluates a decision input producing a draft.
    pub fn evaluate(&self, input: &DecisionInput) -> Result<DecisionDraft, AutonomyError> {
        let load = input.signal.metric("load").unwrap_or(0.3);
        let risk = (load / self.threshold).clamp(0.0, 1.0) as f32;
        let confidence = (1.0 - risk).clamp(0.0, 1.0);

        let summary = if load > self.threshold {
            "Scale capacity to maintain SLOs"
        } else {
            "Maintain current configuration"
        };

        let rationale = format!(
            "load={:.2}, modules={} :: hint={:?}",
            load,
            input.registry_snapshot.len(),
            input.context.operator_hint
        );

        let target_kind = if load > self.threshold {
            ModuleKind::Executor
        } else {
            ModuleKind::Planner
        };

        let directive = ControlDirective::new(ModuleTarget::Kind(target_kind.clone()), summary)
            .with_priority(if load > self.threshold {
                DirectivePriority::Elevated
            } else {
                DirectivePriority::Routine
            });

        Ok(DecisionDraft {
            hypothesis: DecisionHypothesis {
                summary: summary.into(),
                rationale,
                risk,
            },
            directives: vec![directive],
            confidence,
            generated_at: Utc::now(),
        })
    }
}

impl Default for DecisionEngine {
    fn default() -> Self {
        Self::new(0.65)
    }
}

/// Utility for constructing inputs from the current registry.
pub fn build_input(signal: AutonomySignal, registry: &ModuleRegistry) -> DecisionInput {
    DecisionInput {
        signal,
        registry_snapshot: registry.snapshot(),
        context: DecisionContext::default(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::module::{ModuleRegistry, ModuleSpec, SignalScope};

    #[test]
    fn produces_draft() {
        let registry = ModuleRegistry::default();
        registry.upsert(ModuleSpec::new("planner", ModuleKind::Planner));
        let signal =
            AutonomySignal::new(SignalScope::Global, "load spike").with_metric("load", 0.7);
        let input = build_input(signal, &registry);
        let engine = DecisionEngine::default();
        let draft = engine.evaluate(&input).unwrap();
        assert_eq!(draft.directives.len(), 1);
    }
}
