use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::time::{sleep, Duration};

/// Available web search channels.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SearchChannel {
    /// General purpose search engine.
    General,
    /// Academic sources.
    Academic,
    /// Social sentiment.
    Social,
}

/// Search result from a provider.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    /// Title of the hit.
    pub title: String,
    /// URL referencing the source.
    pub url: String,
    /// Summary text.
    pub summary: String,
    /// Timestamp when fetched.
    pub fetched_at: DateTime<Utc>,
}

/// Errors emitted while calling web search APIs.
#[derive(Debug, Error)]
pub enum WebSearchError {
    /// Provider failure.
    #[error("provider error: {0}")]
    Provider(String),
}

/// Abstract client that hits external APIs.
#[async_trait]
pub trait WebSearchClient: Send + Sync {
    /// Executes the search returning results.
    async fn search(
        &self,
        channel: SearchChannel,
        query: &str,
    ) -> Result<Vec<SearchResult>, WebSearchError>;
}

/// Loopback client for tests and offline inference.
#[derive(Debug, Default)]
pub struct LoopbackWebClient;

#[async_trait]
impl WebSearchClient for LoopbackWebClient {
    async fn search(
        &self,
        channel: SearchChannel,
        query: &str,
    ) -> Result<Vec<SearchResult>, WebSearchError> {
        sleep(Duration::from_millis(10)).await;
        Ok(vec![SearchResult {
            title: format!("{} insight", query),
            url: format!("https://example.com/{:?}/{}", channel, query),
            summary: format!("Auto-generated summary for {}", query),
            fetched_at: Utc::now(),
        }])
    }
}

/// Orchestrator that fans out across multiple channels.
#[derive(Clone)]
pub struct WebSearcher {
    client: Arc<dyn WebSearchClient>,
    channels: Vec<SearchChannel>,
}

impl std::fmt::Debug for WebSearcher {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WebSearcher")
            .field("channels", &self.channels)
            .finish()
    }
}

impl WebSearcher {
    /// Creates a new web searcher.
    #[must_use]
    pub fn new(client: Arc<dyn WebSearchClient>) -> Self {
        Self {
            client,
            channels: vec![SearchChannel::General, SearchChannel::Academic],
        }
    }

    /// Adds an additional channel.
    #[must_use]
    pub fn with_channel(mut self, channel: SearchChannel) -> Self {
        self.channels.push(channel);
        self
    }

    /// Executes multi-channel search.
    pub async fn search(&self, query: &str) -> Result<Vec<SearchResult>, WebSearchError> {
        let mut tasks = Vec::new();
        for channel in &self.channels {
            let client = Arc::clone(&self.client);
            let channel = channel.clone();
            let query = query.to_string();
            tasks.push(tokio::spawn(
                async move { client.search(channel, &query).await },
            ));
        }

        let mut results = Vec::new();
        for task in tasks {
            let chunk = task
                .await
                .unwrap_or_else(|err| Err(WebSearchError::Provider(err.to_string())))?;
            results.extend(chunk);
        }
        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn aggregator_returns_results() {
        let searcher = WebSearcher::new(Arc::new(LoopbackWebClient));
        let results = searcher.search("zappy").await.unwrap();
        assert!(!results.is_empty());
    }
}
