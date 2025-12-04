use regex::Regex;
use serde::{Deserialize, Serialize};

/// Single finding emitted by the content inspector.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InspectionFinding {
    /// Type of issue detected.
    pub label: String,
    /// Human readable notes.
    pub notes: String,
    /// Severity between 0 and 1.
    pub severity: f32,
}

/// Performs lightweight inspection on incoming content.
#[derive(Debug, Clone)]
pub struct ContentInspector {
    sensitive_regex: Regex,
    pii_regex: Regex,
}

impl Default for ContentInspector {
    fn default() -> Self {
        Self {
            sensitive_regex: Regex::new("(?i)(top secret|classified|internal use only)").unwrap(),
            pii_regex: Regex::new(r"(?i)\b\d{3}-\d{2}-\d{4}\b").unwrap(),
        }
    }
}

impl ContentInspector {
    /// Runs inspection routines and returns findings.
    #[must_use]
    pub fn inspect(&self, content: &str) -> Vec<InspectionFinding> {
        let mut findings = Vec::new();
        if self.sensitive_regex.is_match(content) {
            findings.push(InspectionFinding {
                label: "sensitive_phrase".into(),
                notes: "Sensitive phrase detected".into(),
                severity: 0.7,
            });
        }
        if self.pii_regex.is_match(content) {
            findings.push(InspectionFinding {
                label: "pii".into(),
                notes: "Possible PII detected".into(),
                severity: 0.9,
            });
        }
        findings
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inspector_detects_sensitive_text() {
        let inspector = ContentInspector::default();
        let findings = inspector.inspect("This is top secret information.");
        assert!(!findings.is_empty());
    }
}
