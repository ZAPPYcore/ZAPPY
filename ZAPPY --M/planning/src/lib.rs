#![deny(
    clippy::all,
    clippy::pedantic,
    clippy::nursery,
    missing_docs,
    rust_2018_idioms
)]

//! Tier-10 planning runtime combining strategic (long-term) and tactical (short-term) engines.

/// Long-term planning engine and data structures.
#[path = "../long_term/main.rs"]
pub mod long_term;
/// Planning module utilities (signals, directives).
#[path = "../module.rs"]
pub mod module;
/// Planning runtime orchestration entry points.
#[path = "../main.rs"]
pub mod orchestration_entry;
/// Short-term planning engine.
#[path = "../short_term/main.rs"]
pub mod short_term;
/// Telemetry helpers for planning.
#[path = "../telemetry.rs"]
pub mod telemetry;

pub use long_term::{LongTermPlanner, PlanPhase, StrategicObjective, StrategicPlan};
pub use module::{PlanningDirective, PlanningSignal, PriorityBand};
pub use orchestration_entry::PlanningRuntime;
pub use short_term::{ShortTermPlanner, TacticalSchedule, TacticalTask};
pub use telemetry::{PlanningTelemetry, PlanningTelemetryBuilder};
