#![deny(
    clippy::all,
    clippy::pedantic,
    clippy::nursery,
    missing_docs,
    rust_2018_idioms
)]

//! Tier-9 AGI memory orchestration (short-term + long-term persistence).

/// Long-term memory persistence utilities.
pub mod long_term;
/// Short-term, high-speed buffer.
pub mod short_term;

/// Telemetry helpers for memory orchestration.
#[path = "../telemetry.rs"]
pub mod telemetry;

#[path = "../main.rs"]
pub mod orchestration_entry;

pub use long_term::{LongTermMemory, MemoryLevel};
pub use short_term::{MemoryEntry, MemoryImportance, ShortTermMemory};
pub use telemetry::{MemoryTelemetry, MemoryTelemetryBuilder};
