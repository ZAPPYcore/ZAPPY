use anyhow::Result;
use serde_json::json;

use crate::{
    long_term::{
        PlanArchive, PlanScore, PlanScoringEngine, StrategicObjective, StrategicPlan,
        StrategicPlanReviewer,
    },
    telemetry::PlanningTelemetry,
};

use super::LongTermPlanner;

/// Portfolio planner that enriches base plans with scoring/review/telemetry.
pub struct AdvancedPortfolioPlanner {
    planner: LongTermPlanner,
    scoring: PlanScoringEngine,
    reviewer: StrategicPlanReviewer,
    archive: PlanArchive,
    telemetry: Option<PlanningTelemetry>,
}

impl AdvancedPortfolioPlanner {
    /// Creates planner with dependencies.
    #[must_use]
    pub fn new(
        planner: LongTermPlanner,
        scoring: PlanScoringEngine,
        reviewer: StrategicPlanReviewer,
        telemetry: Option<PlanningTelemetry>,
    ) -> Self {
        Self {
            planner,
            scoring,
            reviewer,
            archive: PlanArchive::new(32),
            telemetry,
        }
    }

    /// Generates a reviewed portfolio.
    pub fn build_portfolio(
        &mut self,
        objectives: Vec<StrategicObjective>,
        capacity: usize,
    ) -> Result<Vec<StrategicPlan>> {
        let mut accepted = Vec::new();
        for plan in self.planner.generate_portfolio(objectives, capacity) {
            let score = self.scoring.score(&plan);
            if self.reviewer.approve(&plan, &score) {
                self.archive.push(&plan, &score);
                self.log_plan(&plan, &score, true);
                accepted.push(plan);
            } else {
                self.log_plan(&plan, &score, false);
            }
        }
        Ok(accepted)
    }

    fn log_plan(&self, plan: &StrategicPlan, score: &PlanScore, accepted: bool) {
        if let Some(tel) = &self.telemetry {
            let _ = tel.event(
                "planning.long_term.scored",
                json!({
                    "objective": plan.objective.description,
                    "roi": score.roi,
                    "risk": score.risk,
                    "confidence": score.confidence,
                    "accepted": accepted
                }),
            );
        }
    }

    /// Exposes archive for inspection.
    #[must_use]
    pub fn archive(&self) -> &PlanArchive {
        &self.archive
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::long_term::StrategicObjective;

    #[test]
    fn advanced_planner_filters_plans() {
        let mut planner = AdvancedPortfolioPlanner::new(
            LongTermPlanner::default(),
            PlanScoringEngine::new(),
            StrategicPlanReviewer::new(0.9, 0.1),
            None,
        );
        let plans = planner
            .build_portfolio(vec![StrategicObjective::new("scale", 70, 12)], 1)
            .unwrap();
        assert_eq!(plans.len(), 1);
    }
}
