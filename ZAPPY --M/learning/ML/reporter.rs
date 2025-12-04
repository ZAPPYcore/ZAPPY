use serde::{Deserialize, Serialize};

/// Report describing classical ML training results.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingReport {
    /// Model name.
    pub model: String,
    /// Training mean squared error.
    pub mse: f32,
    /// Number of epochs.
    pub epochs: usize,
}

impl TrainingReport {
    /// Renders a concise summary string.
    #[must_use]
    pub fn summary(&self) -> String {
        format!(
            "[ML] model={} mse={:.4} epochs={}",
            self.model, self.mse, self.epochs
        )
    }
}
