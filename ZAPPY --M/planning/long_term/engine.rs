use serde::{Deserialize, Serialize};

use crate::long_term::{PlanPhase, StrategicPlan};

use super::func::{confidence_score, projected_roi, risk_from_complexity};
use super::helper::normalize_resources;

/// Score assigned to a strategic plan.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanScore {
    /// ROI estimate.
    pub roi: f32,
    /// Risk estimate.
    pub risk: f32,
    /// Overall confidence.
    pub confidence: f32,
}

/// Engine that evaluates plans and phases.
#[derive(Debug, Clone)]
pub struct PlanScoringEngine;

impl PlanScoringEngine {
    /// Creates a new engine.
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// Scores a plan based on ROI/risk heuristics.
    #[must_use]
    pub fn score(&self, plan: &StrategicPlan) -> PlanScore {
        let roi = projected_roi(
            plan.objective.priority,
            &plan.objective.metrics,
            plan.total_duration(),
        );
        let risk = self.phase_risk(plan);
        let confidence = confidence_score(roi, risk);
        PlanScore {
            roi,
            risk,
            confidence,
        }
    }

    fn phase_risk(&self, plan: &StrategicPlan) -> f32 {
        let resources = plan
            .phases
            .iter()
            .map(|phase| total_resources(&phase))
            .collect::<Vec<_>>();
        if resources.is_empty() {
            return 0.0;
        }
        let avg = resources.iter().sum::<f32>() / resources.len() as f32;
        (risk_from_complexity(plan.phases.len(), plan.objective.priority) + avg * 0.2)
            .clamp(0.0, 1.0)
    }
}

fn total_resources(phase: &PlanPhase) -> f32 {
    let mut resources = phase.resources.clone();
    normalize_resources(&mut resources);
    resources.values().copied().sum()
}

impl Default for PlanScoringEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::long_term::{PlanPhase, StrategicObjective};
    use indexmap::indexmap;

    #[test]
    fn engine_scores_plan() {
        let mut objective = StrategicObjective::new("scale", 80, 20);
        objective.metrics = indexmap! { "growth".into() => 0.7 };
        let plan = StrategicPlan {
            objective,
            phases: vec![PlanPhase {
                label: "phase".into(),
                start_week: 0,
                end_week: 10,
                resources: indexmap! { "eng".into() => 0.8 },
                risk_multiplier: 1.1,
            }],
            risk_score: 0.3,
            expected_roi: 0.5,
            generated_at: chrono::Utc::now(),
        };
        let score = PlanScoringEngine::new().score(&plan);
        assert!(score.confidence > 0.0);
    }
}
