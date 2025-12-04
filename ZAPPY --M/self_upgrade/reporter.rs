use std::{fs, path::PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::{
    helpermethods::UpgradeTelemetry,
    module::{UpgradeDirective, UpgradePlan},
};

/// Report summarizing upgrade execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpgradeReport {
    /// Directive id.
    pub directive_id: uuid::Uuid,
    /// Status.
    pub status: String,
    /// Notes.
    pub notes: String,
}

/// Emits upgrade reports.
pub struct UpgradeReporter {
    telemetry: Option<UpgradeTelemetry>,
    output_dir: PathBuf,
}

impl UpgradeReporter {
    /// Creates reporter writing to `output_dir`.
    #[must_use]
    pub fn new(output_dir: PathBuf, telemetry: Option<UpgradeTelemetry>) -> Self {
        Self {
            telemetry,
            output_dir,
        }
    }

    /// Writes report to disk.
    pub fn write(
        &self,
        directive: &UpgradeDirective,
        plan: &UpgradePlan,
        notes: &str,
    ) -> Result<PathBuf> {
        fs::create_dir_all(&self.output_dir)?;
        let report = UpgradeReport {
            directive_id: directive.id,
            status: format!("{:?}", plan.status),
            notes: notes.into(),
        };
        let path = self
            .output_dir
            .join(format!("upgrade-{}.json", directive.id));
        fs::write(&path, serde_json::to_vec_pretty(&report)?)
            .with_context(|| format!("writing {:?}", path))?;
        if let Some(tel) = &self.telemetry {
            let _ = tel.event(
                "self_upgrade.report.generated",
                json!({ "path": path, "status": report.status }),
            );
        }
        Ok(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::module::{UpgradeAction, UpgradePlan};
    use tempfile::tempdir;

    #[test]
    fn reporter_writes_file() {
        let temp = tempdir().unwrap();
        let reporter = UpgradeReporter::new(temp.path().into(), None);
        let directive = UpgradeDirective::new("test", "v2", 50);
        let plan = UpgradePlan::new(
            directive.id,
            vec![UpgradeAction {
                name: "step".into(),
                metadata: json!({}),
                estimate_secs: 5,
            }],
        );
        let path = reporter.write(&directive, &plan, "ok").unwrap();
        assert!(path.exists());
    }
}
