use chrono::{DateTime, Utc};
use indexmap::IndexMap;
use rand::{rngs::SmallRng, Rng, SeedableRng};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Strategic objective provided by upstream systems (autonomy, operators).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategicObjective {
    /// Unique identifier.
    pub id: Uuid,
    /// Narrative summary.
    pub description: String,
    /// Priority (0-100).
    pub priority: u8,
    /// Target horizon in weeks.
    pub horizon_weeks: u16,
    /// Key metrics to improve (name -> target delta).
    pub metrics: IndexMap<String, f32>,
}

impl StrategicObjective {
    /// Convenience constructor.
    #[must_use]
    pub fn new(description: impl Into<String>, priority: u8, horizon_weeks: u16) -> Self {
        Self {
            id: Uuid::new_v4(),
            description: description.into(),
            priority,
            horizon_weeks,
            metrics: IndexMap::new(),
        }
    }
}

/// Detailed phase inside a strategic plan.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanPhase {
    /// Phase name.
    pub label: String,
    /// Week offset where the phase starts.
    pub start_week: u16,
    /// Week offset where the phase completes.
    pub end_week: u16,
    /// Required resources (team -> commitment percentage).
    pub resources: IndexMap<String, f32>,
    /// Risk multiplier for this phase.
    pub risk_multiplier: f32,
}

/// Multi-phase plan built by the long-term planner.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategicPlan {
    /// Objective that triggered the plan.
    pub objective: StrategicObjective,
    /// Generated phases.
    pub phases: Vec<PlanPhase>,
    /// Aggregate risk score (0-1).
    pub risk_score: f32,
    /// Expected blended ROI between 0-1.
    pub expected_roi: f32,
    /// Timestamp when the plan was produced.
    pub generated_at: DateTime<Utc>,
}

impl StrategicPlan {
    /// Returns total duration (weeks).
    #[must_use]
    pub fn total_duration(&self) -> u16 {
        self.phases
            .iter()
            .map(|phase| phase.end_week)
            .max()
            .unwrap_or(0)
    }
}

/// Heuristic configuration for the long-term planner.
#[derive(Debug, Clone)]
pub struct PlanningHeuristics {
    /// Maximum number of phases per plan.
    pub max_phases: usize,
    /// Risk penalty per additional phase.
    pub per_phase_risk: f32,
    /// Baseline ROI multiplier for high priority objectives.
    pub high_priority_roi_boost: f32,
}

impl Default for PlanningHeuristics {
    fn default() -> Self {
        Self {
            max_phases: 4,
            per_phase_risk: 0.04,
            high_priority_roi_boost: 0.15,
        }
    }
}

/// Long-term planner that transforms objectives into phased plans.
#[derive(Debug, Clone)]
pub struct LongTermPlanner {
    heuristics: PlanningHeuristics,
    rng: SmallRng,
}

impl LongTermPlanner {
    /// Creates a new planner.
    #[must_use]
    pub fn new(heuristics: PlanningHeuristics) -> Self {
        Self {
            heuristics,
            rng: SmallRng::from_entropy(),
        }
    }

    /// Generates plans for the provided objectives (sorted by priority).
    pub fn generate_portfolio(
        &mut self,
        mut objectives: Vec<StrategicObjective>,
        capacity: usize,
    ) -> Vec<StrategicPlan> {
        objectives.sort_by(|a, b| b.priority.cmp(&a.priority));
        objectives
            .into_iter()
            .take(capacity)
            .map(|objective| self.generate_plan(objective))
            .collect()
    }

    fn generate_plan(&mut self, objective: StrategicObjective) -> StrategicPlan {
        let phase_count = self
            .rng
            .gen_range(2..=self.heuristics.max_phases.max(2))
            .min(objective.horizon_weeks.max(1) as usize);
        let mut phases = Vec::with_capacity(phase_count);
        let mut cursor = 0u16;
        for idx in 0..phase_count {
            let span = (objective.horizon_weeks / phase_count as u16).max(1);
            let label = format!("Phase {}", idx + 1);
            let mut resources = IndexMap::new();
            resources.insert("engineering".into(), self.rng.gen_range(0.3..0.7));
            resources.insert("ops".into(), self.rng.gen_range(0.1..0.3));
            if idx % 2 == 0 {
                resources.insert("research".into(), self.rng.gen_range(0.05..0.2));
            }
            let risk_multiplier = 1.0 + (idx as f32 * 0.05);
            phases.push(PlanPhase {
                label,
                start_week: cursor,
                end_week: cursor + span,
                resources,
                risk_multiplier,
            });
            cursor += span;
        }

        let risk_score = phases.len() as f32 * self.heuristics.per_phase_risk
            + (objective.priority as f32 / 300.0);
        let mut expected_roi = 0.35 + (objective.priority as f32 / 200.0);
        if objective.priority >= 80 {
            expected_roi += self.heuristics.high_priority_roi_boost;
        }
        expected_roi = expected_roi.clamp(0.0, 1.0);

        StrategicPlan {
            objective,
            phases,
            risk_score: risk_score.clamp(0.0, 1.0),
            expected_roi,
            generated_at: Utc::now(),
        }
    }
}

impl Default for LongTermPlanner {
    fn default() -> Self {
        Self::new(PlanningHeuristics::default())
    }
}

/// Advanced planning utilities.
pub mod advanced;
/// Plan scoring engine helpers.
pub mod engine;
/// Mathematical helper functions.
pub mod func;
/// Resource helper functions.
pub mod helper;
/// Plan archival utilities.
pub mod plans;
/// Reviewer for strategic plans.
pub mod reviewer;
/// Objective sources.
pub mod sources;

pub use advanced::AdvancedPortfolioPlanner;
pub use engine::{PlanScore, PlanScoringEngine};
pub use plans::PlanArchive;
pub use reviewer::StrategicPlanReviewer;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn planner_generates_phased_plan() {
        let mut planner = LongTermPlanner::default();
        let objective = StrategicObjective::new("Scale infra", 85, 24);
        let plan = planner
            .generate_portfolio(vec![objective], 1)
            .pop()
            .unwrap();
        assert!(!plan.phases.is_empty());
        assert!(plan.risk_score >= 0.0);
    }
}
