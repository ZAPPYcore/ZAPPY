use anyhow::Result;
use serde_json::json;
use tokio::time::{sleep, Duration};

use crate::{
    compare::{compare, SimulationObservation},
    helper::SimulationTelemetry,
    methods::SimulationMethod,
    predictor::{ScenarioPredictor, SimulationPrediction},
    reviewer::SimulationReviewer,
    simul_env_generator::{EnvironmentGenerator, SimulationScenario},
};

/// Result after running a simulation batch.
pub struct SimulationBatch {
    /// Scenarios executed.
    pub scenarios: Vec<SimulationScenario>,
    /// Predictions produced.
    pub predictions: Vec<SimulationPrediction>,
    /// Observations recorded.
    pub observations: Vec<SimulationObservation>,
}

/// Simulator orchestrates scenario generation, prediction, and comparison.
pub struct Simulator {
    generator: EnvironmentGenerator,
    predictor: ScenarioPredictor,
    reviewer: SimulationReviewer,
    telemetry: Option<SimulationTelemetry>,
}

impl Simulator {
    /// Creates a simulator.
    #[must_use]
    pub fn new(
        generator: EnvironmentGenerator,
        predictor: ScenarioPredictor,
        reviewer: SimulationReviewer,
        telemetry: Option<SimulationTelemetry>,
    ) -> Self {
        Self {
            generator,
            predictor,
            reviewer,
            telemetry,
        }
    }

    /// Runs a single batch.
    pub async fn run(&self, method: SimulationMethod, count: usize) -> Result<SimulationBatch> {
        if let Some(tel) = &self.telemetry {
            let _ = tel.log(
                shared_logging::LogLevel::Info,
                "simulation.batch.start",
                json!({ "method": method.label(), "count": count }),
            );
        }
        let scenarios = self.generator.generate(count);
        let predictions = self.predictor.predict(&scenarios);
        let observations = self.execute_observations(&predictions, method).await?;
        let comparisons = compare(&predictions, &observations);
        let failing = self.reviewer.review(&comparisons)?;
        if let Some(tel) = &self.telemetry {
            let _ = tel.event(
                "simulation.batch.completed",
                json!({ "failing": failing.len(), "method": method.label() }),
            );
        }
        Ok(SimulationBatch {
            scenarios,
            predictions,
            observations,
        })
    }

    async fn execute_observations(
        &self,
        predictions: &[SimulationPrediction],
        method: SimulationMethod,
    ) -> Result<Vec<SimulationObservation>> {
        let mut observations = Vec::new();
        for prediction in predictions {
            sleep(Duration::from_millis(10 * method.step_multiplier() as u64)).await;
            let mut observed = prediction.projected_metrics.clone();
            for value in observed.values_mut() {
                *value = (*value + rand::random::<f32>() * 0.05).clamp(0.0, 1.5);
            }
            observations.push(SimulationObservation {
                scenario_id: prediction.scenario_id,
                observed_metrics: observed,
            });
        }
        Ok(observations)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::helper::SimulationTelemetry;
    use shared_event_bus::MemoryEventBus;
    use std::sync::Arc;
    use tempfile::tempdir;

    #[tokio::test]
    async fn simulator_runs_batch() {
        let telemetry = SimulationTelemetry::builder("simulator")
            .log_path(tempdir().unwrap().path().join("sim.log"))
            .event_publisher(Arc::new(MemoryEventBus::new(8)))
            .build()
            .ok();
        let simulator = Simulator::new(
            EnvironmentGenerator::default(),
            ScenarioPredictor::default(),
            SimulationReviewer::new(telemetry.clone()),
            telemetry,
        );
        let batch = simulator
            .run(SimulationMethod::Approximate, 2)
            .await
            .unwrap();
        assert_eq!(batch.scenarios.len(), 2);
    }
}
