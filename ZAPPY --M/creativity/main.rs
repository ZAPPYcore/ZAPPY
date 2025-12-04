//! High-level orchestration utilities for the creativity stack.

use anyhow::Result;

use crate::{
    create::{CreativeBrief, CreativePortfolio, CreativityDialect, IdeationEngine},
    helpermethod::NarrativeWeaver,
    hepler::InspirationCache,
    reviewer::CreativeReviewBoard,
    telemetry::CreativityTelemetry,
};
use serde_json::json;
use shared_logging::LogLevel;

/// Runtime that owns stateful engines used for creativity workflows.
#[derive(Debug)]
pub struct CreativityRuntime {
    ideation: IdeationEngine,
    weaver: NarrativeWeaver,
    cache: InspirationCache,
    reviewers: CreativeReviewBoard,
    telemetry: Option<CreativityTelemetry>,
}

impl Default for CreativityRuntime {
    fn default() -> Self {
        Self {
            ideation: IdeationEngine::default(),
            weaver: NarrativeWeaver::default(),
            cache: InspirationCache::default(),
            reviewers: CreativeReviewBoard::default(),
            telemetry: None,
        }
    }
}

impl CreativityRuntime {
    /// Executes the full pipeline for the supplied brief.
    pub fn execute(&mut self, mut brief: CreativeBrief) -> Result<CreativePortfolio> {
        let brief_title = brief.title.clone();
        if let Some(tel) = &self.telemetry {
            let _ = tel.log(
                LogLevel::Info,
                "creativity.runtime.execute_start",
                json!({
                    "title": brief_title,
                    "dialect": format!("{:?}", brief.dialect),
                    "constraints": brief.constraints.complexity()
                }),
            );
            let _ = tel.event(
                "creativity.brief.received",
                json!({ "title": brief_title, "constraints": brief.constraints.complexity() }),
            );
        }

        if let Some(snippet) = self.cache.random() {
            brief = brief.with_seed(snippet);
        }

        let outcome = self.ideation.ideate(&brief)?;
        if let Some(tel) = &self.telemetry {
            let _ = tel.log(
                LogLevel::Info,
                "creativity.ideation.completed",
                json!({ "title": brief.title, "ideas": outcome.portfolio.len() }),
            );
        }

        let arc = self
            .weaver
            .weave(&outcome.portfolio.ranked(), brief.title.clone());
        self.cache.push(arc.title);

        let reviewed = self.reviewers.evaluate(outcome.portfolio.ranked());
        if let Some(tel) = &self.telemetry {
            let _ = tel.log(
                LogLevel::Info,
                "creativity.review.completed",
                json!({ "ideas": reviewed.len(), "title": brief.title }),
            );
            let _ = tel.event(
                "creativity.portfolio.completed",
                json!({ "title": brief.title, "ideas": reviewed.len() }),
            );
        }
        Ok(reviewed)
    }

    /// Attaches telemetry sinks for observability.
    #[must_use]
    pub fn with_telemetry(mut self, telemetry: CreativityTelemetry) -> Self {
        self.telemetry = Some(telemetry);
        self
    }

    /// Sets telemetry after construction.
    pub fn set_telemetry(&mut self, telemetry: CreativityTelemetry) {
        self.telemetry = Some(telemetry);
    }

    /// Returns the telemetry handle if configured.
    #[must_use]
    pub fn telemetry(&self) -> Option<&CreativityTelemetry> {
        self.telemetry.as_ref()
    }
}

/// Fires a quick sample run using a built-in brief.
pub fn sample_run() -> Result<CreativePortfolio> {
    let mut runtime = CreativityRuntime::default();
    let brief = CreativeBrief::new(
        "Aqueous Cities",
        "Design rituals for floating societies",
        CreativityDialect::Poetic,
    )
    .with_seed("Compose a sonic lighthouse for displaced coral reefs");
    runtime.execute(brief)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sample_run_produces_portfolio() {
        let portfolio = sample_run().unwrap();
        assert!(portfolio.len() > 0);
    }
}
