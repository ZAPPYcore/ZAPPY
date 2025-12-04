use anyhow::Result;
use serde_json::json;

use crate::{helper::SimulationTelemetry, methods::SimulationMethod, simulator::Simulator};

use super::{report::SimulationReportBuilder, thinking::ScenarioThinker};

/// High fidelity simulator that post-processes batches with advanced thinking/reporting.
pub struct AdvancedSimulator {
    simulator: Simulator,
    thinker: ScenarioThinker,
    telemetry: Option<SimulationTelemetry>,
}

impl AdvancedSimulator {
    /// Creates a new advanced simulator.
    #[must_use]
    pub fn new(
        simulator: Simulator,
        thinker: ScenarioThinker,
        telemetry: Option<SimulationTelemetry>,
    ) -> Self {
        Self {
            simulator,
            thinker,
            telemetry,
        }
    }

    /// Runs simulation with thinking/reporting pipeline.
    pub async fn run(
        &self,
        method: SimulationMethod,
        count: usize,
    ) -> Result<super::report::SimulationReport> {
        let batch = self.simulator.run(method, count).await?;
        let insights = self.thinker.analyze(&batch)?;
        let report = SimulationReportBuilder::new()
            .method(method)
            .batch(&batch)
            .insights(insights)
            .build();
        if let Some(tel) = &self.telemetry {
            let _ = tel.log(
                shared_logging::LogLevel::Info,
                "simulation.advanced.report_generated",
                json!({ "method": method.label(), "count": count }),
            );
        }
        Ok(report)
    }
}
