use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::json;
use shared_logging::LogLevel;

use crate::{
    create::{CreativeBrief, CreativePortfolio, CreativityDialect},
    orchestration_entry::CreativityRuntime,
};

/// Snapshot of a creativity cycle.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreativityCycle {
    /// Timestamp when cycle started.
    pub started_at: DateTime<Utc>,
    /// Title associated with the brief.
    pub brief_title: String,
    /// Number of ideas generated.
    pub ideas: usize,
}

/// High-level kernel that coordinates runtime executions and tracks history.
#[derive(Debug, Default)]
pub struct CreativityKernel {
    runtime: CreativityRuntime,
    history: Vec<CreativityCycle>,
}

impl CreativityKernel {
    /// Creates a new kernel.
    #[must_use]
    pub fn new(runtime: CreativityRuntime) -> Self {
        Self {
            runtime,
            history: Vec::new(),
        }
    }

    /// Runs a cycle with the provided brief.
    pub fn run_cycle(&mut self, brief: CreativeBrief) -> anyhow::Result<CreativePortfolio> {
        let started_at = Utc::now();
        let title = brief.title.clone();
        if let Some(tel) = self.runtime.telemetry() {
            let _ = tel.log(
                LogLevel::Info,
                "creativity.kernel.cycle_start",
                json!({ "title": title }),
            );
        }
        let portfolio = self.runtime.execute(brief)?;
        self.history.push(CreativityCycle {
            started_at,
            brief_title: title.clone(),
            ideas: portfolio.len(),
        });
        if let Some(tel) = self.runtime.telemetry() {
            let _ = tel.log(
                LogLevel::Info,
                "creativity.kernel.cycle_complete",
                json!({ "title": title, "ideas": portfolio.len() }),
            );
        }
        Ok(portfolio)
    }

    /// Returns execution history.
    #[must_use]
    pub fn history(&self) -> &[CreativityCycle] {
        &self.history
    }
}

/// Convenience helper that builds a canonical brief.
#[must_use]
pub fn canonical_brief(title: &str) -> CreativeBrief {
    CreativeBrief::new(
        title,
        "Imagine regenerative interactions between humans and infrastructure",
        CreativityDialect::Experimental,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kernel_tracks_history() {
        let runtime = CreativityRuntime::default();
        let mut kernel = CreativityKernel::new(runtime);
        let portfolio = kernel.run_cycle(canonical_brief("Test City")).unwrap();
        assert!(portfolio.len() > 0);
        assert_eq!(kernel.history().len(), 1);
    }
}
