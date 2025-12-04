use crate::classical_ml::editor::{DataPoint, Dataset};

/// Splits dataset into train/test partitions (80/20 by default).
#[must_use]
pub fn train_test_split(dataset: &Dataset, test_ratio: f32) -> (Dataset, Dataset) {
    let split =
        ((1.0 - test_ratio).clamp(0.1, 0.9) * dataset.samples.len() as f32).round() as usize;
    let mut train = Dataset::default();
    let mut test = Dataset::default();
    for (idx, sample) in dataset.samples.iter().enumerate() {
        if idx < split {
            train.samples.push(sample.clone());
        } else {
            test.samples.push(sample.clone());
        }
    }
    (train, test)
}

/// Computes mean squared error between predictions and labels.
#[must_use]
pub fn mean_squared_error(predictions: &[f32], labels: &[f32]) -> f32 {
    if predictions.is_empty() || predictions.len() != labels.len() {
        return 0.0;
    }
    predictions
        .iter()
        .zip(labels.iter())
        .map(|(pred, label)| (pred - label).powi(2))
        .sum::<f32>()
        / predictions.len() as f32
}

/// Converts dataset samples into matrix/vector for linear models.
pub fn to_matrix(dataset: &Dataset) -> (Vec<Vec<f32>>, Vec<f32>) {
    let mut features = Vec::with_capacity(dataset.samples.len());
    let mut labels = Vec::with_capacity(dataset.samples.len());
    for DataPoint { features: f, label } in &dataset.samples {
        features.push(f.clone());
        labels.push(*label);
    }
    (features, labels)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::classical_ml::editor::Dataset;

    #[test]
    fn mse_handles_inputs() {
        let mse = mean_squared_error(&[1.0, 2.0], &[1.0, 1.5]);
        assert!(mse > 0.0);
    }

    #[test]
    fn split_generates_partitions() {
        let dataset = Dataset::synthetic(10, 2);
        let (train, test) = train_test_split(&dataset, 0.2);
        assert_eq!(train.samples.len(), 8);
        assert_eq!(test.samples.len(), 2);
    }
}
