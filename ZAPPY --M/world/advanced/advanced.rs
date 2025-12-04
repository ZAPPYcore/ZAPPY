use anyhow::Result;
use indexmap::IndexMap;

use super::{
    advmodel::PredictiveModel,
    reviewer::StateReviewer,
    train::{Trainer, TrainingArtifact, TrainingConfig},
};
use crate::{model::WorldState, telemetry::WorldTelemetry};

/// High-level controller wiring predictive models + reviewer.
pub struct AdvancedController {
    model: PredictiveModel,
    reviewer: StateReviewer,
    trainer: Trainer,
    telemetry: Option<WorldTelemetry>,
}

impl AdvancedController {
    /// Creates a controller from baseline metrics.
    #[must_use]
    pub fn new(baseline: IndexMap<String, f32>, telemetry: Option<WorldTelemetry>) -> Self {
        let reviewer = StateReviewer::new(telemetry.clone());
        let trainer = Trainer::new(telemetry.clone());
        Self {
            model: PredictiveModel::new(baseline),
            reviewer,
            trainer,
            telemetry,
        }
    }

    /// Scores incoming metrics for a region.
    pub fn score_metrics(&mut self, metrics: &IndexMap<String, f32>) -> f32 {
        let score = self.model.update(metrics);
        if let Some(tel) = &self.telemetry {
            let _ = tel.log(
                shared_logging::LogLevel::Debug,
                "world.advanced.score",
                serde_json::json!({ "score": score }),
            );
        }
        score
    }

    /// Reviews full world state and returns whether action is required.
    pub fn review_state(&self, state: &WorldState) -> Result<bool> {
        self.reviewer.review(state)
    }

    /// Launches offline training.
    pub async fn retrain(&self, config: TrainingConfig) -> Result<TrainingArtifact> {
        self.trainer.train(config).await
    }
}
