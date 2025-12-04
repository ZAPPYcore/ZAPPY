use anyhow::Result;
use serde_json::json;

use crate::{
    helpermethods::UpgradeTelemetry,
    module::{UpgradePlan, UpgradeStatus},
};

/// Reviews upgrade plans for risk.
pub struct UpgradeReviewer {
    telemetry: Option<UpgradeTelemetry>,
    threshold: f32,
}

impl UpgradeReviewer {
    /// Creates reviewer.
    #[must_use]
    pub fn new(telemetry: Option<UpgradeTelemetry>) -> Self {
        Self {
            telemetry,
            threshold: 0.7,
        }
    }

    /// Accepts or blocks a plan based on heuristics.
    pub fn review(&self, plan: &mut UpgradePlan, max_severity: f32) -> Result<bool> {
        let accepted = max_severity <= self.threshold;
        plan.status = if accepted {
            UpgradeStatus::Pending
        } else {
            UpgradeStatus::Blocked
        };
        if let Some(tel) = &self.telemetry {
            let _ = tel.event(
                "self_upgrade.review.completed",
                json!({ "plan": plan.directive_id, "accepted": accepted }),
            );
        }
        Ok(accepted)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::module::{UpgradeAction, UpgradePlan};
    use serde_json::json;
    use uuid::Uuid;

    #[test]
    fn reviewer_blocks_high_severity() {
        let mut plan = UpgradePlan::new(
            Uuid::new_v4(),
            vec![UpgradeAction {
                name: "restart".into(),
                metadata: json!({}),
                estimate_secs: 5,
            }],
        );
        let reviewer = UpgradeReviewer::new(None);
        let accepted = reviewer.review(&mut plan, 0.9).unwrap();
        assert!(!accepted);
        assert_eq!(plan.status, UpgradeStatus::Blocked);
    }
}
