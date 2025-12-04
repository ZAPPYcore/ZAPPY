use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

/// Directive describing a requested self-upgrade.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpgradeDirective {
    /// Directive id.
    pub id: Uuid,
    /// Description.
    pub description: String,
    /// Target version or feature.
    pub target: String,
    /// Priority 0-100.
    pub priority: u8,
}

impl UpgradeDirective {
    /// Creates a directive.
    #[must_use]
    pub fn new(description: impl Into<String>, target: impl Into<String>, priority: u8) -> Self {
        Self {
            id: Uuid::new_v4(),
            description: description.into(),
            target: target.into(),
            priority,
        }
    }
}

/// Diagnostic finding produced by the checker.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpgradeFinding {
    /// Finding id.
    pub id: Uuid,
    /// Severity 0-1.
    pub severity: f32,
    /// Description.
    pub message: String,
    /// Suggested remediation.
    pub remediation: String,
}

/// Status of an upgrade plan.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum UpgradeStatus {
    /// Pending execution.
    Pending,
    /// Executing tasks.
    InProgress,
    /// Completed successfully.
    Completed,
    /// Blocked by reviewer.
    Blocked,
}

/// Action executed as part of an upgrade plan.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpgradeAction {
    /// Action name.
    pub name: String,
    /// Arbitrary metadata.
    pub metadata: Value,
    /// Estimated duration seconds.
    pub estimate_secs: u64,
}

/// Plan describing steps to self-upgrade.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpgradePlan {
    /// Directive id.
    pub directive_id: Uuid,
    /// Ordered actions.
    pub actions: Vec<UpgradeAction>,
    /// Current status.
    pub status: UpgradeStatus,
    /// Generated timestamp.
    pub generated_at: DateTime<Utc>,
}

impl UpgradePlan {
    /// Creates plan from actions.
    #[must_use]
    pub fn new(directive_id: Uuid, actions: Vec<UpgradeAction>) -> Self {
        Self {
            directive_id,
            actions,
            status: UpgradeStatus::Pending,
            generated_at: Utc::now(),
        }
    }
}
