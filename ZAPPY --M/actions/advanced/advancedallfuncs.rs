use chrono::Duration;
use serde::{Deserialize, Serialize};

use crate::actions::{ActionDomain, ActionPlan, ActionStep};

/// Summary for a scenario that spans multiple domains.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScenarioSummary {
    /// Primary KPI influenced.
    pub kpi: String,
    /// Weighted impact score.
    pub impact_score: f32,
    /// Domains affected.
    pub domains: Vec<ActionDomain>,
}

/// Quantizer that generates scenario summaries for large plans.
#[derive(Debug, Default)]
pub struct ScenarioQuantizer;

impl ScenarioQuantizer {
    /// Builds scenario summaries by clustering steps.
    #[must_use]
    pub fn quantize(&self, plan: &ActionPlan) -> Vec<ScenarioSummary> {
        let mut summaries = Vec::new();
        let chunk_size = (plan.steps.len().max(1) / 3).max(1);
        for chunk in plan.steps.chunks(chunk_size) {
            let domains = chunk.iter().map(|step| step.domain.clone()).collect();
            let impact_score = chunk
                .iter()
                .map(|step| step.estimated_duration.num_minutes())
                .sum::<i64>() as f32
                / 60.0;
            summaries.push(ScenarioSummary {
                kpi: format!(
                    "{}-kpi",
                    chunk.first().map_or("global", |step| step.domain.label())
                ),
                impact_score,
                domains,
            });
        }
        summaries
    }

    /// Produces an "accelerated" mini plan for urgent responses.
    #[must_use]
    pub fn accelerated_plan(&self, plan: &ActionPlan) -> ActionPlan {
        let mut steps = Vec::new();
        for (index, step) in plan.steps.iter().enumerate() {
            if index % 2 == 0 {
                steps.push(ActionStep {
                    ordinal: index,
                    description: format!("Accelerated: {}", step.description),
                    domain: step.domain.clone(),
                    required_capabilities: step.required_capabilities.clone(),
                    estimated_duration: Duration::minutes(5),
                    dependencies: step.dependencies.clone(),
                    instrumentation: step.instrumentation.clone(),
                });
            }
        }

        ActionPlan {
            id: format!("accelerated-{}", plan.id),
            hypothesis: format!("Accelerated({})", plan.hypothesis),
            steps,
            risk: plan.risk.clone(),
        }
    }
}
