use crate::{module::ModuleBroker, telemetry::AutonomyTelemetry};

use super::MasterController;

/// Builder used to configure a [`MasterController`].
#[derive(Debug, Clone)]
pub struct MasterControllerBuilder {
    broker: ModuleBroker,
    max_inflight: usize,
    telemetry: Option<AutonomyTelemetry>,
}

impl MasterControllerBuilder {
    /// Creates a new builder.
    #[must_use]
    pub fn new(broker: ModuleBroker) -> Self {
        Self {
            broker,
            max_inflight: 8,
            telemetry: None,
        }
    }

    /// Overrides the maximum number of directives that can be issued per cycle.
    #[must_use]
    pub fn max_inflight(mut self, max_inflight: usize) -> Self {
        self.max_inflight = max_inflight.max(1);
        self
    }

    /// Attaches telemetry used by the master controller.
    #[must_use]
    pub fn telemetry(mut self, telemetry: AutonomyTelemetry) -> Self {
        self.telemetry = Some(telemetry);
        self
    }

    /// Finalizes the configuration.
    #[must_use]
    pub fn build(self) -> MasterController {
        let mut controller = MasterController::new(self.broker, self.max_inflight);
        if let Some(tel) = self.telemetry {
            controller = controller.with_telemetry(tel);
        }
        controller
    }
}
