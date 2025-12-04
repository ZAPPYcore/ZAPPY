//! Advanced simulation stack: deep scenario thinking + report generation.

/// High-level advanced simulator orchestrations.
pub mod advanced_simulator;
/// Report builder utilities.
pub mod report;
/// Scenario thinking/introspection utilities.
pub mod thinking;

pub use advanced_simulator::AdvancedSimulator;
pub use report::{SimulationReport, SimulationReportBuilder};
pub use thinking::{ScenarioInsight, ScenarioThinker};
