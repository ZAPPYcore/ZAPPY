use serde::{Deserialize, Serialize};

use crate::receiver::KnowledgeArtifact;

use super::{helper::ContentInspector, methods::RiskComputation};

/// Security policy thresholds.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityPolicy {
    /// Maximum acceptable risk score.
    pub max_risk: f32,
    /// Whether to reject artifacts missing source metadata.
    pub require_source: bool,
}

impl Default for SecurityPolicy {
    fn default() -> Self {
        Self {
            max_risk: 0.6,
            require_source: true,
        }
    }
}

/// Enforces policy against incoming artifacts.
#[derive(Debug, Clone)]
pub struct KnowledgeGuard {
    policy: SecurityPolicy,
    inspector: ContentInspector,
    risk: RiskComputation,
}

impl KnowledgeGuard {
    /// Creates a new guard instance.
    #[must_use]
    pub fn new(policy: SecurityPolicy) -> Self {
        Self {
            policy,
            inspector: ContentInspector::default(),
            risk: RiskComputation::default(),
        }
    }

    /// Enforces the policy on the artifact.
    pub fn enforce(&self, artifact: &KnowledgeArtifact) -> Result<(), String> {
        if self.policy.require_source && artifact.source.trim().is_empty() {
            return Err("missing source".into());
        }

        let findings = self.inspector.inspect(&artifact.content);
        let profile = self.risk.profile(&findings);
        if profile.score > self.policy.max_risk {
            return Err(format!(
                "risk {:.2} exceeds threshold: {:?}",
                profile.score, profile.labels
            ));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn guard_rejects_high_risk() {
        let guard = KnowledgeGuard::new(SecurityPolicy::default());
        let artifact = KnowledgeArtifact::new("src", "title", "This contains top secret info.");
        assert!(guard.enforce(&artifact).is_err());
    }
}
