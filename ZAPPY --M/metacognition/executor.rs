use std::collections::VecDeque;

use rand::{seq::SliceRandom, thread_rng};
use serde::{Deserialize, Serialize};

use crate::{
    cmd::{ReflectionCommand, ReflectionVerb},
    methods::{ReflectionMethod, ReflectionPlan},
};

/// Aggregated insight emitted after executing reflection commands.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ExecutionInsight {
    /// Normalized resiliency score (-1.0..1.0).
    pub resiliency_score: f32,
    /// Command-level diagnostics.
    pub diagnostics: Vec<CommandInsight>,
}

/// Per-command diagnostic.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandInsight {
    /// Underlying verb.
    pub verb: ReflectionVerb,
    /// Status.
    pub outcome: CommandOutcome,
    /// Weighted impact on resiliency.
    pub impact: f32,
    /// Human-readable note.
    pub note: String,
    /// Command description.
    pub description: String,
}

/// Possible command outcomes.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum CommandOutcome {
    /// Execution succeeded.
    Success,
    /// Execution failed.
    Failure,
    /// Execution skipped/deferred.
    Skipped,
}

impl CommandOutcome {
    fn label(self) -> &'static str {
        match self {
            Self::Success => "success",
            Self::Failure => "failure",
            Self::Skipped => "skipped",
        }
    }
}

/// Executor responsible for running reflection commands with adaptive heuristics.
#[derive(Debug, Default)]
pub struct ReflectionExecutor;

impl ReflectionExecutor {
    /// Executes commands and emits structured insights.
    pub fn execute(plan: &ReflectionPlan, commands: &[ReflectionCommand]) -> ExecutionInsight {
        let mut diagnostics = Vec::with_capacity(commands.len());
        let mut queue: VecDeque<_> = commands.iter().collect();
        let mut rng = thread_rng();
        let method_weight = match plan.method {
            ReflectionMethod::RapidReview => 0.85,
            ReflectionMethod::StructuredAnalysis => 1.0,
            ReflectionMethod::ComprehensiveAudit => 1.15,
        };
        let mut resiliency = 0.0f32;

        while let Some(command) = queue.pop_front() {
            let (outcome, impact, note) = match command.verb {
                ReflectionVerb::Collect => (
                    CommandOutcome::Success,
                    0.12 * command.weight,
                    "Collected supporting evidence.".to_string(),
                ),
                ReflectionVerb::Analyze => (
                    CommandOutcome::Success,
                    0.18 * command.weight,
                    "Analyzed multi-signal context.".to_string(),
                ),
                ReflectionVerb::Simulate => (
                    CommandOutcome::Success,
                    0.22 * command.weight,
                    "Stress-tested hypotheses via simulation.".to_string(),
                ),
                ReflectionVerb::Debate => (
                    CommandOutcome::Success,
                    0.15 * command.weight,
                    "Debated counterfactual narratives.".to_string(),
                ),
                ReflectionVerb::Fallback => (
                    CommandOutcome::Skipped,
                    0.05 * command.weight,
                    "Fallback command deferred: no longer needed.".to_string(),
                ),
                ReflectionVerb::Custom => {
                    let outcome = [CommandOutcome::Success, CommandOutcome::Failure]
                        .choose(&mut rng)
                        .copied()
                        .unwrap_or(CommandOutcome::Success);
                    let impact = if matches!(outcome, CommandOutcome::Success) {
                        0.1 * command.weight
                    } else {
                        -0.12 * command.weight
                    };
                    let note = format!("Custom verb executed with {}.", outcome.label());
                    (outcome, impact, note)
                }
            };

            resiliency += impact;
            diagnostics.push(CommandInsight {
                verb: command.verb,
                outcome,
                impact,
                note,
                description: command.description.clone(),
            });
        }

        ExecutionInsight {
            resiliency_score: (resiliency * method_weight).clamp(-1.0, 1.0),
            diagnostics,
        }
    }
}
