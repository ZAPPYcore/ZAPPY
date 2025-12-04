//! Classical machine learning pipeline modules.

/// Dataset editing utilities.
pub mod editor;
/// Feature engineering helpers.
pub mod func;
/// Model implementations.
pub mod ml;
/// Reporting utilities.
pub mod reporter;
/// Submodel management for ensembles.
pub mod submodel;

use std::path::PathBuf;

use editor::Dataset;
use ml::LinearRegressionModel;
use reporter::TrainingReport;
use serde_json::json;
use shared_logging::LogLevel;

use crate::telemetry::LearningTelemetry;

/// End-to-end classical ML pipeline orchestrator.
#[derive(Debug, Default)]
pub struct ClassicalMlPipeline;

impl ClassicalMlPipeline {
    /// Runs the pipeline and returns a training report.
    pub fn run(&self, dataset: Dataset) -> anyhow::Result<TrainingReport> {
        self.run_with_telemetry(dataset, None)
    }

    /// Runs the pipeline with optional telemetry instrumentation.
    pub fn run_with_telemetry(
        &self,
        mut dataset: Dataset,
        telemetry: Option<&LearningTelemetry>,
    ) -> anyhow::Result<TrainingReport> {
        log(
            telemetry,
            LogLevel::Info,
            "classical_ml_standardize",
            json!({ "samples": dataset.samples.len(), "feature_dim": dataset.feature_dim() }),
        );
        dataset.standardize();
        let weights_path =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("dataset/linear_weights.json");
        let mut model = LinearRegressionModel::from_dataset_file(weights_path)?;
        log(
            telemetry,
            LogLevel::Debug,
            "classical_ml_training_start",
            json!({ "epochs": 10, "learning_rate": 0.05 }),
        );
        let mse = model.fit(&dataset, 0.05, 10);
        let report = TrainingReport {
            model: "linear_regression".into(),
            mse,
            epochs: 10,
        };
        log(
            telemetry,
            LogLevel::Info,
            "classical_ml_training_complete",
            json!({ "mse": report.mse, "epochs": report.epochs }),
        );
        if let Some(telemetry) = telemetry {
            let _ = telemetry.event(
                "learning.classical.report",
                json!({ "mse": report.mse, "epochs": report.epochs }),
            );
        }
        Ok(report)
    }
}

fn log(
    telemetry: Option<&LearningTelemetry>,
    level: LogLevel,
    message: &str,
    metadata: serde_json::Value,
) {
    if let Some(tel) = telemetry {
        let _ = tel.log(level, message, metadata);
    }
}
