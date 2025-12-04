use indexmap::IndexMap;

use super::helper::composite_metric;

/// Computes a projected ROI from priority, metrics delta, and duration.
#[must_use]
pub fn projected_roi(priority: u8, metrics: &IndexMap<String, f32>, duration_weeks: u16) -> f32 {
    let base = priority as f32 / 100.0;
    let metric_factor = composite_metric(metrics);
    let duration_factor = 1.0 - (duration_weeks as f32 / 52.0).min(0.6);
    (base * 0.5 + metric_factor * 0.4 + duration_factor * 0.1).clamp(0.0, 1.2)
}

/// Calculates normalized risk from phase count and priority.
#[must_use]
pub fn risk_from_complexity(phases: usize, priority: u8) -> f32 {
    let phase_factor = (phases as f32 * 0.05).min(0.5);
    let priority_factor = (priority as f32 / 100.0) * 0.3;
    (phase_factor + priority_factor).clamp(0.0, 1.0)
}

/// Estimates confidence score combining ROI and risk.
#[must_use]
pub fn confidence_score(roi: f32, risk: f32) -> f32 {
    (roi * 0.7 + (1.0 - risk) * 0.3).clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use indexmap::indexmap;

    #[test]
    fn projected_roi_combines_inputs() {
        let metrics = indexmap! { "reliability".into() => 0.8 };
        let roi = projected_roi(90, &metrics, 26);
        assert!(roi > 0.5);
    }

    #[test]
    fn confidence_penalizes_risk() {
        let confidence = confidence_score(0.8, 0.5);
        assert!(confidence < 0.8);
    }
}
