//! Advanced forecasting + governance utilities for the world module.

/// High-level controller wiring predictive models and reviewers.
pub mod advanced;
/// Predictive model implementations.
pub mod advmodel;
/// Shared algorithms such as EWMA and anomaly scoring.
pub mod algo;
/// Governance reviewer for world states.
pub mod reviewer;
/// Offline training orchestration.
pub mod train;

pub use advanced::AdvancedController;
pub use advmodel::PredictiveModel;
pub use reviewer::StateReviewer;
pub use train::{Trainer, TrainingArtifact, TrainingConfig};
