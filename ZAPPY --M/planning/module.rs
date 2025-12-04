use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Priority tiers for directives flowing into the planning runtime.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum PriorityBand {
    /// Low-priority / exploratory.
    Low,
    /// Medium priority.
    Medium,
    /// Highest priority / urgent.
    High,
}

impl PriorityBand {
    /// Converts to percentile score.
    #[must_use]
    pub fn as_score(self) -> u8 {
        match self {
            Self::Low => 20,
            Self::Medium => 60,
            Self::High => 90,
        }
    }
}

/// Signal describing an environmental change that may require re-planning.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanningSignal {
    /// Unique signal id.
    pub id: Uuid,
    /// Human readable description.
    pub narrative: String,
    /// Impact score (0-100).
    pub impact: u8,
}

impl PlanningSignal {
    /// Creates a new signal instance.
    #[must_use]
    pub fn new(narrative: impl Into<String>, impact: u8) -> Self {
        Self {
            id: Uuid::new_v4(),
            narrative: narrative.into(),
            impact,
        }
    }
}

/// Planning directive referencing priority and objectives.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanningDirective {
    /// Associated signal (if any).
    pub signal: Option<PlanningSignal>,
    /// Requested priority.
    pub priority: PriorityBand,
    /// Objective description.
    pub objective: String,
}

impl PlanningDirective {
    /// Convenience helper.
    #[must_use]
    pub fn critical(objective: impl Into<String>) -> Self {
        Self {
            signal: None,
            priority: PriorityBand::High,
            objective: objective.into(),
        }
    }
}
