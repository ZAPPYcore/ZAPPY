use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::define::SubsidiaryTask;
use super::saver::SubsidiaryStore;

/// Result returned when searching for subsidiary learning opportunities.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    /// Underlying task.
    pub task: SubsidiaryTask,
    /// Suitability score.
    pub score: f32,
    /// Timestamp of evaluation.
    pub evaluated_at: DateTime<Utc>,
}

/// Searches across stored tasks to select the most promising candidates.
#[derive(Debug)]
pub struct TaskSearcher {
    store: SubsidiaryStore,
}

impl TaskSearcher {
    /// Creates a new searcher.
    #[must_use]
    pub fn new(store: SubsidiaryStore) -> Self {
        Self { store }
    }

    /// Runs a search filtering by domain and priority threshold.
    pub fn search(&self, domain: &str, min_priority: u8) -> Vec<SearchResult> {
        self.store
            .tasks_by_domain(domain)
            .into_iter()
            .filter(|task| task.priority >= min_priority)
            .map(|task| SearchResult {
                score: task.priority as f32 / 10.0,
                evaluated_at: Utc::now(),
                task,
            })
            .collect()
    }
}
