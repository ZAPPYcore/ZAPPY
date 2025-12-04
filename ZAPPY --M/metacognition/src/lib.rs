#![deny(
    clippy::all,
    clippy::pedantic,
    clippy::nursery,
    missing_docs,
    rust_2018_idioms
)]

//! AGI metacognition kernel: self-monitoring, reflective planning, and execution alignment.

/// Reflection command definitions.
pub mod cmd;
/// Self-observation primitives.
pub mod cognition;
/// Helper utilities.
pub mod helper;
/// Metacognition kernel.
pub mod metacognition;
/// Reflection planning methods.
pub mod methods;
/// Review subsystems.
pub mod reviewer;
/// Script generation utilities.
pub mod script;

/// Executors used by the runtime.
#[path = "../executor.rs"]
pub mod executor;

/// Global helper utilities shared with other crates.
#[path = "../helper.rs"]
pub mod global_helper;

/// Runtime entrypoint orchestrator.
#[path = "orchestration_entry.rs"]
pub mod orchestration_entry;

/// Module definitions for metacognitive subsystems.
#[path = "../module.rs"]
pub mod module;

/// Configuration options surfaced to operators.
#[path = "../options.rs"]
pub mod options;

/// Reporting utilities for metacognition outcomes.
#[path = "../reporter.rs"]
pub mod reporter;

/// Telemetry helpers for metacognition runtime.
#[path = "../telemetry.rs"]
pub mod telemetry;
