//! Deep learning orchestration pipeline.

/// Batch building utilities.
pub mod editor;
/// Model definitions.
pub mod model;
/// Training reporters.
pub mod reporter;
/// Snapshot persistence.
pub mod savor;
/// Architecture search tools.
pub mod search;
/// Dataset seekers.
pub mod seek;
/// Training loops.
pub mod trainer;

use std::path::PathBuf;

use model::DenseModel;
use reporter::DlReport;
use savor::SnapshotSavor;
use search::ArchitectureSearch;
use serde_json::json;
use shared_logging::LogLevel;
use trainer::{Trainer, TrainingConfig};

use crate::telemetry::LearningTelemetry;

/// Deep learning pipeline orchestrator.
#[derive(Debug)]
pub struct DeepLearningPipeline {
    trainer: Trainer,
}

impl Default for DeepLearningPipeline {
    fn default() -> Self {
        Self {
            trainer: Trainer::new(TrainingConfig::default()),
        }
    }
}

impl DeepLearningPipeline {
    /// Runs architecture search, training, and reporting.
    pub fn run(&mut self) -> anyhow::Result<DlReport> {
        self.run_with_telemetry(None)
    }

    /// Runs pipeline with optional telemetry.
    pub fn run_with_telemetry(
        &mut self,
        telemetry: Option<&LearningTelemetry>,
    ) -> anyhow::Result<DlReport> {
        let mut search = ArchitectureSearch::default();
        let candidates = search.run(5);
        if let Some(tel) = telemetry {
            let _ = tel.log(
                LogLevel::Info,
                "deep_learning_architecture_search",
                json!({ "candidates": candidates.len() }),
            );
        }
        let best = &candidates[0];
        let weights_path =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("dataset/dense_weights.json");
        let mut model = DenseModel::from_dataset_file(weights_path)?;
        if let Some(tel) = telemetry {
            let _ = tel.log(
                LogLevel::Info,
                "deep_learning_training_start",
                json!({ "hidden_dim": best.hidden_dim, "output_dim": best.hidden_dim / 2 }),
            );
        }
        let snapshots = self.trainer.train(&mut model);
        let mut savor = SnapshotSavor::new(3);
        for snapshot in snapshots.clone() {
            savor.store(snapshot);
        }
        let report = DlReport {
            experiment: format!("layers-{}", best.layers),
            snapshots,
        };
        if let Some(tel) = telemetry {
            let best_loss = report.best().map(|snap| snap.val_loss);
            let _ = tel.log(
                LogLevel::Info,
                "deep_learning_training_complete",
                json!({ "experiment": report.experiment, "best_loss": best_loss }),
            );
            let _ = tel.event(
                "learning.deep.report",
                json!({ "experiment": report.experiment, "best_loss": best_loss }),
            );
        }
        Ok(report)
    }
}
