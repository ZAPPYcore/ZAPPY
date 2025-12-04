#![deny(
    clippy::all,
    clippy::pedantic,
    clippy::nursery,
    rust_2018_idioms,
    missing_docs
)]

//! Zappy World module – maintains environment state, seekers, and learning.

/// Telemetry builder for world runtime.
#[path = "../telemetry.rs"]
pub mod telemetry;

/// Core world model definitions.
#[path = "../model.rs"]
pub mod model;

/// Learning pipeline that assimilates new observations.
#[path = "../learning.rs"]
pub mod learning;

/// Info seeker that ingests signals from external sources.
#[path = "../infoseeker.rs"]
pub mod infoseeker;

/// External feed configuration helpers.
#[path = "../feed_config.rs"]
pub mod feed_config;

/// Feature store for persisting assimilation inputs.
#[path = "../feature_store.rs"]
pub mod feature_store;

/// Advanced world algorithms.
#[path = "../advanced/main.rs"]
pub mod advanced;

/// Runtime entry point orchestrating state updates.
#[path = "../main.rs"]
pub mod runtime;

pub use feature_store::FeatureStore;
pub use feed_config::{FeedConfig, FeedKind, FeedsDocument};
pub use infoseeker::{InfoSeeker, InfoSeekerBuilder, InfoSignal};
pub use learning::{AssimilationEngine, AssimilationJob};
pub use model::{WorldModel, WorldState};
pub use runtime::{WorldRuntime, WorldRuntimeBuilder};
pub use telemetry::{WorldTelemetry, WorldTelemetryBuilder};
