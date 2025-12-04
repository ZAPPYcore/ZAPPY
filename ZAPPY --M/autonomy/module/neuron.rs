use indexmap::IndexMap;

/// Snapshot of a neuron's activation state.
#[derive(Debug, Clone)]
pub struct NeuronPulse {
    /// Unique neuron identifier.
    pub name: String,
    /// Activation score between 0 and 1.
    pub activation: f32,
    /// Optional commentary explaining the score.
    pub commentary: String,
}

#[derive(Debug, Clone)]
struct Neuron {
    name: String,
    weight: f32,
    bias: f32,
}

impl Neuron {
    fn activate(&self, input: f32) -> f32 {
        let raw = (input * self.weight) + self.bias;
        1.0 / (1.0 + (-raw).exp())
    }
}

/// Lightweight neural ensemble that scores module health.
#[derive(Debug, Clone)]
pub struct NeuronGraph {
    neurons: Vec<Neuron>,
}

impl Default for NeuronGraph {
    fn default() -> Self {
        Self {
            neurons: vec![
                Neuron {
                    name: "stability".into(),
                    weight: 1.4,
                    bias: 0.2,
                },
                Neuron {
                    name: "safety".into(),
                    weight: 1.1,
                    bias: -0.1,
                },
            ],
        }
    }
}

impl NeuronGraph {
    /// Scores the provided metrics producing pulses.
    #[must_use]
    pub fn pulse(&self, metrics: &IndexMap<String, f64>) -> Vec<NeuronPulse> {
        self.neurons
            .iter()
            .map(|neuron| {
                let input = metrics.get(&neuron.name).copied().unwrap_or(0.5) as f32;
                let activation = neuron.activate(input);
                NeuronPulse {
                    name: neuron.name.clone(),
                    activation,
                    commentary: format!("{}= {:.2}", neuron.name, activation),
                }
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn produces_pulses() {
        let graph = NeuronGraph::default();
        let mut metrics = IndexMap::new();
        metrics.insert("stability".into(), 0.7);
        let pulses = graph.pulse(&metrics);
        assert_eq!(pulses.len(), 2);
        assert!(pulses.iter().any(|p| p.name == "stability"));
    }
}
