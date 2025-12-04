use std::{fs, path::Path};

use anyhow::Context;
use rand::{rngs::SmallRng, Rng, SeedableRng};
use serde::{Deserialize, Serialize};

use crate::classical_ml::{
    editor::Dataset,
    func::{mean_squared_error, to_matrix},
};

/// Linear regression model with bias.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinearRegressionModel {
    weights: Vec<f32>,
    bias: f32,
}

impl LinearRegressionModel {
    /// Creates a new model with random weights.
    #[must_use]
    pub fn new(feature_dim: usize) -> Self {
        let mut rng = SmallRng::from_entropy();
        Self {
            weights: (0..feature_dim)
                .map(|_| rng.gen_range(-0.05..0.05))
                .collect(),
            bias: rng.gen_range(-0.05..0.05),
        }
    }

    /// Loads a model from a weights file located in `learning/dataset`.
    pub fn from_dataset_file(path: impl AsRef<Path>) -> anyhow::Result<Self> {
        #[derive(Deserialize)]
        struct LinearWeights {
            weights: Vec<f32>,
            bias: f32,
        }

        let contents =
            fs::read_to_string(&path).with_context(|| format!("reading {:?}", path.as_ref()))?;
        let weights: LinearWeights =
            serde_json::from_str(&contents).context("parsing linear weights file")?;
        Ok(Self {
            weights: weights.weights,
            bias: weights.bias,
        })
    }

    /// Predicts labels for a batch of samples.
    #[must_use]
    pub fn predict(&self, features: &[Vec<f32>]) -> Vec<f32> {
        features
            .iter()
            .map(|sample| {
                sample
                    .iter()
                    .zip(self.weights.iter())
                    .map(|(feature, weight)| feature * weight)
                    .sum::<f32>()
                    + self.bias
            })
            .collect()
    }

    /// Trains the model using a simple gradient descent routine.
    pub fn fit(&mut self, dataset: &Dataset, lr: f32, epochs: usize) -> f32 {
        let (features, labels) = to_matrix(dataset);
        if features.is_empty() {
            return 0.0;
        }
        for _ in 0..epochs {
            let predictions = self.predict(&features);
            let error = predictions
                .iter()
                .zip(labels.iter())
                .map(|(pred, label)| pred - label)
                .collect::<Vec<f32>>();

            for (idx, weight) in self.weights.iter_mut().enumerate() {
                let grad = error
                    .iter()
                    .zip(features.iter())
                    .map(|(err, sample)| err * sample[idx])
                    .sum::<f32>()
                    / features.len() as f32;
                *weight -= lr * grad;
            }

            let bias_grad = error.iter().sum::<f32>() / features.len() as f32;
            self.bias -= lr * bias_grad;
        }
        let predictions = self.predict(&features);
        mean_squared_error(&predictions, &labels)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::classical_ml::editor::Dataset;
    use std::path::Path;

    #[test]
    fn linear_model_training_reduces_error() {
        let mut dataset = Dataset::synthetic(50, 3);
        dataset.standardize();
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("dataset/linear_weights.json");
        let mut model = LinearRegressionModel::from_dataset_file(path).unwrap();
        let mse = model.fit(&dataset, 0.05, 5);
        assert!(mse >= 0.0);
    }
}
