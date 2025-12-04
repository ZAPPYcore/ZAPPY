use std::collections::HashMap;

use rand::{rngs::SmallRng, Rng, SeedableRng};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::module::{ReasoningDirective, ReasoningHypothesis, SignalPacket};

/// Graph connecting signals and derived hypotheses.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct SignalGraph {
    /// Nodes keyed by signal id.
    pub signals: HashMap<Uuid, SignalNode>,
    /// Hypotheses keyed by id.
    pub hypotheses: HashMap<Uuid, ReasoningHypothesis>,
}

impl SignalGraph {
    /// Adds a signal node.
    pub fn add_signal(&mut self, packet: SignalPacket) {
        self.signals.insert(
            packet.id,
            SignalNode {
                packet,
                weight: 0.0,
            },
        );
    }

    /// Records a hypothesis.
    pub fn add_hypothesis(&mut self, hypothesis: ReasoningHypothesis) {
        self.hypotheses.insert(hypothesis.id, hypothesis);
    }
}

/// Internal signal representation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignalNode {
    /// Source packet.
    pub packet: SignalPacket,
    /// Weight assigned during inference.
    pub weight: f32,
}

/// Result after running inference.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InferenceResult {
    /// Directive that was processed.
    pub directive: ReasoningDirective,
    /// Generated hypotheses.
    pub hypotheses: Vec<ReasoningHypothesis>,
    /// Graph state.
    pub graph: SignalGraph,
}

/// Core inference engine.
#[derive(Debug, Clone)]
pub struct InferenceEngine {
    rng: SmallRng,
    acceptance: f32,
}

impl InferenceEngine {
    /// Creates a default engine.
    #[must_use]
    pub fn new(acceptance: f32) -> Self {
        Self {
            rng: SmallRng::from_entropy(),
            acceptance,
        }
    }

    /// Runs inference from signals and directive.
    pub fn infer(
        &mut self,
        directive: ReasoningDirective,
        signals: Vec<SignalPacket>,
    ) -> InferenceResult {
        let mut graph = SignalGraph::default();
        for packet in signals.clone() {
            graph.add_signal(packet);
        }
        let mut hypotheses = Vec::new();
        for chunk in signals.chunks(2) {
            let summary = format!("{} -> {} related signals", directive.prompt, chunk.len());
            let confidence = self.sample_confidence(directive.priority.score(), chunk.len());
            let hypothesis = ReasoningHypothesis {
                id: Uuid::new_v4(),
                summary,
                confidence,
                supporting_signals: chunk.iter().map(|s| s.id).collect(),
            };
            graph.add_hypothesis(hypothesis.clone());
            hypotheses.push(hypothesis);
        }
        InferenceResult {
            directive,
            hypotheses,
            graph,
        }
    }

    fn sample_confidence(&mut self, priority_score: u8, signal_count: usize) -> f32 {
        let base = (priority_score as f32 / 100.0) * 0.6;
        let signal_bonus = (signal_count as f32 * 0.08).min(0.3);
        let acceptance_bonus = self.acceptance * 0.2;
        (base + signal_bonus + acceptance_bonus + self.rng.gen::<f32>() * 0.2).clamp(0.0, 0.99)
    }
}

impl Default for InferenceEngine {
    fn default() -> Self {
        Self::new(0.55)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::module::{DirectivePriority, ReasoningDirective};
    use serde_json::json;

    #[test]
    fn inference_generates_hypotheses() {
        let directive = ReasoningDirective::new("Assess anomaly", DirectivePriority::Medium);
        let mut engine = InferenceEngine::default();
        let signals = vec![
            SignalPacket::new("sensor spike", json!({ "value": 12 })),
            SignalPacket::new("latency jump", json!({ "ms": 300 })),
        ];
        let result = engine.infer(directive, signals);
        assert!(!result.hypotheses.is_empty());
    }
}
