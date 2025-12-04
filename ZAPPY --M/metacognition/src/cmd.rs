use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::methods::ReflectionPlan;

/// Canonical verbs executed during reflection.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ReflectionVerb {
    /// Collect or ingest evidence.
    Collect,
    /// Analyze or correlate data.
    Analyze,
    /// Stress test or simulate futures.
    Simulate,
    /// Debate, summarize, or draft counterfactuals.
    Debate,
    /// Optional fallback or cleanup step.
    Fallback,
    /// Catch-all custom verb.
    Custom,
}

/// Runnable command derived from a reflection plan.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReflectionCommand {
    /// Command identifier.
    pub id: Uuid,
    /// Verb classification.
    pub verb: ReflectionVerb,
    /// Human-readable description.
    pub description: String,
    /// Weight/importance in range 0-1.
    pub weight: f32,
}

impl ReflectionCommand {
    /// Creates a new command.
    #[must_use]
    pub fn new(verb: ReflectionVerb, description: impl Into<String>, weight: f32) -> Self {
        Self {
            id: Uuid::new_v4(),
            verb,
            description: description.into(),
            weight: weight.clamp(0.05, 1.0),
        }
    }
}

/// Synthesizes commands from a reflection plan.
#[derive(Debug, Default)]
pub struct CommandSynthesizer;

impl CommandSynthesizer {
    /// Generates a deterministic command list from plan steps.
    pub fn synthesize(plan: &ReflectionPlan) -> Vec<ReflectionCommand> {
        let severity = plan.observation.severity.max(0.1);
        let total_steps = plan.steps.len().max(1) as f32;
        plan.steps
            .iter()
            .enumerate()
            .map(|(idx, step)| {
                let position = (idx as f32 + 1.0) / total_steps;
                let verb = classify_step(step);
                let weight = (0.3 * severity) + (0.3 * position);
                ReflectionCommand::new(verb, step.clone(), weight)
            })
            .collect()
    }
}

fn classify_step(step: &str) -> ReflectionVerb {
    let lowered = step.to_lowercase();
    if lowered.contains("gather") || lowered.contains("collect") || lowered.contains("ingest") {
        ReflectionVerb::Collect
    } else if lowered.contains("summarize")
        || lowered.contains("analy")
        || lowered.contains("align")
        || lowered.contains("correlate")
    {
        ReflectionVerb::Analyze
    } else if lowered.contains("stress") || lowered.contains("simulate") || lowered.contains("risk")
    {
        ReflectionVerb::Simulate
    } else if lowered.contains("counterfactual")
        || lowered.contains("debate")
        || lowered.contains("plan")
    {
        ReflectionVerb::Debate
    } else if lowered.contains("fallback") || lowered.contains("cleanup") {
        ReflectionVerb::Fallback
    } else {
        ReflectionVerb::Custom
    }
}
