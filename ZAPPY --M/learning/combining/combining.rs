use serde::{Deserialize, Serialize};

use super::reviewer::CombinationReviewer;
use crate::classical_ml::submodel::SubModelManager;

/// Result produced by the combination engine.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CombinationResult {
    /// Combined predictions.
    pub predictions: Vec<f32>,
    /// Reviewer notes.
    pub notes: String,
}

/// Engine that blends multiple submodels and validates the output.
#[derive(Debug)]
pub struct CombinationEngine {
    reviewer: CombinationReviewer,
}

impl CombinationEngine {
    /// Creates a new engine.
    #[must_use]
    pub fn new(reviewer: CombinationReviewer) -> Self {
        Self { reviewer }
    }

    /// Runs combination across the manager and returns validated predictions.
    pub fn combine(
        &self,
        manager: &SubModelManager,
        features: &[Vec<f32>],
    ) -> anyhow::Result<CombinationResult> {
        let predictions = manager.blend(features);
        self.reviewer.review(&predictions)?;
        Ok(CombinationResult {
            predictions,
            notes: "ensemble validated".into(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::super::reviewer::CombinationReviewer;
    use super::*;
    use crate::classical_ml::{
        ml::LinearRegressionModel,
        submodel::{SubModel, SubModelManager},
    };

    #[test]
    fn engine_combines_models() {
        let mut manager = SubModelManager::default();
        manager.add(SubModel::new(LinearRegressionModel::new(2), 1.0));
        let engine = CombinationEngine::new(CombinationReviewer::default());
        let result = engine.combine(&manager, &[vec![0.0, 0.0]]).unwrap();
        assert_eq!(result.predictions.len(), 1);
    }
}
