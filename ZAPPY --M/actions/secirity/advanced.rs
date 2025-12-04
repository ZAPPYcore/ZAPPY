use regex::Regex;

use crate::{
    actions::{ActionIntent, ActionRequest},
    security_link::SecurityGrade,
};

/// Signal raised by the advanced analyzer.
#[derive(Debug, Clone)]
pub struct AdvancedSignal {
    /// Reason explaining the flag.
    pub reason: String,
    /// Grade required to proceed.
    pub grade: SecurityGrade,
    /// Confidence between 0 and 1.
    pub confidence: f32,
}

/// High-dimensional heuristic analyzer that looks for unsafe content.
#[derive(Debug, Clone)]
pub struct AdvancedSecurityAnalyzer {
    patterns: Vec<(Regex, SecurityGrade, &'static str)>,
}

impl AdvancedSecurityAnalyzer {
    /// Constructs the analyzer with hardened defaults.
    #[must_use]
    pub fn hardened() -> Self {
        Self {
            patterns: vec![
                (
                    Regex::new("(?i)unauthorized access|privilege escalation").unwrap(),
                    SecurityGrade::High,
                    "Privilege escalation keywords detected",
                ),
                (
                    Regex::new("(?i)zero\\s*day|exploit kit").unwrap(),
                    SecurityGrade::Maximum,
                    "Potential exploit development",
                ),
                (
                    Regex::new("(?i)delete all|drop database").unwrap(),
                    SecurityGrade::High,
                    "Destructive action request",
                ),
            ],
        }
    }

    /// Evaluates the request returning threat signals.
    #[must_use]
    pub fn evaluate(&self, request: &ActionRequest) -> Vec<AdvancedSignal> {
        let mut signals = Vec::new();
        let payload = format!(
            "{}\n{}\n{}",
            request.payload.summary,
            request.payload.narrative,
            request.intent_label()
        );

        for (pattern, grade, note) in &self.patterns {
            if pattern.is_match(&payload) {
                signals.push(AdvancedSignal {
                    reason: note.to_string(),
                    grade: *grade,
                    confidence: 0.85,
                });
            }
        }

        if matches!(
            request.intent,
            ActionIntent::Execute | ActionIntent::Remediate
        ) && matches!(request.domain, crate::actions::ActionDomain::Security)
        {
            signals.push(AdvancedSignal {
                reason: "Security domain execute intent".into(),
                grade: SecurityGrade::High,
                confidence: 0.65,
            });
        }

        signals
    }
}

trait IntentLabel {
    fn intent_label(&self) -> String;
}

impl IntentLabel for ActionRequest {
    fn intent_label(&self) -> String {
        format!("{:?}", self.intent)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actions::{ActionDomain, ActionIntent, ActionPayload, ActionRequest};

    #[test]
    fn detects_privilege_escalation() {
        let payload = ActionPayload::textual("Test", "Attempt privilege escalation");
        let request =
            ActionRequest::builder(ActionDomain::Security, ActionIntent::Execute, payload).build();

        let analyzer = AdvancedSecurityAnalyzer::hardened();
        let signals = analyzer.evaluate(&request);
        assert!(!signals.is_empty());
    }
}
