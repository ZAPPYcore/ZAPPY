use indexmap::IndexMap;
use rand::Rng;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::helper::{random_seed, seeded_rng};

/// Scenario describing initial environment conditions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulationScenario {
    /// Scenario id.
    pub id: Uuid,
    /// Human readable label.
    pub label: String,
    /// Parameters such as load/traffic.
    pub parameters: IndexMap<String, f32>,
}

/// Generates simulation scenarios using seeded randomness.
pub struct EnvironmentGenerator {
    seed: u64,
}

impl EnvironmentGenerator {
    /// Creates generator with seed.
    #[must_use]
    pub fn new(seed: u64) -> Self {
        Self { seed }
    }

    /// Generates a set of scenarios.
    #[must_use]
    pub fn generate(&self, count: usize) -> Vec<SimulationScenario> {
        let mut rng = seeded_rng(self.seed);
        (0..count)
            .map(|idx| {
                let mut params = IndexMap::new();
                params.insert("load".into(), rng.gen_range(0.2..0.95));
                params.insert("latency".into(), rng.gen_range(15.0..180.0));
                params.insert("traffic".into(), rng.gen_range(0.1..0.9));
                SimulationScenario {
                    id: Uuid::new_v4(),
                    label: format!("scenario-{}", idx),
                    parameters: params,
                }
            })
            .collect()
    }
}

impl Default for EnvironmentGenerator {
    fn default() -> Self {
        Self::new(random_seed())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generator_produces_scenarios() {
        let generator = EnvironmentGenerator::new(42);
        let scenarios = generator.generate(2);
        assert_eq!(scenarios.len(), 2);
        assert!(scenarios[0].parameters.contains_key("load"));
    }
}
