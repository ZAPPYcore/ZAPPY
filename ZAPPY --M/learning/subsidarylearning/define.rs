use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Task handled by the subsidiary learning loop.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubsidiaryTask {
    /// Identifier.
    pub id: Uuid,
    /// Domain (e.g., "supply_chain").
    pub domain: String,
    /// Description.
    pub objective: String,
    /// Priority level.
    pub priority: u8,
    /// Creation timestamp.
    pub created_at: DateTime<Utc>,
}

impl SubsidiaryTask {
    /// Creates a new task.
    #[must_use]
    pub fn new(domain: impl Into<String>, objective: impl Into<String>, priority: u8) -> Self {
        Self {
            id: Uuid::new_v4(),
            domain: domain.into(),
            objective: objective.into(),
            priority,
            created_at: Utc::now(),
        }
    }
}

/// Plan mapping tasks to specific submodels.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubsidiaryPlan {
    /// Task reference.
    pub task_id: Uuid,
    /// Selected submodel.
    pub submodel_id: Uuid,
    /// Notes.
    pub notes: String,
}
