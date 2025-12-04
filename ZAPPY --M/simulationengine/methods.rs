use serde::{Deserialize, Serialize};

/// Supported simulation methods.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum SimulationMethod {
    /// Fast approximate simulation.
    Approximate,
    /// High fidelity multi-step simulation.
    HighFidelity,
}

impl SimulationMethod {
    /// Label for logging.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Approximate => "approximate",
            Self::HighFidelity => "high_fidelity",
        }
    }

    /// Returns step count multiplier.
    #[must_use]
    pub fn step_multiplier(self) -> usize {
        match self {
            Self::Approximate => 1,
            Self::HighFidelity => 3,
        }
    }
}
