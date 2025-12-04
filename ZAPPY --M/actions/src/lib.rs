#![deny(
    clippy::all,
    clippy::pedantic,
    clippy::nursery,
    missing_docs,
    rust_2018_idioms
)]

//! Zappy Tier-9 AGI action orchestration library.

/// Core primitives representing requests, plans, and outcomes.
#[path = "../actions.rs"]
pub mod actions;

/// Domain-aware execution agents and registry.
#[path = "../agents.rs"]
pub mod agents;

/// High-level orchestrator responsible for end-to-end execution.
#[path = "../actioncommander.rs"]
pub mod actioncommander;

/// Ergonomic helper functions for bootstrapping the action fabric.
#[path = "../allfunctions.rs"]
pub mod allfunctions;

/// Command and plan generation pipeline.
#[path = "../commandgeneration.rs"]
pub mod commandgeneration;

/// Internet-facing execution connectors.
#[path = "../internetact.rs"]
pub mod internetact;

/// Offline/sandboxed execution helpers.
#[path = "../offlineact.rs"]
pub mod offlineact;

/// Programming-aware executors and patch builders.
#[path = "../programming.rs"]
pub mod programming;

/// Utilities for generating diffs and code review artifacts.
#[path = "../programminghelper.rs"]
pub mod programminghelper;

/// Security enforcement facade bridging policy and heuristics.
#[path = "../security_link.rs"]
pub mod security_link;

/// Self-training execution utilities and interfaces.
#[path = "../self_trainact.rs"]
pub mod self_trainact;

/// Sample orchestration entrypoints.
#[path = "../main.rs"]
pub mod orchestration_entry;

/// Advanced analytics and scenario tooling.
#[path = "../advanced/main.rs"]
pub mod advanced;

/// Security orchestration and policy modules.
#[path = "../secirity/main.rs"]
pub mod security;

/// Telemetry helpers for action orchestration.
#[path = "../telemetry.rs"]
pub mod telemetry;

/// Prelude exports for consumers that interact with the action fabric.
pub mod prelude {
    pub use crate::actioncommander::{ActionCommander, ActionCommanderBuilder};
    pub use crate::actions::{
        ActionDomain, ActionId, ActionIntent, ActionPayload, ActionPriority, ActionRequest,
        ActionStatus,
    };
    pub use crate::agents::{ActionAgent, AgentRegistry};
    pub use crate::commandgeneration::{CommandGenerator, HeuristicCommandGenerator};
    pub use crate::security_link::{SecurityLink, SecurityLinkBuilder};
    pub use crate::telemetry::{ActionTelemetry, ActionTelemetryBuilder};
}
