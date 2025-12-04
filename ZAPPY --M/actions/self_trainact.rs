use std::sync::Arc;

use async_trait::async_trait;

use crate::actions::{ActionArtifact, ActionError, ActionOutcome, ActionPlan, ActionRequest};

/// Metadata describing a dataset supplied for self-training.
#[derive(Debug, Clone)]
pub struct TrainingDataset {
    /// Identifier or URI.
    pub location: String,
    /// Human readable description.
    pub description: String,
    /// Number of records.
    pub examples: usize,
}

/// Training job specification.
#[derive(Debug, Clone)]
pub struct TrainingJobSpec {
    /// Dataset identifier.
    pub dataset_id: String,
    /// Target capability.
    pub capability: String,
    /// Epoch count.
    pub epochs: u32,
}

/// Status returned after launching training.
#[derive(Debug, Clone)]
pub struct TrainingJobStatus {
    /// Unique job id.
    pub job_id: String,
    /// Estimated completion time in minutes.
    pub eta_minutes: u32,
}

/// Abstraction over concrete training backends.
#[async_trait]
pub trait TrainingInterface: Send + Sync {
    /// Registers a dataset and returns its assigned id.
    async fn register_dataset(&self, dataset: TrainingDataset) -> Result<String, ActionError>;

    /// Schedules a training job.
    async fn launch_training(
        &self,
        spec: TrainingJobSpec,
    ) -> Result<TrainingJobStatus, ActionError>;
}

/// Loopback backend that simulates training locally.
#[derive(Debug, Default)]
pub struct LoopbackTrainingInterface;

#[async_trait]
impl TrainingInterface for LoopbackTrainingInterface {
    async fn register_dataset(&self, dataset: TrainingDataset) -> Result<String, ActionError> {
        Ok(format!(
            "dataset-{}-{}",
            dataset.examples,
            dataset.location.len()
        ))
    }

    async fn launch_training(
        &self,
        spec: TrainingJobSpec,
    ) -> Result<TrainingJobStatus, ActionError> {
        Ok(TrainingJobStatus {
            job_id: format!("job-{}", spec.capability),
            eta_minutes: spec.epochs * 2,
        })
    }
}

/// Executes self-training plans and emits progress artifacts.
#[derive(Clone)]
pub struct SelfTrainingExecutor {
    backend: Arc<dyn TrainingInterface>,
}

impl SelfTrainingExecutor {
    /// Creates a new executor.
    #[must_use]
    pub fn new(backend: Arc<dyn TrainingInterface>) -> Self {
        Self { backend }
    }

    /// Executes the plan.
    pub async fn execute_plan(
        &self,
        request: &ActionRequest,
        plan: &ActionPlan,
    ) -> Result<ActionOutcome, ActionError> {
        let dataset = self.extract_dataset(request)?;
        let dataset_id = self.backend.register_dataset(dataset).await?;
        let spec = TrainingJobSpec {
            dataset_id: dataset_id.clone(),
            capability: request.payload.summary.clone(),
            epochs: 3,
        };
        let status = self.backend.launch_training(spec).await?;

        Ok(ActionOutcome::textual(
            format!("Scheduled training job {}", status.job_id),
            vec![ActionArtifact {
                label: "training_job".into(),
                importance: request.priority,
                content: crate::actions::ArtifactContent::Json(serde_json::json!({
                    "dataset_id": dataset_id,
                    "job_id": status.job_id,
                    "eta_minutes": status.eta_minutes,
                    "steps": plan.steps.len(),
                })),
            }],
        ))
    }

    fn extract_dataset(&self, request: &ActionRequest) -> Result<TrainingDataset, ActionError> {
        let attachment = request
            .payload
            .attachments
            .iter()
            .find(|att| att.label == "training_dataset")
            .ok_or_else(|| ActionError::Invalid("missing training_dataset attachment".into()))?;

        Ok(TrainingDataset {
            location: attachment
                .content
                .get("location")
                .and_then(|v| v.as_str())
                .ok_or_else(|| ActionError::Invalid("dataset location missing".into()))?
                .to_string(),
            description: attachment
                .content
                .get("description")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string(),
            examples: attachment
                .content
                .get("examples")
                .and_then(|v| v.as_u64())
                .unwrap_or(0) as usize,
        })
    }
}
