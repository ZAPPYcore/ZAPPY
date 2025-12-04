use ndarray::Array2;
use serde::{Deserialize, Serialize};

use super::model::{DenseModel, ModelSnapshot};

/// Training hyperparameters.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingConfig {
    /// Learning rate.
    pub learning_rate: f32,
    /// Number of steps.
    pub steps: u64,
}

impl Default for TrainingConfig {
    fn default() -> Self {
        Self {
            learning_rate: 0.01,
            steps: 100,
        }
    }
}

/// Trainer responsible for running optimization.
#[derive(Debug)]
pub struct Trainer {
    config: TrainingConfig,
}

impl Trainer {
    /// Creates a new trainer.
    #[must_use]
    pub fn new(config: TrainingConfig) -> Self {
        Self { config }
    }

    /// Runs training with dummy gradients and returns snapshots.
    pub fn train(&self, model: &mut DenseModel) -> Vec<ModelSnapshot> {
        let mut snapshots = Vec::new();
        let shape = model.weight_shape();
        let grad = Array2::from_elem(shape, 0.05);
        for step in 0..self.config.steps {
            // Fake gradient descent step
            model.sgd_step(&grad, self.config.learning_rate);
            let loss = 1.0 / (step as f32 + 1.0);
            snapshots.push(model.snapshot(step, loss));
        }
        snapshots
    }
}
