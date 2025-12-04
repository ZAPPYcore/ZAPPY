use serde::{Deserialize, Serialize};

/// Reviewer that validates ensemble predictions.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct CombinationReviewer;

impl CombinationReviewer {
    /// Ensures predictions are finite and not all zero.
    pub fn review(&self, predictions: &[f32]) -> anyhow::Result<()> {
        if predictions.iter().any(|value| !value.is_finite()) {
            anyhow::bail!("combination produced non-finite values");
        }
        let norm: f32 = predictions.iter().map(|value| value.abs()).sum();
        if norm.abs() < f32::EPSILON {
            anyhow::bail!("combination produced zero vector");
        }
        Ok(())
    }
}
