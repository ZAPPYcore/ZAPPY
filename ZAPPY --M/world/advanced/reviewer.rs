use anyhow::Result;
use serde_json::json;

use crate::{model::WorldState, telemetry::WorldTelemetry};

/// Reviews world states and emits governance alerts.
pub struct StateReviewer {
    telemetry: Option<WorldTelemetry>,
    critical_threshold: f32,
}

impl StateReviewer {
    /// Creates reviewer.
    #[must_use]
    pub fn new(telemetry: Option<WorldTelemetry>) -> Self {
        Self {
            telemetry,
            critical_threshold: 1.1,
        }
    }

    /// Reviews state and returns whether action is needed.
    pub fn review(&self, state: &WorldState) -> Result<bool> {
        let decision = state
            .highest_severity()
            .map(|anom| anom.severity >= self.critical_threshold)
            .unwrap_or(false);
        if let Some(tel) = &self.telemetry {
            let _ = tel.event(
                "world.state.reviewed",
                json!({
                    "regions": state.regions.len(),
                    "anomalies": state.anomalies.len(),
                    "action_required": decision
                }),
            );
        }
        Ok(decision)
    }
}
