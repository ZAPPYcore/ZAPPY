//! Master control loop coordinating directives.

/// Builder utilities for the master controller.
pub mod maker;
/// Reliability calculations for control loops.
pub mod masterfunc;

use std::sync::Arc;

use chrono::{DateTime, Utc};
use maker::MasterControllerBuilder;
use masterfunc::ReliabilityCalculator;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use serde_json::json;
use shared_logging::LogLevel;

use crate::{
    decision::DecisionVerdict,
    module::{AutonomyError, ModuleBroker},
    telemetry::AutonomyTelemetry,
};

/// Observability metrics for the master loop.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MasterMetrics {
    /// Description of the last cycle.
    pub last_cycle: Option<String>,
    /// Total directives issued in the current epoch.
    pub directives_issued: usize,
    /// Average reviewer confidence over time.
    pub avg_confidence: f32,
    /// Number of active modules in the registry.
    pub modules_active: usize,
    /// Last update timestamp.
    pub updated_at: DateTime<Utc>,
}

impl Default for MasterMetrics {
    fn default() -> Self {
        Self {
            last_cycle: None,
            directives_issued: 0,
            avg_confidence: 0.0,
            modules_active: 0,
            updated_at: Utc::now(),
        }
    }
}

/// Applies decisions to modules and tracks reliability.
#[derive(Debug, Clone)]
pub struct MasterController {
    broker: ModuleBroker,
    max_inflight: usize,
    metrics: Arc<RwLock<MasterMetrics>>,
    reliability: Arc<RwLock<ReliabilityCalculator>>,
    telemetry: Option<AutonomyTelemetry>,
}

impl MasterController {
    /// Creates a new controller.
    #[must_use]
    pub fn new(broker: ModuleBroker, max_inflight: usize) -> Self {
        Self {
            broker,
            max_inflight,
            metrics: Arc::new(RwLock::new(MasterMetrics::default())),
            reliability: Arc::new(RwLock::new(ReliabilityCalculator::default())),
            telemetry: None,
        }
    }

    /// Returns a builder for the controller.
    #[must_use]
    pub fn builder(broker: ModuleBroker) -> MasterControllerBuilder {
        MasterControllerBuilder::new(broker)
    }

    /// Attaches telemetry.
    pub fn with_telemetry(mut self, telemetry: AutonomyTelemetry) -> Self {
        self.telemetry = Some(telemetry);
        self
    }

    /// Applies an approved verdict to the module fabric.
    pub async fn apply_verdict(
        &self,
        verdict: &DecisionVerdict,
    ) -> Result<MasterMetrics, AutonomyError> {
        if verdict.directives.len() > self.max_inflight {
            return Err(AutonomyError::Internal(format!(
                "too many directives: {} > {}",
                verdict.directives.len(),
                self.max_inflight
            )));
        }

        if let Some(tel) = &self.telemetry {
            let _ = tel.log(
                LogLevel::Info,
                "autonomy.master.apply_start",
                json!({
                    "directives": verdict.directives.len(),
                    "confidence": verdict.confidence
                }),
            );
        }

        {
            let mut reliability = self.reliability.write();
            reliability.record(verdict.confidence);
        }

        {
            let mut metrics = self.metrics.write();
            metrics.directives_issued += verdict.directives.len();
            metrics.avg_confidence = self.reliability.read().score();
            metrics.last_cycle = Some(verdict.hypothesis.summary.clone());
            metrics.modules_active = self.broker.registry().len();
            metrics.updated_at = Utc::now();
        }

        let snapshot = self.metrics.read().clone();
        if let Some(tel) = &self.telemetry {
            let _ = tel.log(
                LogLevel::Info,
                "autonomy.master.apply_complete",
                json!({
                    "directives": verdict.directives.len(),
                    "avg_confidence": snapshot.avg_confidence,
                    "modules_active": snapshot.modules_active
                }),
            );
            let _ = tel.event(
                "autonomy.master.metrics",
                json!({
                    "directives": snapshot.directives_issued,
                    "avg_confidence": snapshot.avg_confidence
                }),
            );
        }

        Ok(snapshot)
    }

    /// Returns the latest metrics snapshot.
    #[must_use]
    pub fn metrics(&self) -> MasterMetrics {
        self.metrics.read().clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        decision::decisionmaking::DecisionHypothesis,
        module::{
            ControlDirective, DirectivePriority, ModuleKind, ModuleRegistry, ModuleSpec,
            ModuleTarget,
        },
    };

    fn sample_verdict() -> DecisionVerdict {
        DecisionVerdict {
            hypothesis: DecisionHypothesis {
                summary: "scale".into(),
                rationale: "test".into(),
                risk: 0.2,
            },
            directives: vec![ControlDirective::new(
                ModuleTarget::Kind(ModuleKind::Planner),
                "noop",
            )
            .with_priority(DirectivePriority::Routine)],
            findings: Vec::new(),
            confidence: 0.8,
        }
    }

    #[tokio::test]
    async fn controller_updates_metrics() {
        let registry = ModuleRegistry::default();
        registry.upsert(ModuleSpec::new("planner", ModuleKind::Planner));
        let broker = ModuleBroker::new(registry);
        let controller = MasterController::builder(broker).max_inflight(4).build();
        let verdict = sample_verdict();
        let metrics = controller.apply_verdict(&verdict).await.unwrap();
        assert_eq!(metrics.directives_issued, 1);
        assert_eq!(metrics.modules_active, 1);
    }
}
