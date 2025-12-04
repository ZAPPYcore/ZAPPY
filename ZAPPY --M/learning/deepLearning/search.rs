use rand::{rngs::SmallRng, Rng, SeedableRng};
use serde::{Deserialize, Serialize};

/// Candidate architecture configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchitectureCandidate {
    /// Number of layers.
    pub layers: usize,
    /// Hidden dimension.
    pub hidden_dim: usize,
    /// Dropout rate.
    pub dropout: f32,
    /// Score assigned by the search heuristic.
    pub score: f32,
}

/// Performs lightweight random search.
#[derive(Debug, Clone)]
pub struct ArchitectureSearch {
    rng: SmallRng,
}

impl Default for ArchitectureSearch {
    fn default() -> Self {
        Self {
            rng: SmallRng::from_entropy(),
        }
    }
}

impl ArchitectureSearch {
    /// Generates a set of candidates.
    #[must_use]
    pub fn run(&mut self, iterations: usize) -> Vec<ArchitectureCandidate> {
        let mut candidates = Vec::with_capacity(iterations);
        for _ in 0..iterations {
            let layers = self.rng.gen_range(2..8);
            let hidden_dim = self.rng.gen_range(64..512);
            let dropout = self.rng.gen_range(0.0..0.5);
            let score = self.evaluate(layers, hidden_dim, dropout);
            candidates.push(ArchitectureCandidate {
                layers,
                hidden_dim,
                dropout,
                score,
            });
        }
        candidates.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
        candidates
    }

    fn evaluate(&self, layers: usize, hidden_dim: usize, dropout: f32) -> f32 {
        // Heuristic: favor moderate layer counts and hidden dims.
        let layer_term = (layers as f32 - 5.0).abs();
        let dim_term = (hidden_dim as f32 - 256.0).abs() / 256.0;
        1.0 / (1.0 + layer_term + dim_term + dropout)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn search_produces_candidates() {
        let mut search = ArchitectureSearch::default();
        let candidates = search.run(5);
        assert_eq!(candidates.len(), 5);
    }
}
