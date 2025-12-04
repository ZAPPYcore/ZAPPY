use anyhow::Result;

use crate::{
    advanced::{AdvancedSimulator, ScenarioThinker, SimulationReport},
    helper::SimulationTelemetry,
    methods::SimulationMethod,
    predictor::ScenarioPredictor,
    reviewer::SimulationReviewer,
    simul_env_generator::EnvironmentGenerator,
    simulator::{SimulationBatch, Simulator},
};

/// High-level simulation engine orchestrating base and advanced runs.
pub struct SimulationEngine {
    telemetry: Option<SimulationTelemetry>,
    simulator: Simulator,
    advanced: AdvancedSimulator,
}

impl SimulationEngine {
    /// Returns a builder.
    #[must_use]
    pub fn builder() -> SimulationEngineBuilder {
        SimulationEngineBuilder::default()
    }

    /// Runs a base batch and returns raw results.
    pub async fn run_batch(
        &self,
        method: SimulationMethod,
        count: usize,
    ) -> Result<SimulationBatch> {
        self.simulator.run(method, count).await
    }

    /// Runs advanced pipeline and returns final report.
    pub async fn run_advanced(
        &self,
        method: SimulationMethod,
        count: usize,
    ) -> Result<SimulationReport> {
        self.advanced.run(method, count).await
    }

    /// Returns telemetry handle.
    #[must_use]
    pub fn telemetry(&self) -> Option<&SimulationTelemetry> {
        self.telemetry.as_ref()
    }
}

/// Builder for `SimulationEngine`.
pub struct SimulationEngineBuilder {
    telemetry: Option<SimulationTelemetry>,
    env_seed: u64,
    predictor_noise: f32,
}

impl SimulationEngineBuilder {
    /// Sets telemetry.
    #[must_use]
    pub fn telemetry(mut self, telemetry: SimulationTelemetry) -> Self {
        self.telemetry = Some(telemetry);
        self
    }

    /// Seeds the environment generator.
    #[must_use]
    pub fn env_seed(mut self, seed: u64) -> Self {
        self.env_seed = seed;
        self
    }

    /// Configures predictor noise.
    #[must_use]
    pub fn predictor_noise(mut self, noise: f32) -> Self {
        self.predictor_noise = noise;
        self
    }

    /// Builds the engine.
    pub fn build(self) -> Result<SimulationEngine> {
        let telemetry = self.telemetry;
        let generator = EnvironmentGenerator::new(self.env_seed);
        let predictor = ScenarioPredictor::new(self.predictor_noise);
        let reviewer = SimulationReviewer::new(telemetry.clone());
        let simulator = Simulator::new(generator, predictor, reviewer, telemetry.clone());
        let advanced = AdvancedSimulator::new(
            Simulator::new(
                EnvironmentGenerator::new(self.env_seed + 1),
                ScenarioPredictor::new(self.predictor_noise / 2.0),
                SimulationReviewer::new(telemetry.clone()),
                telemetry.clone(),
            ),
            ScenarioThinker::default(),
            telemetry.clone(),
        );
        Ok(SimulationEngine {
            telemetry,
            simulator,
            advanced,
        })
    }
}

impl Default for SimulationEngineBuilder {
    fn default() -> Self {
        Self {
            telemetry: None,
            env_seed: crate::helper::random_seed(),
            predictor_noise: 0.15,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use shared_event_bus::MemoryEventBus;
    use std::sync::Arc;
    use tempfile::tempdir;

    fn test_telemetry() -> SimulationTelemetry {
        SimulationTelemetry::builder("simulation-engine")
            .log_path(tempdir().unwrap().path().join("sim-engine.log"))
            .event_publisher(Arc::new(MemoryEventBus::new(8)))
            .build()
            .unwrap()
    }

    #[tokio::test]
    async fn engine_runs_batch() {
        let engine = SimulationEngine::builder()
            .telemetry(test_telemetry())
            .build()
            .unwrap();
        let batch = engine
            .run_batch(SimulationMethod::Approximate, 2)
            .await
            .unwrap();
        assert_eq!(batch.scenarios.len(), 2);
    }

    #[tokio::test]
    async fn engine_generates_report() {
        let engine = SimulationEngine::builder().build().unwrap();
        let report = engine
            .run_advanced(SimulationMethod::HighFidelity, 1)
            .await
            .unwrap();
        assert_eq!(report.scenario_count, 1);
    }
}
