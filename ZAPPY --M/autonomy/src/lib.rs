#![deny(
    clippy::all,
    clippy::pedantic,
    clippy::nursery,
    missing_docs,
    rust_2018_idioms
)]

//! Zappy Tier-9 autonomy kernel combining decision, module, and master control loops.

/// Decision engines and reviewers.
#[path = "../decision/main.rs"]
pub mod decision;

/// Cycle linker coordinating director and master.
#[path = "../linker.rs"]
pub mod linker;

/// Master control loop components.
#[path = "../master/main.rs"]
pub mod master;

/// Module registry, brokers, and helpers.
#[path = "../module/main.rs"]
pub mod module;

/// Telemetry helpers.
#[path = "../telemetry.rs"]
pub mod telemetry;

/// Runtime entrypoints and orchestration helpers.
#[path = "../main.rs"]
pub mod orchestration_entry;

pub use decision::decisionmaking::DecisionInput;
pub use decision::{DecisionDirector, DecisionVerdict};
pub use linker::{AutonomyLinker, CycleReport};
pub use master::{MasterController, MasterMetrics};
pub use module::{
    AutonomyError, AutonomySignal, ControlDirective, DirectivePriority, ModuleBroker, ModuleKind,
    ModulePulse, ModuleRegistry, ModuleSpec, ModuleTarget, SignalScope,
};
pub use orchestration_entry::AutonomyRuntime;
pub use telemetry::{AutonomyTelemetry, AutonomyTelemetryBuilder};
