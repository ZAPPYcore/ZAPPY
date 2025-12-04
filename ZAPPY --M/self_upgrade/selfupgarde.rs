use anyhow::Result;
use serde_json::json;

use crate::{
    checker::UpgradeChecker,
    helpermethods::UpgradeTelemetry,
    module::{UpgradeAction, UpgradeDirective, UpgradePlan},
    reviewer::UpgradeReviewer,
};

/// Planner orchestrating diagnostics + reviewer to produce upgrade plans.
pub struct UpgradePlanner {
    checker: UpgradeChecker,
    reviewer: UpgradeReviewer,
    telemetry: Option<UpgradeTelemetry>,
}

impl UpgradePlanner {
    /// Creates planner.
    #[must_use]
    pub fn new(
        checker: UpgradeChecker,
        reviewer: UpgradeReviewer,
        telemetry: Option<UpgradeTelemetry>,
    ) -> Self {
        Self {
            checker,
            reviewer,
            telemetry,
        }
    }

    /// Creates plan from directive, returning accepted plan.
    pub fn plan(&self, directive: &UpgradeDirective) -> Result<UpgradePlan> {
        let findings = self.checker.run(directive)?;
        let mut actions = Vec::new();
        for finding in &findings {
            actions.push(UpgradeAction {
                name: format!("mitigate-{}", finding.id),
                metadata: json!({ "severity": finding.severity }),
                estimate_secs: (finding.severity * 300.0) as u64,
            });
        }
        actions.push(UpgradeAction {
            name: "deploy-target-version".into(),
            metadata: json!({ "target": directive.target }),
            estimate_secs: 180,
        });
        let mut plan = UpgradePlan::new(directive.id, actions);
        let max_severity = findings.iter().map(|f| f.severity).fold(0.0, f32::max);
        let accepted = self.reviewer.review(&mut plan, max_severity)?;
        if let Some(tel) = &self.telemetry {
            let _ = tel.log(
                shared_logging::LogLevel::Info,
                "self_upgrade.plan.created",
                json!({
                    "directive": directive.id,
                    "accepted": accepted,
                    "actions": plan.actions.len()
                }),
            );
        }
        Ok(plan)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::helpermethods::UpgradeTelemetryBuilder;
    use shared_event_bus::MemoryEventBus;
    use std::sync::Arc;
    use tempfile::tempdir;

    #[test]
    fn planner_builds_plan() {
        let directive = UpgradeDirective::new("test", "v2", 80);
        let telemetry = UpgradeTelemetryBuilder::new("planner")
            .log_path(tempdir().unwrap().path().join("planner.log"))
            .event_publisher(Arc::new(MemoryEventBus::new(4)))
            .build()
            .ok();
        let planner = UpgradePlanner::new(
            UpgradeChecker::new(telemetry.clone()),
            UpgradeReviewer::new(telemetry.clone()),
            telemetry,
        );
        let plan = planner.plan(&directive).unwrap();
        assert!(!plan.actions.is_empty());
    }
}
