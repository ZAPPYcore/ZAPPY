use anyhow::Result;
use serde_json::json;

use crate::{compare::ComparisonResult, helper::SimulationTelemetry};

/// Reviewer that inspects comparison results and raises alerts.
pub struct SimulationReviewer {
    telemetry: Option<SimulationTelemetry>,
    mae_threshold: f32,
}

impl SimulationReviewer {
    /// Creates reviewer.
    #[must_use]
    pub fn new(telemetry: Option<SimulationTelemetry>) -> Self {
        Self {
            telemetry,
            mae_threshold: 0.2,
        }
    }

    /// Reviews results and returns failing scenario ids.
    pub fn review(&self, results: &[ComparisonResult]) -> Result<Vec<uuid::Uuid>> {
        let failing: Vec<_> = results
            .iter()
            .filter(|res| res.mae >= self.mae_threshold)
            .map(|res| res.scenario_id)
            .collect();
        if let Some(tel) = &self.telemetry {
            let _ = tel.event(
                "simulation.review.complete",
                json!({
                    "total": results.len(),
                    "failing": failing.len(),
                }),
            );
        }
        Ok(failing)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use indexmap::indexmap;
    use uuid::Uuid;

    #[test]
    fn reviewer_detects_failures() {
        let results = vec![ComparisonResult {
            scenario_id: Uuid::new_v4(),
            mae: 0.3,
            per_metric_error: indexmap! {},
        }];
        let reviewer = SimulationReviewer::new(None);
        let failing = reviewer.review(&results).unwrap();
        assert_eq!(failing.len(), 1);
    }
}
