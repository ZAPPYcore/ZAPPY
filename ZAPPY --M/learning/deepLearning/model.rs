use std::{fs, path::Path};

use anyhow::Context;
use ndarray::{Array2, ArrayBase, OwnedRepr};
use rand::{rngs::SmallRng, Rng, SeedableRng};
use serde::{Deserialize, Serialize};

/// Metadata describing a deep learning model snapshot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelSnapshot {
    /// Snapshot identifier.
    pub id: uuid::Uuid,
    /// Training step.
    pub step: u64,
    /// Validation loss.
    pub val_loss: f32,
}

/// Simple dense model used for demonstration.
#[derive(Debug, Clone)]
pub struct DenseModel {
    weights: Array2<f32>,
}

impl DenseModel {
    /// Creates a new dense model with random weights.
    #[must_use]
    pub fn new(input_dim: usize, output_dim: usize) -> Self {
        let mut rng = SmallRng::from_entropy();
        let weights = Array2::from_shape_fn((input_dim, output_dim), |_| rng.gen_range(-0.1..0.1));
        Self { weights }
    }

    /// Loads a dense model from dataset weights file.
    pub fn from_dataset_file(path: impl AsRef<Path>) -> anyhow::Result<Self> {
        #[derive(Deserialize)]
        struct DenseWeights {
            input_dim: usize,
            output_dim: usize,
            weights: Vec<f32>,
        }

        let contents =
            fs::read_to_string(&path).with_context(|| format!("reading {:?}", path.as_ref()))?;
        let dense: DenseWeights =
            serde_json::from_str(&contents).context("parsing dense weights file")?;
        if dense.weights.len() != dense.input_dim * dense.output_dim {
            anyhow::bail!("dense weight length mismatch");
        }
        let weights =
            Array2::from_shape_vec((dense.input_dim, dense.output_dim), dense.weights.clone())
                .context("building weight matrix")?;
        Ok(Self { weights })
    }

    /// Executes a forward pass.
    #[must_use]
    pub fn forward(
        &self,
        input: &ArrayBase<OwnedRepr<f32>, ndarray::Dim<[usize; 2]>>,
    ) -> Array2<f32> {
        input.dot(&self.weights)
    }

    /// Returns the weight matrix shape.
    #[must_use]
    pub fn weight_shape(&self) -> (usize, usize) {
        self.weights.dim()
    }

    /// Applies a simple SGD update.
    pub fn sgd_step(&mut self, grad: &Array2<f32>, lr: f32) {
        self.weights = &self.weights - &(grad * lr);
    }

    /// Creates a snapshot for auditing.
    #[must_use]
    pub fn snapshot(&mut self, step: u64, loss: f32) -> ModelSnapshot {
        ModelSnapshot {
            id: uuid::Uuid::new_v4(),
            step,
            val_loss: loss,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn dense_model_forward_runs() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("dataset/dense_weights.json");
        let model = DenseModel::from_dataset_file(path).unwrap();
        let input = Array2::from_shape_vec((1, 4), vec![1.0, 2.0, 3.0, 4.0]).unwrap();
        let output = model.forward(&input);
        assert_eq!(output.shape(), &[1, 2]);
    }
}
