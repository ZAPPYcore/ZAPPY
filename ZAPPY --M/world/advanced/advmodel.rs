use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::algo::{anomaly_score, ewma};

/// Predictive model capturing rolling metrics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PredictiveModel {
    /// Model id.
    pub model_id: Uuid,
    /// Baseline metrics.
    pub baseline: IndexMap<String, f32>,
    /// History window.
    pub history: Vec<f32>,
}

impl PredictiveModel {
    /// Creates a new model.
    #[must_use]
    pub fn new(baseline: IndexMap<String, f32>) -> Self {
        Self {
            model_id: Uuid::new_v4(),
            baseline,
            history: Vec::new(),
        }
    }

    /// Updates the model with new measurements and returns risk score.
    pub fn update(&mut self, metrics: &IndexMap<String, f32>) -> f32 {
        let score = anomaly_score(metrics, &self.baseline);
        self.history.push(score);
        if self.history.len() > 64 {
            self.history.remove(0);
        }
        score
    }

    /// Forecasts near-term risk using EWMA.
    #[must_use]
    pub fn forecast(&self) -> f32 {
        let smoothed = ewma(&self.history, 0.3);
        smoothed.last().copied().unwrap_or(0.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn predictive_model_forecasts() {
        let mut baseline = IndexMap::new();
        baseline.insert("load".into(), 0.4);
        let mut model = PredictiveModel::new(baseline);
        let mut metrics = IndexMap::new();
        metrics.insert("load".into(), 0.7);
        model.update(&metrics);
        assert!(model.forecast() >= 0.0);
    }
}
