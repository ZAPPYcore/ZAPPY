use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::simulator::SimulationBatch;

/// Insight extracted from simulation batch.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScenarioInsight {
    /// Scenario label.
    pub label: String,
    /// Key finding.
    pub finding: String,
}

/// Produces insights from simulation batches.
pub struct ScenarioThinker;

impl ScenarioThinker {
    /// Creates thinker.
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// Analyzes batch and emits insights.
    pub fn analyze(&self, batch: &SimulationBatch) -> Result<Vec<ScenarioInsight>> {
        let mut insights = Vec::new();
        for scenario in &batch.scenarios {
            let load = scenario.parameters.get("load").copied().unwrap_or_default();
            let finding = if load > 0.8 {
                "high_load".to_string()
            } else {
                "nominal".to_string()
            };
            insights.push(ScenarioInsight {
                label: scenario.label.clone(),
                finding,
            });
        }
        Ok(insights)
    }
}

impl Default for ScenarioThinker {
    fn default() -> Self {
        Self::new()
    }
}
