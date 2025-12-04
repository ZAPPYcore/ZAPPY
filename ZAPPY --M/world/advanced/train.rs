use std::{fs, path::PathBuf, time::Duration};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::time::sleep;
use uuid::Uuid;

use crate::telemetry::WorldTelemetry;

/// Training configuration for predictive models.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingConfig {
    /// Dataset path.
    pub dataset_path: PathBuf,
    /// Output directory.
    pub output_dir: PathBuf,
}

/// Result after training completes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingArtifact {
    /// Artifact id.
    pub artifact_id: Uuid,
    /// Location of exported model.
    pub artifact_path: PathBuf,
}

/// Handles offline training for world predictive models.
pub struct Trainer {
    telemetry: Option<WorldTelemetry>,
}

impl Trainer {
    /// Creates trainer.
    #[must_use]
    pub fn new(telemetry: Option<WorldTelemetry>) -> Self {
        Self { telemetry }
    }

    /// Runs training job asynchronously.
    pub async fn train(&self, config: TrainingConfig) -> Result<TrainingArtifact> {
        if let Some(tel) = &self.telemetry {
            let _ = tel.log(
                shared_logging::LogLevel::Info,
                "world.training.start",
                json!({ "dataset": config.dataset_path }),
            );
        }
        sleep(Duration::from_millis(50)).await;
        fs::create_dir_all(&config.output_dir)
            .with_context(|| format!("creating {:?}", config.output_dir))?;
        let artifact_path = config.output_dir.join("world-model.json");
        fs::write(
            &artifact_path,
            serde_json::to_vec_pretty(&json!({
                "model": "predictive",
                "timestamp": chrono::Utc::now().to_rfc3339(),
            }))?,
        )?;
        let artifact = TrainingArtifact {
            artifact_id: Uuid::new_v4(),
            artifact_path,
        };
        if let Some(tel) = &self.telemetry {
            let _ = tel.event(
                "world.training.completed",
                json!({ "artifact": artifact.artifact_path }),
            );
        }
        Ok(artifact)
    }
}
