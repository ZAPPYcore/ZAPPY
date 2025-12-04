use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::{comprehension::ComprehensionResult, telemetry::NlpTelemetry};

/// Synthesized answer artifact emitted by the NLP stack.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnswerDraft {
    /// Final answer text.
    pub content: String,
    /// Supporting justification string.
    pub justification: String,
    /// Confidence between 0-1.
    pub confidence: f32,
}

/// Generates production-ready answers based on comprehension results.
pub struct AnswerGenerator {
    telemetry: Option<NlpTelemetry>,
}

impl AnswerGenerator {
    /// Creates a new generator.
    #[must_use]
    pub fn new(telemetry: Option<NlpTelemetry>) -> Self {
        Self { telemetry }
    }

    /// Synthesizes an answer for a single question.
    pub fn synthesize(&self, question: &str, comprehension: &ComprehensionResult) -> AnswerDraft {
        let content = if comprehension.ranked.is_empty() {
            format!("No reliable evidence available to answer: {}", question)
        } else {
            format!(
                "{} Based on evidence: {}",
                question, comprehension.justification
            )
        };
        let confidence = comprehension
            .ranked
            .first()
            .map(|score| score.score)
            .unwrap_or(0.0);
        let draft = AnswerDraft {
            content,
            justification: comprehension.justification.clone(),
            confidence,
        };
        if let Some(tel) = &self.telemetry {
            let _ = tel.log(
                shared_logging::LogLevel::Info,
                "nlp.answer.generated",
                json!({
                    "confidence": draft.confidence,
                    "method": comprehension.method.label(),
                }),
            );
        }
        draft
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::comprehension::{ComprehensionMethod, SentenceScore};

    #[test]
    fn generator_returns_answer() {
        let generator = AnswerGenerator::new(None);
        let result = ComprehensionResult {
            method: ComprehensionMethod::Extractive,
            ranked: vec![SentenceScore {
                sentence: "Rust has zero-cost abstractions.".into(),
                score: 0.8,
            }],
            justification: "Rust has zero-cost abstractions.".into(),
        };
        let answer = generator.synthesize("Tell me about Rust", &result);
        assert!(answer.content.contains("Rust"));
        assert!(answer.confidence > 0.7);
    }
}
