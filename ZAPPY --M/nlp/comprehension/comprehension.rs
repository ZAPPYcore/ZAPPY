use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::comprehension::{
    algo::{rank_sentences, SentenceScore},
    method::ComprehensionMethod,
};

/// Document provided to comprehension engine.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvidencePassage {
    /// Unique passage id.
    pub id: Uuid,
    /// Passage body.
    pub content: String,
    /// Metadata e.g., source.
    pub metadata: serde_json::Value,
}

/// Request object for comprehension analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComprehensionRequest {
    /// Natural language question.
    pub question: String,
    /// Evidence passages.
    pub passages: Vec<EvidencePassage>,
    /// Preferred method.
    pub method: ComprehensionMethod,
}

/// Result describing the best supporting sentences.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComprehensionResult {
    /// Method used.
    pub method: ComprehensionMethod,
    /// Ranked sentences.
    pub ranked: Vec<SentenceScore>,
    /// Aggregated justification text.
    pub justification: String,
}

/// Engine performing multi-document comprehension.
#[derive(Clone)]
pub struct ComprehensionEngine {
    top_k: usize,
    min_score: f32,
}

impl ComprehensionEngine {
    /// Creates a new engine.
    #[must_use]
    pub fn new(top_k: usize, min_score: f32) -> Self {
        Self { top_k, min_score }
    }

    /// Runs comprehension and returns ranked evidence.
    pub fn analyze(&self, request: &ComprehensionRequest) -> ComprehensionResult {
        let mut global_ranked = Vec::new();
        for passage in &request.passages {
            let ranked = rank_sentences(&passage.content, &request.question);
            global_ranked.extend(ranked);
        }
        global_ranked.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
        let filtered: Vec<SentenceScore> = global_ranked
            .into_iter()
            .filter(|score| score.score >= self.min_score)
            .take(self.top_k)
            .collect();
        let justification = filtered
            .iter()
            .map(|s| s.sentence.clone())
            .collect::<Vec<_>>()
            .join(" ");
        ComprehensionResult {
            method: request.method,
            ranked: filtered,
            justification,
        }
    }
}

impl Default for ComprehensionEngine {
    fn default() -> Self {
        Self::new(5, 0.2)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn engine_returns_ranked_sentences() {
        let engine = ComprehensionEngine::default();
        let request = ComprehensionRequest {
            question: "borrow checker".into(),
            method: ComprehensionMethod::Extractive,
            passages: vec![EvidencePassage {
                id: Uuid::new_v4(),
                content: "Rust has a borrow checker. C++ does not.".into(),
                metadata: json!({"source": "doc"}),
            }],
        };
        let result = engine.analyze(&request);
        assert!(!result.ranked.is_empty());
    }
}
