use crate::{actions::ActionRequest, security_link::SecurityGrade};

use super::{
    advanced::{AdvancedSecurityAnalyzer, AdvancedSignal},
    basesecurity::{BasePolicyDecision, PolicyEffect, SecurityPolicy},
};

/// Verdict emitted by the commander.
#[derive(Debug, Clone)]
pub struct CommanderVerdict {
    /// Approved flag.
    pub approved: bool,
    /// Minimum grade required.
    pub grade: SecurityGrade,
    /// Notes justifying the verdict.
    pub notes: Vec<String>,
}

/// Aggregates base policy evaluation with advanced heuristics.
#[derive(Debug, Clone)]
pub struct SecurityCommander {
    policy: SecurityPolicy,
    analyzer: AdvancedSecurityAnalyzer,
}

impl SecurityCommander {
    /// Creates a commander with the provided components.
    #[must_use]
    pub fn new(policy: SecurityPolicy, analyzer: AdvancedSecurityAnalyzer) -> Self {
        Self { policy, analyzer }
    }

    /// Evaluates a request producing a commander verdict.
    #[must_use]
    pub fn evaluate(&self, request: &ActionRequest) -> CommanderVerdict {
        let base = self.policy.evaluate(request);
        let signals = self.analyzer.evaluate(request);
        self.combine(base, signals)
    }

    fn combine(&self, base: BasePolicyDecision, signals: Vec<AdvancedSignal>) -> CommanderVerdict {
        let mut approved = !matches!(base.effect, PolicyEffect::Deny);
        let mut grade = base.required_grade;
        let mut notes = base.notes;

        for signal in signals {
            grade = grade.max(signal.grade);
            notes.push(signal.reason);
            if signal.grade == SecurityGrade::Maximum && signal.confidence > 0.7 {
                approved = false;
                break;
            }
        }

        CommanderVerdict {
            approved,
            grade,
            notes,
        }
    }
}
