//! Runtime entrypoints and sample execution helpers.

use std::time::Duration;

use anyhow::Result;
use tokio::time::sleep;

use crate::{
    decision::build_director,
    linker::{AutonomyLinker, CycleReport},
    master::{MasterController, MasterMetrics},
    module::{
        AutonomySignal, ControlDirective, DirectivePriority, ModuleBroker, ModuleKind,
        ModuleRegistry, ModuleSpec, SignalScope,
    },
    telemetry::AutonomyTelemetryBuilder,
};

/// Fully wired autonomy runtime ready to execute decision cycles.
#[derive(Debug, Clone)]
pub struct AutonomyRuntime {
    linker: AutonomyLinker,
    broker: ModuleBroker,
}

impl AutonomyRuntime {
    /// Bootstraps the runtime with default modules.
    #[must_use]
    pub fn bootstrap() -> Self {
        let registry = ModuleRegistry::default();
        registry.upsert(ModuleSpec::new("global-planner", ModuleKind::Planner));
        registry.upsert(ModuleSpec::new("infra-executor", ModuleKind::Executor));
        registry.upsert(ModuleSpec::new("sensor-array", ModuleKind::Sensor));

        let broker = ModuleBroker::new(registry.clone());
        let telemetry = AutonomyTelemetryBuilder::new("autonomy.runtime")
            .log_path("logs/autonomy/runtime.log.jsonl")
            .build()
            .ok();

        let mut director = build_director(&broker);
        if let Some(tel) = telemetry.clone() {
            director = director.with_telemetry(tel.clone());
        }
        let mut master = MasterController::builder(broker.clone())
            .max_inflight(6)
            .build();
        if let Some(tel) = telemetry.clone() {
            master = master.with_telemetry(tel.clone());
        }
        let mut linker = AutonomyLinker::new(director, master, broker.clone());
        if let Some(tel) = telemetry {
            linker = linker.with_telemetry(tel);
        }

        Self { linker, broker }
    }

    /// Runs a single sample cycle returning its report.
    pub async fn run_sample_cycle(&self) -> Result<CycleReport> {
        let signal = AutonomySignal::new(SignalScope::Global, "sample")
            .with_metric("load", 0.52)
            .with_tag("mode", "demo");
        self.run_cycle(signal).await
    }

    /// Executes a cycle with the provided signal.
    pub async fn run_cycle(&self, signal: AutonomySignal) -> Result<CycleReport> {
        let report = self.linker.execute_cycle(signal).await?;
        Ok(report)
    }

    /// Returns the most recent master metrics.
    #[must_use]
    pub fn metrics(&self) -> MasterMetrics {
        self.linker.metrics()
    }

    /// Issues an emergent directive directly via the broker.
    #[must_use]
    pub fn emergency_directive(&self, description: &str) -> ControlDirective {
        let directive = self.issue_directive(
            ModuleKind::SelfHealing,
            DirectivePriority::Critical,
            description,
        );
        tracing::warn!("Emergency directive issued: {}", directive.instructions);
        directive
    }

    /// Issues a directive for the specified module kind.
    #[must_use]
    pub fn issue_directive(
        &self,
        kind: ModuleKind,
        priority: DirectivePriority,
        description: impl Into<String>,
    ) -> ControlDirective {
        self.broker.issue_directive(kind, priority, description)
    }
}

/// Runs a demonstration loop with delays, intended for integration tests.
pub async fn demo_run(iterations: usize) -> Result<Vec<CycleReport>> {
    let runtime = AutonomyRuntime::bootstrap();
    let mut reports = Vec::new();
    for _ in 0..iterations {
        reports.push(runtime.run_sample_cycle().await?);
        sleep(Duration::from_millis(10)).await;
    }
    Ok(reports)
}
