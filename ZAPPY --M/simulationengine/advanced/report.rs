use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::{methods::SimulationMethod, simulator::SimulationBatch};

use super::thinking::ScenarioInsight;

/// Structured report summarizing a simulation batch.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulationReport {
    /// Method used.
    pub method: SimulationMethod,
    /// Number of scenarios.
    pub scenario_count: usize,
    /// Generated insights.
    pub insights: Vec<ScenarioInsight>,
    /// Timestamp.
    pub generated_at: DateTime<Utc>,
}

/// Builder for `SimulationReport`.
pub struct SimulationReportBuilder<'a> {
    method: SimulationMethod,
    batch: Option<&'a SimulationBatch>,
    insights: Vec<ScenarioInsight>,
}

impl<'a> SimulationReportBuilder<'a> {
    /// Creates builder.
    #[must_use]
    pub fn new() -> Self {
        Self {
            method: SimulationMethod::Approximate,
            batch: None,
            insights: Vec::new(),
        }
    }

    /// Sets method.
    #[must_use]
    pub fn method(mut self, method: SimulationMethod) -> Self {
        self.method = method;
        self
    }

    /// Sets batch.
    #[must_use]
    pub fn batch(mut self, batch: &'a SimulationBatch) -> Self {
        self.batch = Some(batch);
        self
    }

    /// Sets insights.
    #[must_use]
    pub fn insights(mut self, insights: Vec<ScenarioInsight>) -> Self {
        self.insights = insights;
        self
    }

    /// Builds report.
    pub fn build(self) -> SimulationReport {
        let scenario_count = self.batch.map(|b| b.scenarios.len()).unwrap_or(0);
        SimulationReport {
            method: self.method,
            scenario_count,
            insights: self.insights,
            generated_at: Utc::now(),
        }
    }
}
