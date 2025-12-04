use rand::SeedableRng;
use rand::{rngs::SmallRng, Rng};
use serde::{Deserialize, Serialize};

/// Single training data point.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataPoint {
    /// Feature vector.
    pub features: Vec<f32>,
    /// Target label.
    pub label: f32,
}

/// Dataset wrapper.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Dataset {
    /// Data points.
    pub samples: Vec<DataPoint>,
}

impl Dataset {
    /// Generates a synthetic dataset for testing and demos.
    #[must_use]
    pub fn synthetic(count: usize, feature_dim: usize) -> Self {
        let mut rng = SmallRng::from_entropy();
        let mut samples = Vec::with_capacity(count);
        for _ in 0..count {
            let mut features = Vec::with_capacity(feature_dim);
            for _ in 0..feature_dim {
                features.push(rng.gen_range(-1.0..1.0));
            }
            let label = features.iter().sum::<f32>() + rng.gen_range(-0.1..0.1);
            samples.push(DataPoint { features, label });
        }
        Self { samples }
    }

    /// Applies feature standardization.
    pub fn standardize(&mut self) {
        if self.samples.is_empty() {
            return;
        }
        let feature_dim = self.samples[0].features.len();
        let mut means = vec![0.0; feature_dim];
        for point in &self.samples {
            for (idx, value) in point.features.iter().enumerate() {
                means[idx] += value;
            }
        }
        for mean in &mut means {
            *mean /= self.samples.len() as f32;
        }

        let mut variances = vec![0.0; feature_dim];
        for point in &self.samples {
            for (idx, value) in point.features.iter().enumerate() {
                variances[idx] += (value - means[idx]).powi(2);
            }
        }
        for variance in &mut variances {
            *variance = (*variance / self.samples.len() as f32).sqrt().max(1e-6);
        }

        for point in &mut self.samples {
            for (idx, value) in point.features.iter_mut().enumerate() {
                *value = (*value - means[idx]) / variances[idx];
            }
        }
    }

    /// Returns the feature dimensionality.
    #[must_use]
    pub fn feature_dim(&self) -> usize {
        self.samples.first().map_or(0, |point| point.features.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dataset_standardizes_features() {
        let mut dataset = Dataset::synthetic(8, 3);
        dataset.standardize();
        assert_eq!(dataset.feature_dim(), 3);
    }
}
