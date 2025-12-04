#![deny(
    clippy::all,
    clippy::pedantic,
    clippy::nursery,
    rust_2018_idioms,
    missing_docs
)]

//! Self-upgrade module orchestrating diagnostics, planning, and execution with telemetry.

/// Diagnostic checker implementations.
#[path = "../checker.rs"]
pub mod checker;
/// Telemetry helpers and filesystem utilities.
#[path = "../helpermethods.rs"]
pub mod helpermethods;
/// Domain models for upgrade directives.
#[path = "../module.rs"]
pub mod module;
/// Planner and executor.
#[path = "../selfupgarde.rs"]
pub mod planner;
/// Reporter for upgrade actions.
#[path = "../reporter.rs"]
pub mod reporter;
/// Plan reviewer.
#[path = "../reviewer.rs"]
pub mod reviewer;
/// Runtime entry.
#[path = "../main.rs"]
pub mod runtime;

pub use helpermethods::{UpgradeTelemetry, UpgradeTelemetryBuilder};
pub use module::{UpgradeAction, UpgradeDirective, UpgradeFinding, UpgradePlan, UpgradeStatus};
pub use runtime::{SelfUpgradeRuntime, SelfUpgradeRuntimeBuilder};
