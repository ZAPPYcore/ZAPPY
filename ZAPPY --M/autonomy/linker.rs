//! Autonomy linker connecting decision and master loops.

use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::json;
use shared_logging::LogLevel;
use tracing::instrument;
use uuid::Uuid;

use crate::{
    decision::{DecisionDirector, DecisionVerdict},
    master::{MasterController, MasterMetrics},
    module::{AutonomyError, AutonomySignal, ModuleBroker},
    telemetry::AutonomyTelemetry,
};

/// Report returned after running a full autonomy cycle.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CycleReport {
    /// Cycle identifier.
    pub cycle_id: Uuid,
    /// Decision output.
    pub verdict: DecisionVerdict,
    /// Master metrics after applying the verdict.
    pub master_metrics: MasterMetrics,
    /// Timestamp of completion.
    pub completed_at: chrono::DateTime<Utc>,
}

/// Connects the decision layer with the master/controller layer.
#[derive(Debug, Clone)]
pub struct AutonomyLinker {
    director: DecisionDirector,
    master: MasterController,
    broker: ModuleBroker,
    telemetry: Option<AutonomyTelemetry>,
}

impl AutonomyLinker {
    /// Creates a new linker from its components.
    #[must_use]
    pub fn new(director: DecisionDirector, master: MasterController, broker: ModuleBroker) -> Self {
        Self {
            director,
            master,
            broker,
            telemetry: None,
        }
    }

    /// Attaches telemetry.
    #[must_use]
    pub fn with_telemetry(mut self, telemetry: AutonomyTelemetry) -> Self {
        self.telemetry = Some(telemetry);
        self
    }

    /// Returns the latest master metrics snapshot.
    #[must_use]
    pub fn metrics(&self) -> MasterMetrics {
        self.master.metrics()
    }

    /// Executes a full cycle, returning a comprehensive report.
    #[instrument(skip(self))]
    pub async fn execute_cycle(
        &self,
        signal: AutonomySignal,
    ) -> Result<CycleReport, AutonomyError> {
        // Evaluate modules for additional context.
        let _pulse = self.broker.evaluate_signal(&signal)?;
        if let Some(tel) = &self.telemetry {
            let narrative = signal.narrative.clone();
            let tags = signal.tags.clone();
            let _ = tel.log(
                LogLevel::Info,
                "autonomy.cycle.signal_evaluated",
                json!({ "narrative": narrative, "tags": tags }),
            );
        }
        let verdict = self.director.decide_signal(signal).await?;
        let metrics = self.master.apply_verdict(&verdict).await?;
        if let Some(tel) = &self.telemetry {
            let _ = tel.log(
                LogLevel::Info,
                "autonomy.cycle.completed",
                json!({ "directives": verdict.directives.len(), "confidence": verdict.confidence }),
            );
            let _ = tel.event(
                "autonomy.cycle.completed",
                json!({ "directives": verdict.directives.len(), "confidence": verdict.confidence }),
            );
        }

        Ok(CycleReport {
            cycle_id: Uuid::new_v4(),
            verdict,
            master_metrics: metrics,
            completed_at: Utc::now(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::module::{ModuleBroker, ModuleKind, ModuleRegistry, ModuleSpec, SignalScope};

    #[tokio::test]
    async fn linker_runs_cycle() {
        let registry = ModuleRegistry::default();
        registry.upsert(ModuleSpec::new("planner", ModuleKind::Planner));
        let broker = ModuleBroker::new(registry.clone());
        let director = crate::decision::build_director(&broker);
        let master = MasterController::builder(broker.clone()).build();
        let linker = AutonomyLinker::new(director, master, broker.clone());
        let signal = AutonomySignal::new(SignalScope::Global, "cycle").with_metric("load", 0.3);
        let report = linker.execute_cycle(signal).await.unwrap();
        assert_eq!(report.master_metrics.directives_issued, 1);
    }
}
