#![deny(
    clippy::all,
    clippy::pedantic,
    clippy::nursery,
    missing_docs,
    rust_2018_idioms
)]

//! Zappy Tier-9 creativity engine: idea generation, evaluation, and orchestration.

/// Idea generation primitives.
#[path = "../create.rs"]
pub mod create;

/// Helper transformers and weavers.
#[path = "../helpermethod.rs"]
pub mod helpermethod;

/// Inspiration caches and prompt helpers.
#[path = "../hepler.rs"]
pub mod hepler;

/// High-level runtime entrypoints.
#[path = "../main.rs"]
pub mod orchestration_entry;

/// Kernel and control functions.
#[path = "../mainfunc/main.rs"]
pub mod mainfunc;

/// Batch helpers.
#[path = "../mainfunc/allfunc.rs"]
pub mod allfunc;

/// Advanced divergence/convergence planners.
#[path = "../mainfunc/advanced.rs"]
pub mod advanced;

/// Review board definitions.
#[path = "../mainfunc/reviewer.rs"]
pub mod reviewer;

/// Telemetry helpers.
#[path = "../telemetry.rs"]
pub mod telemetry;

pub use create::{
    CreativeBrief, CreativeConstraint, CreativeIdea, CreativeIdeaId, CreativePortfolio,
    CreativityDialect, IdeationEngine, IdeationOutcome,
};
pub use helpermethod::{IdeaTransformer, NarrativeWeaver};
pub use mainfunc::CreativityKernel;
pub use telemetry::{CreativityTelemetry, CreativityTelemetryBuilder};
