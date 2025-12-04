use std::sync::Arc;

use indexmap::IndexMap;
use parking_lot::RwLock;
use rand::{seq::SliceRandom, thread_rng};

/// Caches inspiration snippets gathered from previous sessions.
#[derive(Debug, Default, Clone)]
pub struct InspirationCache {
    inner: Arc<RwLock<Vec<String>>>,
}

impl InspirationCache {
    /// Adds a snippet to the cache.
    pub fn push(&self, snippet: impl Into<String>) {
        self.inner.write().push(snippet.into());
    }

    /// Returns a random snippet if available.
    #[must_use]
    pub fn random(&self) -> Option<String> {
        let cache = self.inner.read();
        cache.choose(&mut thread_rng()).cloned()
    }
}

/// Builds creative prompts from structured metadata.
#[derive(Debug, Default)]
pub struct PromptHepler;

impl PromptHepler {
    /// Generates a prompt string from the provided tags.
    #[must_use]
    pub fn forge(tags: &IndexMap<String, String>) -> String {
        let mut prompt = String::new();
        for (key, value) in tags {
            prompt.push_str(&format!("{key}: {value}; "));
        }
        prompt.push_str("Compose a transcendent artifact.");
        prompt
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cache_returns_random_item() {
        let cache = InspirationCache::default();
        cache.push("seed");
        assert!(cache.random().is_some());
    }

    #[test]
    fn prompt_helper_serializes_tags() {
        let mut tags = IndexMap::new();
        tags.insert("tone".into(), "playful".into());
        let prompt = PromptHepler::forge(&tags);
        assert!(prompt.contains("tone"));
    }
}
