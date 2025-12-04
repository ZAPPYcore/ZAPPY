#![deny(
    clippy::all,
    clippy::pedantic,
    clippy::nursery,
    missing_docs,
    rust_2018_idioms
)]

//! Zappy Tier-9 learning stack: classical ML, deep learning, model combination, and subsidiary loops.

/// Module registry for learning subsystems.
#[path = "../modules.rs"]
pub mod modules;

/// Classical ML pipeline.
#[path = "../ML/main.rs"]
pub mod classical_ml;

/// Deep learning pipeline.
#[path = "../deepLearning/main.rs"]
pub mod deep_learning;

/// Model combining orchestration.
#[path = "../combining/main.rs"]
pub mod combining;

/// Telemetry helpers for logging/event emission.
#[path = "../telemetry.rs"]
pub mod telemetry;

/// Device discovery and allocation utilities.
#[path = "../device_manager.rs"]
pub mod device_manager;

/// Dataset indexing and shard loading helpers.
#[path = "../dataloader.rs"]
pub mod dataloader;

/// Subsidiary learning orchestration.
#[path = "../subsidarylearning/main.rs"]
pub mod subsidiary;

/// Shared learning pipeline protocol utilities.
#[path = "../pipeline.rs"]
pub mod pipeline;

/// Experience replay service for downstream modules.
#[path = "../replay.rs"]
pub mod replay;

/// High-level orchestration entry point.
#[path = "../main.rs"]
pub mod orchestration_entry;

pub use classical_ml::{editor::Dataset as ClassicalDataset, ClassicalMlPipeline};
pub use combining::{CombinationEngine, CombinationResult, CombinationReviewer};
pub use dataloader::{DatasetIndex, ShardBatch, ShardLoader};
pub use deep_learning::DeepLearningPipeline;
pub use device_manager::{AllocationPlan, DeviceInfo, DeviceKind, DeviceManager, DevicePreference};
pub use modules::{LearningModuleDescriptor, LearningModuleRegistry};
pub use pipeline::{ExperienceArchive, ExperienceHub, ExperienceRecorder, PipelineEnvelope};
pub use replay::ExperienceReplayService;
pub use subsidiary::SubsidiaryLearningRuntime;
pub use telemetry::{LearningTelemetry, LearningTelemetryBuilder};
