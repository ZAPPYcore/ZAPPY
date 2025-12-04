use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::saver::{KnowledgeRecord, KnowledgeStore};

/// Query object describing the user's need.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeQuery {
    /// Raw query string.
    pub text: String,
    /// Optional domain hint.
    pub domain: Option<String>,
}

impl KnowledgeQuery {
    /// Creates a new query.
    #[must_use]
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            domain: None,
        }
    }
}

/// Short snippet returned to callers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeSnippet {
    /// Record identifier.
    pub record_id: uuid::Uuid,
    /// Highlighted text.
    pub excerpt: String,
    /// Score between 0 and 1.
    pub score: f32,
    /// Timestamp when snippet was produced.
    pub generated_at: DateTime<Utc>,
}

/// Seeker that queries the knowledge store.
#[derive(Debug, Clone)]
pub struct KnowledgeSeeker {
    store: KnowledgeStore,
}

impl KnowledgeSeeker {
    /// Creates a new seeker.
    #[must_use]
    pub fn new(store: KnowledgeStore) -> Self {
        Self { store }
    }

    /// Executes the query and returns snippets.
    pub fn search(&self, query: KnowledgeQuery) -> Vec<KnowledgeSnippet> {
        let mut records = self.store.find_by_keyword(&query.text);
        if records.is_empty() {
            records = self.store.latest(3);
        }

        let mut snippets = Vec::new();
        for record in records {
            let excerpt = extract_excerpt(&record.body, &query.text);
            snippets.push(KnowledgeSnippet {
                record_id: record.id,
                excerpt,
                score: score_record(&record, &query),
                generated_at: Utc::now(),
            });
        }
        snippets
    }
}

fn extract_excerpt(body: &str, needle: &str) -> String {
    let haystack_lower = body.to_lowercase();
    let needle_lower = needle.to_lowercase();
    if let Some(byte_idx) = haystack_lower.find(&needle_lower) {
        let total_chars = body.chars().count();
        let prefix_chars = haystack_lower[..byte_idx].chars().count();
        let match_chars = haystack_lower[byte_idx..byte_idx + needle_lower.len()]
            .chars()
            .count();
        let context = 40;
        let start_char = prefix_chars.saturating_sub(context);
        let end_char = (prefix_chars + match_chars + context).min(total_chars);
        return body
            .chars()
            .skip(start_char)
            .take(end_char - start_char)
            .collect();
    }
    body.split('.').next().unwrap_or(body).to_string()
}

fn score_record(record: &KnowledgeRecord, query: &KnowledgeQuery) -> f32 {
    let mut score: f32 = 0.5;
    if record
        .title
        .to_lowercase()
        .contains(&query.text.to_lowercase())
    {
        score += 0.3;
    }
    if let Some(domain) = &query.domain {
        if record
            .metadata
            .get("category")
            .and_then(|v| v.as_str())
            .map_or(false, |cat| cat == domain)
        {
            score += 0.2;
        }
    }
    score.min(1.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::saver::KnowledgeRecord;

    #[test]
    fn seeker_returns_snippet() {
        let store = KnowledgeStore::default();
        store.insert(
            KnowledgeRecord::new(
                "src",
                "Rust Memory",
                "Ownership makes data races impossible",
            )
            .with_metadata("category", serde_json::json!("systems")),
        );
        let seeker = KnowledgeSeeker::new(store);
        let snippets = seeker.search(KnowledgeQuery::new("ownership"));
        assert!(!snippets.is_empty());
    }
}
