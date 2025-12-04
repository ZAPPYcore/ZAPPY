#![deny(
    clippy::all,
    clippy::pedantic,
    clippy::nursery,
    rust_2018_idioms,
    missing_docs
)]

//! Zappy Simulation Engine – generates synthetic environments, runs predictors, and validates outcomes.

/// Telemetry for simulation engine.
#[path = "../helper.rs"]
pub mod helper;

/// Simulation environment generator definitions.
#[path = "../simul_env_generator.rs"]
pub mod simul_env_generator;

/// Scenario predictor pipeline.
#[path = "../predictor.rs"]
pub mod predictor;

/// Simulator orchestrations.
#[path = "../simulator.rs"]
pub mod simulator;

/// Comparative analysis utilities.
#[path = "../compare.rs"]
pub mod compare;

/// Reviewer logic for simulation outcomes.
#[path = "../reviewer.rs"]
pub mod reviewer;

/// Methods catalogue.
#[path = "../methods.rs"]
pub mod methods;

/// Advanced simulation stack.
#[path = "../advanced/main.rs"]
pub mod advanced;

/// Runtime entry & CLI hooks.
#[path = "../main.rs"]
pub mod runtime;

pub use helper::{SimulationTelemetry, SimulationTelemetryBuilder};
pub use runtime::{SimulationEngine, SimulationEngineBuilder};
