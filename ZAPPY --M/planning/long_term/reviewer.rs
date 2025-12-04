use serde::{Deserialize, Serialize};

use crate::long_term::StrategicPlan;

use super::engine::PlanScore;

/// Reviewer ensures plan risk stays within guardrails.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategicPlanReviewer {
    /// Maximum allowable risk.
    pub max_risk: f32,
    /// Minimum confidence required.
    pub min_confidence: f32,
}

impl StrategicPlanReviewer {
    /// Creates reviewer.
    #[must_use]
    pub fn new(max_risk: f32, min_confidence: f32) -> Self {
        Self {
            max_risk,
            min_confidence,
        }
    }

    /// Determines if plan should be approved.
    #[must_use]
    pub fn approve(&self, _plan: &StrategicPlan, score: &PlanScore) -> bool {
        score.risk <= self.max_risk && score.confidence >= self.min_confidence
    }
}

impl Default for StrategicPlanReviewer {
    fn default() -> Self {
        Self::new(0.65, 0.45)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::long_term::{StrategicObjective, StrategicPlan};
    use chrono::Utc;

    #[test]
    fn reviewer_blocks_high_risk() {
        let reviewer = StrategicPlanReviewer::new(0.2, 0.2);
        let plan = StrategicPlan {
            objective: StrategicObjective::new("test", 60, 10),
            phases: vec![],
            risk_score: 0.5,
            expected_roi: 0.6,
            generated_at: Utc::now(),
        };
        let score = PlanScore {
            roi: 0.7,
            risk: 0.4,
            confidence: 0.6,
        };
        assert!(!reviewer.approve(&plan, &score));
    }
}
