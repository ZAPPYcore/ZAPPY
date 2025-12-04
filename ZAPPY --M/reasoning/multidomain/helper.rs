use serde_json::json;

use crate::module::ReasoningHypothesis;

/// Computes aggregate score across domain outcomes.
#[must_use]
pub fn aggregate_confidence(hypothesis: &ReasoningHypothesis, domain_scores: &[f32]) -> f32 {
    if domain_scores.is_empty() {
        return hypothesis.confidence;
    }
    let avg = domain_scores.iter().copied().sum::<f32>() / domain_scores.len() as f32;
    ((hypothesis.confidence * 0.6) + avg * 0.4).clamp(0.0, 1.0)
}

/// Builds metadata payload for telemetry.
#[must_use]
pub fn telemetry_payload(hypothesis: &ReasoningHypothesis, aggregate: f32) -> serde_json::Value {
    json!({
        "hypothesis_id": hypothesis.id,
        "summary": hypothesis.summary,
        "base_confidence": hypothesis.confidence,
        "aggregate_confidence": aggregate,
        "supporting_signals": hypothesis.supporting_signals,
    })
}
