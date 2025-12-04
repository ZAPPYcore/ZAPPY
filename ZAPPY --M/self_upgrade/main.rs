use anyhow::Result;
use serde_json::json;

use crate::{
    checker::UpgradeChecker,
    helpermethods::UpgradeTelemetry,
    module::{UpgradeDirective, UpgradePlan, UpgradeStatus},
    planner::UpgradePlanner,
    reporter::UpgradeReporter,
    reviewer::UpgradeReviewer,
};

/// Runtime orchestrating self-upgrade pipeline.
pub struct SelfUpgradeRuntime {
    telemetry: Option<UpgradeTelemetry>,
    planner: UpgradePlanner,
    reporter: UpgradeReporter,
}

impl SelfUpgradeRuntime {
    /// Returns builder.
    #[must_use]
    pub fn builder() -> SelfUpgradeRuntimeBuilder {
        SelfUpgradeRuntimeBuilder::default()
    }

    /// Processes directive and emits plan/report.
    pub fn execute(&self, directive: UpgradeDirective) -> Result<UpgradePlan> {
        let mut plan = self.planner.plan(&directive)?;
        plan.status = UpgradeStatus::InProgress;
        if let Some(tel) = &self.telemetry {
            let _ = tel.event(
                "self_upgrade.execution.start",
                json!({ "directive": directive.id }),
            );
        }
        // Simulated execution delay.
        std::thread::sleep(std::time::Duration::from_millis(50));
        plan.status = UpgradeStatus::Completed;
        self.reporter
            .write(&directive, &plan, "upgrade completed")?;
        if let Some(tel) = &self.telemetry {
            let _ = tel.event(
                "self_upgrade.execution.completed",
                json!({ "directive": directive.id }),
            );
        }
        Ok(plan)
    }

    /// Returns telemetry handle.
    #[must_use]
    pub fn telemetry(&self) -> Option<&UpgradeTelemetry> {
        self.telemetry.as_ref()
    }
}

/// Builder for `SelfUpgradeRuntime`.
pub struct SelfUpgradeRuntimeBuilder {
    telemetry: Option<UpgradeTelemetry>,
    report_dir: std::path::PathBuf,
}

impl SelfUpgradeRuntimeBuilder {
    /// Sets telemetry.
    #[must_use]
    pub fn telemetry(mut self, telemetry: UpgradeTelemetry) -> Self {
        self.telemetry = Some(telemetry);
        self
    }

    /// Sets report directory.
    #[must_use]
    pub fn report_dir(mut self, dir: impl Into<std::path::PathBuf>) -> Self {
        self.report_dir = dir.into();
        self
    }

    /// Builds runtime.
    pub fn build(self) -> Result<SelfUpgradeRuntime> {
        let telemetry = self.telemetry;
        let checker = UpgradeChecker::new(telemetry.clone());
        let reviewer = UpgradeReviewer::new(telemetry.clone());
        let planner = UpgradePlanner::new(checker, reviewer, telemetry.clone());
        let reporter = UpgradeReporter::new(self.report_dir, telemetry.clone());
        Ok(SelfUpgradeRuntime {
            telemetry,
            planner,
            reporter,
        })
    }
}

impl Default for SelfUpgradeRuntimeBuilder {
    fn default() -> Self {
        Self {
            telemetry: None,
            report_dir: std::path::PathBuf::from("logs/self_upgrade"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::helpermethods::UpgradeTelemetryBuilder;
    use shared_event_bus::MemoryEventBus;
    use std::sync::Arc;
    use tempfile::tempdir;

    fn telemetry() -> UpgradeTelemetry {
        UpgradeTelemetryBuilder::new("self-upgrade")
            .log_path(tempdir().unwrap().path().join("upgrade.log"))
            .event_publisher(Arc::new(MemoryEventBus::new(8)))
            .build()
            .unwrap()
    }

    #[test]
    fn runtime_executes_directive() {
        let runtime = SelfUpgradeRuntime::builder()
            .telemetry(telemetry())
            .report_dir(tempdir().unwrap().path())
            .build()
            .unwrap();
        let plan = runtime
            .execute(UpgradeDirective::new("upgrade", "v3", 90))
            .unwrap();
        assert_eq!(plan.status, UpgradeStatus::Completed);
    }
}
