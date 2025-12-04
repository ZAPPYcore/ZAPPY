use indexmap::IndexMap;

/// Maintains a moving average over the most recent samples.
#[derive(Debug, Clone)]
pub struct SignalSmoother {
    capacity: usize,
    samples: Vec<f64>,
}

impl SignalSmoother {
    /// Creates a new smoother with the provided capacity.
    #[must_use]
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity: capacity.max(1),
            samples: Vec::new(),
        }
    }

    /// Adds a new sample and returns the updated mean.
    #[must_use]
    pub fn push(&mut self, value: f64) -> f64 {
        if self.samples.len() == self.capacity {
            self.samples.remove(0);
        }
        self.samples.push(value);
        self.mean()
    }

    /// Returns the current mean, or zero when no samples are present.
    #[must_use]
    pub fn mean(&self) -> f64 {
        if self.samples.is_empty() {
            return 0.0;
        }
        #[allow(clippy::cast_precision_loss)]
        let denominator = self.samples.len() as f64;
        self.samples.iter().sum::<f64>() / denominator
    }
}

impl Default for SignalSmoother {
    fn default() -> Self {
        Self::new(5)
    }
}

/// Normalizes the provided metric map so values add up to one.
#[must_use]
pub fn normalize_scores(metrics: &IndexMap<String, f64>) -> IndexMap<String, f64> {
    let sum: f64 = metrics.values().sum();
    if sum == 0.0 {
        return metrics.clone();
    }

    metrics.iter().map(|(k, v)| (k.clone(), v / sum)).collect()
}

/// Calculates an exponentially weighted moving average.
#[must_use]
pub fn ewma(previous: f64, next: f64, alpha: f64) -> f64 {
    let alpha = alpha.clamp(0.0, 1.0);
    (alpha * next) + ((1.0 - alpha) * previous)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn smoother_limits_samples() {
        let mut smoother = SignalSmoother::new(2);
        let _ = smoother.push(1.0);
        let _ = smoother.push(3.0);
        let _ = smoother.push(5.0);
        assert_eq!(smoother.mean(), 4.0);
    }

    #[test]
    fn normalize_preserves_sum() {
        let mut metrics = IndexMap::new();
        metrics.insert("a".into(), 2.0);
        metrics.insert("b".into(), 2.0);
        let normalized = normalize_scores(&metrics);
        assert!((normalized.get("a").unwrap() - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn ewma_behaves() {
        let result = ewma(0.0, 10.0, 0.5);
        assert!((result - 5.0).abs() < f64::EPSILON);
    }
}
