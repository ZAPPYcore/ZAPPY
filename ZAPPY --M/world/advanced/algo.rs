use indexmap::IndexMap;

/// Applies exponential moving average smoothing.
#[must_use]
pub fn ewma(series: &[f32], alpha: f32) -> Vec<f32> {
    if series.is_empty() {
        return Vec::new();
    }
    let mut result = Vec::with_capacity(series.len());
    let mut prev = series[0];
    result.push(prev);
    for &value in &series[1..] {
        prev = alpha * value + (1.0 - alpha) * prev;
        result.push(prev);
    }
    result
}

/// Computes anomaly score relative to baseline metrics.
#[must_use]
pub fn anomaly_score(metrics: &IndexMap<String, f32>, baseline: &IndexMap<String, f32>) -> f32 {
    let mut score = 0.0;
    for (key, value) in metrics {
        let base = baseline.get(key).copied().unwrap_or(*value);
        score += (value - base).abs();
    }
    (score / metrics.len().max(1) as f32).clamp(0.0, 1.5)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ewma_smooths_series() {
        let data = vec![1.0, 2.0, 3.0];
        let smoothed = ewma(&data, 0.2);
        assert_eq!(smoothed.len(), 3);
        assert!(smoothed[1] < data[1]);
    }
}
