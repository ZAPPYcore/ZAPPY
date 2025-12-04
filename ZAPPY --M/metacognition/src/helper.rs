use serde::{Deserialize, Serialize};

/// Utility for categorizing metacognitive actions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetaTagger;

impl MetaTagger {
    /// Derives tags from the input description.
    #[must_use]
    pub fn derive_tags(description: &str) -> Vec<String> {
        let mut tags = Vec::new();
        if description.contains("latency") {
            tags.push("performance".into());
        }
        if description.contains("accuracy") {
            tags.push("quality".into());
        }
        if tags.is_empty() {
            tags.push("general".into());
        }
        tags
    }
}
