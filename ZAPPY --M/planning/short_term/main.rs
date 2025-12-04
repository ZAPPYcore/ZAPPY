use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::long_term::StrategicPlan;

/// Scheduling engine implementation.
pub mod engine;
/// Helper functions supporting scheduling.
pub mod helper;
/// Scheduling methods and enums.
pub mod methods;

pub use engine::ScheduleEngine;
pub use methods::TacticalMethod;

/// Tactical task produced from long-term plan phases.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TacticalTask {
    /// Task identifier.
    pub id: Uuid,
    /// Description of the task.
    pub description: String,
    /// Owner or team responsible.
    pub owner: String,
    /// Estimated effort (hours).
    pub effort_hours: u16,
    /// Phase the task belongs to.
    pub phase_label: String,
    /// Risk multiplier inherited from phase + heuristics.
    pub risk_score: f32,
}

/// Tactical schedule returned by the short-term planner.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TacticalSchedule {
    /// Planning horizon in hours.
    pub horizon_hours: u32,
    /// Generated tasks.
    pub tasks: Vec<TacticalTask>,
    /// Timestamp when schedule was created.
    pub generated_at: DateTime<Utc>,
}

/// Short-term planner generating actionable tasks from strategic plans.
#[derive(Debug, Clone)]
pub struct ShortTermPlanner {
    engine: ScheduleEngine,
    method: TacticalMethod,
}

impl ShortTermPlanner {
    /// Creates a planner with the provided concurrency limit.
    #[must_use]
    pub fn new(max_parallel: usize) -> Self {
        Self {
            engine: ScheduleEngine::new(max_parallel),
            method: TacticalMethod::Kanban,
        }
    }

    /// Selects scheduling method.
    #[must_use]
    pub fn with_method(mut self, method: TacticalMethod) -> Self {
        self.method = method;
        self
    }

    /// Derives a tactical schedule from the selected strategic plan.
    #[must_use]
    pub fn build_schedule(&self, plan: &StrategicPlan) -> TacticalSchedule {
        self.engine.generate(plan, self.method)
    }
}

impl Default for ShortTermPlanner {
    fn default() -> Self {
        Self::new(8)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::long_term::{LongTermPlanner, StrategicObjective};

    #[test]
    fn planner_generates_schedule() {
        let mut long_term = LongTermPlanner::default();
        let plan = long_term
            .generate_portfolio(vec![StrategicObjective::new("stability", 70, 16)], 1)
            .pop()
            .unwrap();
        let short_term = ShortTermPlanner::default().with_method(TacticalMethod::Sprint);
        let schedule = short_term.build_schedule(&plan);
        assert!(!schedule.tasks.is_empty());
    }
}
