use serde::{Deserialize, Serialize};

/// Mini-batch used for deep learning training.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Batch {
    /// Input tensor represented as 2D matrix.
    pub inputs: Vec<Vec<f32>>,
    /// Target tensor.
    pub targets: Vec<Vec<f32>>,
}

/// Utility builder for batches.
#[derive(Debug, Default)]
pub struct BatchBuilder;

impl BatchBuilder {
    /// Creates batches of the provided size from raw tensors.
    #[must_use]
    pub fn build(
        &self,
        inputs: &[Vec<f32>],
        targets: &[Vec<f32>],
        batch_size: usize,
    ) -> Vec<Batch> {
        let mut batches = Vec::new();
        let mut start = 0;
        while start < inputs.len() {
            let end = (start + batch_size).min(inputs.len());
            batches.push(Batch {
                inputs: inputs[start..end].to_vec(),
                targets: targets[start..end].to_vec(),
            });
            start = end;
        }
        batches
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builder_creates_batches() {
        let builder = BatchBuilder::default();
        let inputs = vec![vec![0.1]; 5];
        let targets = vec![vec![0.2]; 5];
        let batches = builder.build(&inputs, &targets, 2);
        assert_eq!(batches.len(), 3);
    }
}
