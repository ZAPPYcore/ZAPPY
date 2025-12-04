#![deny(
    clippy::all,
    clippy::pedantic,
    clippy::nursery,
    missing_docs,
    rust_2018_idioms
)]

//! Zappy Tier-9 knowledge ingestion and curation stack.

/// Knowledge receivers that normalize inbound artifacts.
#[path = "../receiver.rs"]
pub mod receiver;

/// Persistent storage utilities.
#[path = "../saver.rs"]
pub mod saver;

/// Local corpus seeker utilities.
#[path = "../seeker.rs"]
pub mod seeker;

/// External web search orchestration.
#[path = "../websearcher.rs"]
pub mod websearcher;

/// Knowledge editing workflows.
#[path = "../editor/main.rs"]
pub mod editor;

/// Knowledge security orchestrator.
#[path = "../security/main.rs"]
pub mod security;

/// Telemetry helpers.
#[path = "../telemetry.rs"]
pub mod telemetry;

// security submodules are included inside `security::`.

/// High-level orchestration entry point.
#[path = "../main.rs"]
pub mod orchestration_entry;

pub use editor::editor::{EditOperation, KnowledgeEditor};
pub use orchestration_entry::KnowledgeRuntime;
pub use receiver::{KnowledgeArtifact, KnowledgeReceiver};
pub use saver::{KnowledgeRecord, KnowledgeStore};
pub use security::{
    ContentInspector, KnowledgeGuard, RiskComputation, RiskProfile, SecurityPolicy,
};
pub use seeker::{KnowledgeQuery, KnowledgeSeeker, KnowledgeSnippet};
pub use telemetry::{KnowledgeTelemetry, KnowledgeTelemetryBuilder};
pub use websearcher::{SearchChannel, WebSearchClient, WebSearcher};
