use indexmap::IndexMap;
use rand::Rng;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::simul_env_generator::SimulationScenario;

/// Prediction generated for a scenario.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulationPrediction {
    /// Prediction id.
    pub id: Uuid,
    /// Scenario id.
    pub scenario_id: Uuid,
    /// Projected metrics at the end of simulation.
    pub projected_metrics: IndexMap<String, f32>,
}

/// Predictor capable of producing forward-looking metrics.
pub struct ScenarioPredictor {
    noise: f32,
}

impl ScenarioPredictor {
    /// Creates predictor with configurable noise.
    #[must_use]
    pub fn new(noise: f32) -> Self {
        Self { noise }
    }

    /// Runs predictions for provided scenarios.
    #[must_use]
    pub fn predict(&self, scenarios: &[SimulationScenario]) -> Vec<SimulationPrediction> {
        let mut rng = rand::thread_rng();
        scenarios
            .iter()
            .map(|scenario| {
                let mut metrics = IndexMap::new();
                for (key, value) in &scenario.parameters {
                    let delta = rng.gen_range(-self.noise..self.noise);
                    metrics.insert(key.clone(), (value + delta).clamp(0.0, 1.5));
                }
                SimulationPrediction {
                    id: Uuid::new_v4(),
                    scenario_id: scenario.id,
                    projected_metrics: metrics,
                }
            })
            .collect()
    }
}

impl Default for ScenarioPredictor {
    fn default() -> Self {
        Self::new(0.15)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use indexmap::indexmap;

    #[test]
    fn predictor_generates_metrics() {
        let scenario = SimulationScenario {
            id: Uuid::new_v4(),
            label: "test".into(),
            parameters: indexmap! { "load".into() => 0.5 },
        };
        let predictor = ScenarioPredictor::new(0.1);
        let predictions = predictor.predict(&[scenario]);
        assert_eq!(predictions.len(), 1);
        assert!(predictions[0].projected_metrics.contains_key("load"));
    }
}
