use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::predictor::SimulationPrediction;

/// Observations captured after simulation execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulationObservation {
    /// Scenario id.
    pub scenario_id: uuid::Uuid,
    /// Observed metrics.
    pub observed_metrics: IndexMap<String, f32>,
}

/// Comparison result between prediction and observation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComparisonResult {
    /// Scenario id.
    pub scenario_id: uuid::Uuid,
    /// Mean absolute error across metrics.
    pub mae: f32,
    /// Metric-level error map.
    pub per_metric_error: IndexMap<String, f32>,
}

/// Compares predictions to observations.
pub fn compare(
    predictions: &[SimulationPrediction],
    observations: &[SimulationObservation],
) -> Vec<ComparisonResult> {
    let mut results = Vec::new();
    for prediction in predictions {
        if let Some(obs) = observations
            .iter()
            .find(|obs| obs.scenario_id == prediction.scenario_id)
        {
            let mut per_metric = IndexMap::new();
            let mut sum = 0.0;
            let mut count = 0;
            for (key, predicted) in &prediction.projected_metrics {
                if let Some(observed) = obs.observed_metrics.get(key) {
                    let error = (predicted - observed).abs();
                    per_metric.insert(key.clone(), error);
                    sum += error;
                    count += 1;
                }
            }
            let mae = if count > 0 { sum / count as f32 } else { 0.0 };
            results.push(ComparisonResult {
                scenario_id: prediction.scenario_id,
                mae,
                per_metric_error: per_metric,
            });
        }
    }
    results
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::predictor::SimulationPrediction;
    use indexmap::indexmap;
    use uuid::Uuid;

    #[test]
    fn compare_produces_results() {
        let scenario_id = Uuid::new_v4();
        let prediction = SimulationPrediction {
            id: Uuid::new_v4(),
            scenario_id,
            projected_metrics: indexmap! { "load".into() => 0.5 },
        };
        let observation = SimulationObservation {
            scenario_id,
            observed_metrics: indexmap! { "load".into() => 0.7 },
        };
        let results = compare(&[prediction], &[observation]);
        assert_eq!(results.len(), 1);
        assert!(results[0].mae >= 0.0);
    }
}
