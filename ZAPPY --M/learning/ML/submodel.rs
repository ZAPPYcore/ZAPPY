use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::ml::LinearRegressionModel;

/// Represents a submodel participating in an ensemble.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubModel {
    /// Identifier.
    pub id: Uuid,
    /// Weight in the ensemble.
    pub weight: f32,
    /// Underlying linear model.
    pub model: LinearRegressionModel,
}

impl SubModel {
    /// Creates a new submodel with the provided linear model.
    #[must_use]
    pub fn new(model: LinearRegressionModel, weight: f32) -> Self {
        Self {
            id: Uuid::new_v4(),
            weight,
            model,
        }
    }
}

/// Manages a collection of submodels.
#[derive(Debug, Default, Clone)]
pub struct SubModelManager {
    /// Models participating in the ensemble.
    pub models: Vec<SubModel>,
}

impl SubModelManager {
    /// Adds a submodel to the manager.
    pub fn add(&mut self, submodel: SubModel) {
        self.models.push(submodel);
    }

    /// Blends predictions from all submodels using weights.
    #[must_use]
    pub fn blend(&self, features: &[Vec<f32>]) -> Vec<f32> {
        if self.models.is_empty() {
            return vec![0.0; features.len()];
        }
        let mut blended = vec![0.0; features.len()];
        let total_weight: f32 = self.models.iter().map(|model| model.weight).sum();
        for submodel in &self.models {
            let predictions = submodel.model.predict(features);
            for (idx, value) in predictions.iter().enumerate() {
                blended[idx] += (submodel.weight / total_weight.max(1e-6)) * value;
            }
        }
        blended
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::classical_ml::ml::LinearRegressionModel;

    #[test]
    fn manager_blends_predictions() {
        let model = LinearRegressionModel::new(2);
        let mut manager = SubModelManager::default();
        manager.add(SubModel::new(model, 1.0));
        let preds = manager.blend(&vec![vec![0.0, 0.0]]);
        assert_eq!(preds.len(), 1);
    }
}
