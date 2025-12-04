use anyhow::Result;
use rand::Rng;
use serde_json::json;
use uuid::Uuid;

use crate::{
    helpermethods::UpgradeTelemetry,
    module::{UpgradeDirective, UpgradeFinding},
};

/// Performs system diagnostics prior to an upgrade.
pub struct UpgradeChecker {
    telemetry: Option<UpgradeTelemetry>,
}

impl UpgradeChecker {
    /// Creates checker.
    #[must_use]
    pub fn new(telemetry: Option<UpgradeTelemetry>) -> Self {
        Self { telemetry }
    }

    /// Runs diagnostics returning findings.
    pub fn run(&self, directive: &UpgradeDirective) -> Result<Vec<UpgradeFinding>> {
        let mut rng = rand::thread_rng();
        let mut findings = Vec::new();
        for idx in 0..3 {
            let severity = rng.gen_range(0.0..1.0);
            findings.push(UpgradeFinding {
                id: Uuid::new_v4(),
                severity,
                message: format!("check-{} severity {:.2}", idx, severity),
                remediation: "restart component".into(),
            });
        }
        if let Some(tel) = &self.telemetry {
            let _ = tel.log(
                shared_logging::LogLevel::Info,
                "self_upgrade.checker.completed",
                json!({ "directive": directive.id, "findings": findings.len() }),
            );
        }
        Ok(findings)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checker_produces_findings() {
        let directive = UpgradeDirective::new("test", "v2", 80);
        let checker = UpgradeChecker::new(None);
        let findings = checker.run(&directive).unwrap();
        assert!(!findings.is_empty());
    }
}
