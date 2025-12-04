#![deny(
    clippy::all,
    clippy::pedantic,
    clippy::nursery,
    rust_2018_idioms,
    missing_docs
)]

//! Tier-10 reasoning runtime: multi-domain inference + telemetry.

/// Telemetry builder/hook for reasoning components.
#[path = "../telemetry.rs"]
pub mod telemetry;

/// Domain-neutral data structures.
#[path = "../module.rs"]
pub mod module;

/// Core inference engine.
#[path = "../engine.rs"]
pub mod engine;

/// Multi-domain coordinator.
#[path = "../multidomain/main.rs"]
pub mod multidomain;

/// Reasoning runtime entry point.
#[path = "../main.rs"]
pub mod runtime;

pub use engine::{InferenceEngine, InferenceResult, SignalGraph};
pub use module::{ReasoningDirective, ReasoningHypothesis, SignalPacket, Verdict};
pub use runtime::ReasoningRuntime;
pub use telemetry::{ReasoningTelemetry, ReasoningTelemetryBuilder};
