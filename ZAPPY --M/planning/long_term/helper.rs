use indexmap::IndexMap;

/// Normalizes resource allocations so that the sum is at most 1.0.
pub fn normalize_resources(resources: &mut IndexMap<String, f32>) {
    let total: f32 = resources.values().copied().sum();
    if total <= 1.0 || total == 0.0 {
        return;
    }
    for value in resources.values_mut() {
        *value /= total;
    }
}

/// Computes a composite metric score using equal weights.
#[must_use]
pub fn composite_metric(metrics: &IndexMap<String, f32>) -> f32 {
    if metrics.is_empty() {
        return 0.0;
    }
    metrics.values().copied().sum::<f32>() / metrics.len() as f32
}

/// Clamps a score between 0 and 1.
#[must_use]
pub fn clamp_score(value: f32) -> f32 {
    value.clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use indexmap::indexmap;

    #[test]
    fn resources_are_normalized() {
        let mut resources = indexmap! {
            "eng".into() => 0.8,
            "ops".into() => 0.6,
        };
        normalize_resources(&mut resources);
        let sum: f32 = resources.values().copied().sum();
        assert!((sum - 1.0).abs() < 1e-3);
    }

    #[test]
    fn composite_metric_is_average() {
        let metrics = indexmap! {
            "a".into() => 0.5,
            "b".into() => 0.7,
        };
        assert!((composite_metric(&metrics) - 0.6).abs() < 1e-6);
    }
}
