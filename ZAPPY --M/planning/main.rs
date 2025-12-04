use std::sync::Arc;

use anyhow::Result;
use serde_json::json;
use shared_event_bus::MemoryEventBus;
use shared_logging::LogLevel;
use uuid::Uuid;

use crate::{
    long_term::{AdvancedPortfolioPlanner, LongTermPlanner, StrategicObjective, StrategicPlan},
    short_term::{ShortTermPlanner, TacticalSchedule},
    telemetry::PlanningTelemetry,
};

use crate::module::{PlanningDirective, PriorityBand};

/// Composite planning runtime used by the autonomy + metacognition stack.
pub struct PlanningRuntime {
    long_term: LongTermPlanner,
    short_term: ShortTermPlanner,
    telemetry: Option<PlanningTelemetry>,
    advanced: Option<AdvancedPortfolioPlanner>,
}

impl Default for PlanningRuntime {
    fn default() -> Self {
        let bus = Arc::new(MemoryEventBus::new(256));
        let telemetry = PlanningTelemetry::builder("planning-runtime")
            .event_publisher(bus)
            .build()
            .ok();
        Self {
            long_term: LongTermPlanner::default(),
            short_term: ShortTermPlanner::default(),
            telemetry,
            advanced: None,
        }
    }
}

impl PlanningRuntime {
    /// Creates a runtime with custom components.
    #[must_use]
    pub fn new(
        long_term: LongTermPlanner,
        short_term: ShortTermPlanner,
        telemetry: Option<PlanningTelemetry>,
    ) -> Self {
        Self {
            long_term,
            short_term,
            telemetry,
            advanced: None,
        }
    }

    /// Injects telemetry at runtime.
    pub fn with_telemetry(mut self, telemetry: PlanningTelemetry) -> Self {
        self.telemetry = Some(telemetry);
        self
    }

    /// Injects an advanced planner.
    #[must_use]
    pub fn with_advanced(mut self, advanced: AdvancedPortfolioPlanner) -> Self {
        self.advanced = Some(advanced);
        self
    }

    /// Sets advanced planner after construction.
    pub fn set_advanced(&mut self, advanced: AdvancedPortfolioPlanner) {
        self.advanced = Some(advanced);
    }

    /// Produces a strategic plan from incoming directives.
    pub fn propose_strategic_plan(
        &mut self,
        directives: Vec<PlanningDirective>,
    ) -> Result<Option<StrategicPlan>> {
        if directives.is_empty() {
            return Ok(None);
        }
        let objectives = directives
            .into_iter()
            .map(|directive| StrategicObjective {
                id: directive
                    .signal
                    .map(|signal| signal.id)
                    .unwrap_or_else(Uuid::new_v4),
                description: directive.objective,
                priority: directive.priority.as_score(),
                horizon_weeks: match directive.priority {
                    PriorityBand::Low => 8,
                    PriorityBand::Medium => 16,
                    PriorityBand::High => 24,
                },
                metrics: Default::default(),
            })
            .collect::<Vec<_>>();
        self.log(
            LogLevel::Info,
            "planning.long_term.queue",
            json!({ "objectives": objectives.len() }),
        );
        let mut portfolio = if let Some(advanced) = self.advanced.as_mut() {
            advanced.build_portfolio(objectives.clone(), 3)?
        } else {
            self.long_term.generate_portfolio(objectives, 3)
        };
        let plan = portfolio.pop();
        if let Some(plan) = &plan {
            self.log(
                LogLevel::Info,
                "planning.long_term.plan_generated",
                json!({
                    "objective": plan.objective.description,
                    "risk": plan.risk_score,
                    "roi": plan.expected_roi,
                    "duration_weeks": plan.total_duration()
                }),
            );
            self.event(
                "planning.long_term.plan_generated",
                json!({
                    "objective_id": plan.objective.id,
                    "risk": plan.risk_score,
                    "roi": plan.expected_roi
                }),
            );
        }
        Ok(plan)
    }

    /// Converts a strategic plan into a tactical schedule.
    pub fn build_tactical_schedule(&self, plan: &StrategicPlan) -> Result<TacticalSchedule> {
        self.log(
            LogLevel::Info,
            "planning.short_term.begin",
            json!({
                "objective": plan.objective.description,
                "phases": plan.phases.len()
            }),
        );
        let schedule = self.short_term.build_schedule(plan);
        self.log(
            LogLevel::Info,
            "planning.short_term.schedule_ready",
            json!({ "tasks": schedule.tasks.len() }),
        );
        self.event(
            "planning.short_term.schedule_ready",
            json!({ "tasks": schedule.tasks.len(), "horizon_hours": schedule.horizon_hours }),
        );
        Ok(schedule)
    }

    /// Reacts to new signals (re-planning) by evaluating threshold.
    pub fn ingest_signal(&mut self, signal: crate::module::PlanningSignal) -> Result<bool> {
        self.log(
            LogLevel::Debug,
            "planning.signal.received",
            json!({ "impact": signal.impact, "narrative": signal.narrative }),
        );
        let should_replan = signal.impact >= 50;
        if should_replan {
            self.event(
                "planning.signal.replan_triggered",
                json!({ "signal_id": signal.id, "impact": signal.impact }),
            );
        }
        Ok(should_replan)
    }

    fn log(&self, level: LogLevel, message: &str, metadata: serde_json::Value) {
        if let Some(tel) = &self.telemetry {
            let _ = tel.log(level, message, metadata);
        }
    }

    fn event(&self, event_type: &str, payload: serde_json::Value) {
        if let Some(tel) = &self.telemetry {
            let _ = tel.event(event_type, payload);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::telemetry::PlanningTelemetryBuilder;
    use shared_event_bus::MemoryEventBus;
    use std::sync::Arc;
    use tempfile::tempdir;

    #[test]
    fn runtime_generates_plan_and_schedule() {
        let temp = tempdir().unwrap();
        let bus = Arc::new(MemoryEventBus::new(32));
        let telemetry = PlanningTelemetryBuilder::new("planning-tests")
            .event_publisher(bus)
            .log_path(temp.path().join("plan.log"))
            .build()
            .unwrap();
        let mut runtime = PlanningRuntime::new(
            LongTermPlanner::default(),
            ShortTermPlanner::default(),
            None,
        )
        .with_telemetry(telemetry);
        let plan = runtime
            .propose_strategic_plan(vec![PlanningDirective::critical("stabilize infra")])
            .unwrap()
            .unwrap();
        let schedule = runtime.build_tactical_schedule(&plan).unwrap();
        assert!(!schedule.tasks.is_empty());
    }
}
