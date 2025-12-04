//! Memory runtime orchestrating short-term and long-term storage.

use std::path::PathBuf;

use anyhow::Result;

use serde_json::json;
use shared_logging::LogLevel;

use crate::{
    long_term::{LongTermMemory, MemoryLevel},
    short_term::{MemoryEntry, MemoryImportance, ShortTermMemory},
    telemetry::MemoryTelemetry,
};

/// Runtime responsible for capturing, querying, and persisting memories.
#[derive(Debug)]
pub struct MemoryRuntime {
    short_term: ShortTermMemory,
    long_term: LongTermMemory,
    telemetry: Option<MemoryTelemetry>,
}

impl MemoryRuntime {
    /// Creates a runtime with default paths and capacity.
    #[must_use]
    pub fn new() -> Self {
        Self::with_paths(ShortTermMemory::default(), LongTermMemory::default())
    }

    /// Creates a runtime from explicit components (useful for tests).
    #[must_use]
    pub fn with_paths(short_term: ShortTermMemory, long_term: LongTermMemory) -> Self {
        Self {
            short_term,
            long_term,
            telemetry: None,
        }
    }

    /// Attaches telemetry sinks.
    #[must_use]
    pub fn with_telemetry(mut self, telemetry: MemoryTelemetry) -> Self {
        self.telemetry = Some(telemetry);
        self
    }

    /// Sets telemetry after construction.
    pub fn set_telemetry(&mut self, telemetry: MemoryTelemetry) {
        self.telemetry = Some(telemetry);
    }

    /// Returns telemetry handle if configured.
    #[must_use]
    pub fn telemetry(&self) -> Option<&MemoryTelemetry> {
        self.telemetry.as_ref()
    }

    /// Captures a new memory entry, storing it in short-term memory.
    pub fn capture(
        &self,
        content: impl Into<String>,
        importance: MemoryImportance,
        tags: impl IntoIterator<Item = impl Into<String>>,
    ) -> MemoryEntry {
        let entry = MemoryEntry::new(content, importance, tags);
        self.short_term.push(entry.clone());
        if let Some(tel) = &self.telemetry {
            let _ = tel.log(
                LogLevel::Info,
                "memory.capture",
                json!({ "importance": format!("{:?}", entry.importance), "tags": entry.tags }),
            );
        }
        entry
    }

    /// Searches short-term memory by tag.
    #[must_use]
    pub fn search(&self, tag: &str) -> Vec<MemoryEntry> {
        if let Some(tel) = &self.telemetry {
            let _ = tel.log(LogLevel::Debug, "memory.search", json!({ "tag": tag }));
        }
        self.short_term.search_by_tag(tag)
    }

    /// Flushes high-importance memories to long-term storage.
    pub fn flush_high_importance(&self) -> Result<Vec<PathBuf>> {
        let drained = self
            .short_term
            .drain_filter(|entry| matches!(entry.importance, MemoryImportance::High));
        if let Some(tel) = &self.telemetry {
            let _ = tel.log(
                LogLevel::Info,
                "memory.flush.start",
                json!({ "count": drained.len() }),
            );
        }
        let mut persisted_paths = Vec::new();
        for entry in drained {
            let level = MemoryLevel::from_importance(entry.importance);
            let path = self.long_term.persist(entry, level)?;
            persisted_paths.push(path);
        }
        if let Some(tel) = &self.telemetry {
            let _ = tel.log(
                LogLevel::Info,
                "memory.flush.complete",
                json!({ "persisted": persisted_paths.len() }),
            );
            let _ = tel.event(
                "memory.flush.completed",
                json!({ "persisted": persisted_paths.len() }),
            );
        }
        Ok(persisted_paths)
    }

    /// Provides access to the underlying long-term repository.
    #[must_use]
    pub fn long_term_repo(&self) -> &LongTermMemory {
        &self.long_term
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn runtime_captures_and_flushes() {
        let dir = tempdir().unwrap();
        let short = ShortTermMemory::new(8);
        let long = LongTermMemory::new(dir.path());
        let runtime = MemoryRuntime::with_paths(short, long);
        runtime.capture(
            "Mission-critical discovery",
            MemoryImportance::High,
            ["mission", "core"],
        );
        let paths = runtime.flush_high_importance().unwrap();
        assert_eq!(paths.len(), 1);
    }
}
