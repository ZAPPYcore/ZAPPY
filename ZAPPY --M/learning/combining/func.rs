use crate::classical_ml::submodel::SubModel;

/// Normalizes ensemble weights so they add up to one.
pub fn normalize_weights(submodels: &mut [SubModel]) {
    let total: f32 = submodels.iter().map(|model| model.weight).sum();
    let total = total.max(1e-6);
    for model in submodels {
        model.weight /= total;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::classical_ml::ml::LinearRegressionModel;

    #[test]
    fn normalize_weights_sets_unit_sum() {
        let mut models = vec![
            SubModel::new(LinearRegressionModel::new(2), 1.0),
            SubModel::new(LinearRegressionModel::new(2), 2.0),
        ];
        normalize_weights(&mut models);
        let sum: f32 = models.iter().map(|m| m.weight).sum();
        assert!((sum - 1.0).abs() < 1e-5);
    }
}
