use anyhow::Result;
use async_trait::async_trait;
use serde::Deserialize;

use crate::long_term::StrategicObjective;

/// Provides access to strategic objectives.
#[async_trait]
pub trait ObjectiveSource: Send + Sync {
    /// Fetches objectives.
    async fn fetch(&self) -> Result<Vec<StrategicObjective>>;
}

/// In-memory source used for tests/runtime bridging.
pub struct InMemoryObjectiveSource {
    objectives: Vec<StrategicObjective>,
}

impl InMemoryObjectiveSource {
    /// Creates source.
    #[must_use]
    pub fn new(objectives: Vec<StrategicObjective>) -> Self {
        Self { objectives }
    }
}

#[async_trait]
impl ObjectiveSource for InMemoryObjectiveSource {
    async fn fetch(&self) -> Result<Vec<StrategicObjective>> {
        Ok(self.objectives.clone())
    }
}

/// Loads objectives from JSON file.
#[derive(Debug, Deserialize)]
pub struct FileObjectiveRecord {
    /// Description.
    pub description: String,
    /// Target horizon.
    pub horizon_weeks: u16,
    /// Priority.
    pub priority: u8,
}

/// Reads objectives from a JSON file.
pub struct FileObjectiveSource {
    path: std::path::PathBuf,
}

impl FileObjectiveSource {
    /// Creates source from path.
    #[must_use]
    pub fn new(path: impl Into<std::path::PathBuf>) -> Self {
        Self { path: path.into() }
    }
}

#[async_trait]
impl ObjectiveSource for FileObjectiveSource {
    async fn fetch(&self) -> Result<Vec<StrategicObjective>> {
        let data = tokio::fs::read_to_string(&self.path).await?;
        let records: Vec<FileObjectiveRecord> = serde_json::from_str(&data)?;
        Ok(records
            .into_iter()
            .map(|record| {
                StrategicObjective::new(record.description, record.priority, record.horizon_weeks)
            })
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn in_memory_returns_objectives() {
        let source = InMemoryObjectiveSource::new(vec![StrategicObjective::new("test", 70, 16)]);
        assert_eq!(source.fetch().await.unwrap().len(), 1);
    }
}
