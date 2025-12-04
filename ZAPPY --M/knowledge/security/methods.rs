use serde::{Deserialize, Serialize};

use super::helper::InspectionFinding;

/// Risk profile computed from inspection findings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskProfile {
    /// Aggregate risk score (0-1).
    pub score: f32,
    /// Labels that triggered the risk.
    pub labels: Vec<String>,
}

/// Computes risk based on heuristics.
#[derive(Debug, Default, Clone)]
pub struct RiskComputation;

impl RiskComputation {
    /// Returns a profile given inspection findings.
    #[must_use]
    pub fn profile(&self, findings: &[InspectionFinding]) -> RiskProfile {
        if findings.is_empty() {
            return RiskProfile {
                score: 0.1,
                labels: Vec::new(),
            };
        }

        let mut score: f32 = 0.0;
        let mut labels = Vec::new();
        for finding in findings {
            score += finding.severity;
            labels.push(finding.label.clone());
        }

        RiskProfile {
            score: (score / findings.len() as f32).min(1.0),
            labels,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn computes_risk() {
        let computation = RiskComputation::default();
        let findings = vec![InspectionFinding {
            label: "pii".into(),
            notes: "test".into(),
            severity: 0.9,
        }];
        let profile = computation.profile(&findings);
        assert!(profile.score > 0.5);
    }
}
