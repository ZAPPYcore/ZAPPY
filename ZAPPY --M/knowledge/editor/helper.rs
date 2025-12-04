use serde::{Deserialize, Serialize};

/// Helper that rewrites knowledge bodies into concise summaries.
#[derive(Debug, Default, Clone)]
pub struct SummaryBuilder;

impl SummaryBuilder {
    /// Builds a summary limited to roughly `limit` words.
    #[must_use]
    pub fn summarize(&self, body: &str, limit: usize) -> String {
        let mut words = Vec::new();
        for word in body.split_whitespace() {
            words.push(word);
            if words.len() >= limit {
                break;
            }
        }
        words.join(" ")
    }
}

/// Represents a diff produced by the editor.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeDiff {
    /// Original snippet.
    pub before: String,
    /// Edited snippet.
    pub after: String,
    /// Rationale for the edit.
    pub rationale: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn summary_limited() {
        let builder = SummaryBuilder::default();
        let summary = builder.summarize("one two three four five", 3);
        assert_eq!(summary.split_whitespace().count(), 3);
    }
}
