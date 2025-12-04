use chrono::Utc;
use uuid::Uuid;

use crate::long_term::StrategicPlan;

use super::{
    helper::{select_owner, task_count},
    methods::TacticalMethod,
    TacticalSchedule, TacticalTask,
};

/// Engine responsible for turning plan phases into executable tasks.
#[derive(Debug, Clone)]
pub struct ScheduleEngine {
    max_parallel: usize,
}

impl ScheduleEngine {
    /// Creates engine.
    #[must_use]
    pub fn new(max_parallel: usize) -> Self {
        Self {
            max_parallel: max_parallel.max(1),
        }
    }

    /// Generates schedule based on method.
    #[must_use]
    pub fn generate(&self, plan: &StrategicPlan, method: TacticalMethod) -> TacticalSchedule {
        let mut tasks = Vec::new();
        for phase in &plan.phases {
            let count = task_count(phase.resources.len(), phase.risk_multiplier);
            for idx in 0..count {
                tasks.push(TacticalTask {
                    id: Uuid::new_v4(),
                    description: format!("{} :: subtask {}", phase.label, idx + 1),
                    owner: select_owner(idx),
                    effort_hours: self.estimate_effort(phase.risk_multiplier, method),
                    phase_label: phase.label.clone(),
                    risk_score: (plan.risk_score + phase.risk_multiplier / 10.0).clamp(0.0, 1.0),
                });
            }
        }
        tasks.truncate(self.max_parallel * 6 * method.cadence_multiplier() as usize);
        TacticalSchedule {
            horizon_hours: plan.total_duration() as u32 * 24 * method.cadence_multiplier(),
            tasks,
            generated_at: Utc::now(),
        }
    }

    fn estimate_effort(&self, risk: f32, method: TacticalMethod) -> u16 {
        let base = match method {
            TacticalMethod::Kanban => 16,
            TacticalMethod::Sprint => 32,
        };
        (base as f32 + risk * 12.0) as u16
    }
}

impl Default for ScheduleEngine {
    fn default() -> Self {
        Self::new(8)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::long_term::{LongTermPlanner, StrategicObjective};

    #[test]
    fn engine_generates_schedule() {
        let mut planner = LongTermPlanner::default();
        let plan = planner
            .generate_portfolio(vec![StrategicObjective::new("grow", 70, 20)], 1)
            .pop()
            .unwrap();
        let engine = ScheduleEngine::default();
        assert!(!engine
            .generate(&plan, TacticalMethod::Kanban)
            .tasks
            .is_empty());
    }
}
