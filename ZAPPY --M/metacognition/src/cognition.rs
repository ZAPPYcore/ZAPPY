use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Self-observation captured by metacognitive monitors.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelfObservation {
    /// Unique identifier for correlation.
    pub id: Uuid,
    /// Natural-language description of the observation.
    pub description: String,
    /// Severity indicator (0-1).
    pub severity: f32,
    /// Timestamp when the observation was recorded.
    pub recorded_at: DateTime<Utc>,
}

impl SelfObservation {
    /// Creates a new observation.
    #[must_use]
    pub fn new(description: impl Into<String>, severity: f32) -> Self {
        Self {
            id: Uuid::new_v4(),
            description: description.into(),
            severity: severity.clamp(0.0, 1.0),
            recorded_at: Utc::now(),
        }
    }
}
