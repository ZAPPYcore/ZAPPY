use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::create::CreativeIdea;

/// Metric describing the divergence of a portfolio.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DivergenceMetric {
    /// Score between 0 and 1.
    pub score: f32,
    /// Breakdown per token.
    pub distribution: IndexMap<String, f32>,
}

/// Calculates divergence by analyzing body vocab.
#[derive(Debug, Default, Clone)]
pub struct DivergencePlanner;

impl DivergencePlanner {
    /// Scores the divergence.
    #[must_use]
    pub fn score(&self, ideas: &[CreativeIdea]) -> DivergenceMetric {
        let mut freq: IndexMap<String, f32> = IndexMap::new();
        for idea in ideas {
            for token in idea.body.split_whitespace() {
                let token = token.to_lowercase();
                *freq.entry(token).or_insert(0.0) += 1.0;
            }
        }
        let unique = freq.len() as f32;
        let total: f32 = freq.values().sum();
        let score = if total == 0.0 { 0.0 } else { unique / total };
        DivergenceMetric {
            score: score.min(1.0),
            distribution: freq,
        }
    }
}

/// Suggests convergence actions to synthesize ideas.
#[derive(Debug, Clone)]
pub struct ConvergencePlanner;

impl ConvergencePlanner {
    /// Generates a convergence statement.
    #[must_use]
    pub fn synthesize(&self, ideas: &[CreativeIdea]) -> String {
        if ideas.is_empty() {
            return "No ideas to converge.".into();
        }
        let mut statement = String::from("Converge by blending: ");
        for (idx, idea) in ideas.iter().enumerate().take(3) {
            if idx > 0 {
                statement.push_str(" + ");
            }
            statement.push_str(&idea.title);
        }
        statement
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::create::{CreativeIdea, CreativityDialect};

    #[test]
    fn divergence_scores_tokens() {
        let ideas = vec![CreativeIdea::new(
            "a",
            "one two three",
            CreativityDialect::Analytical,
        )];
        let planner = DivergencePlanner::default();
        let metric = planner.score(&ideas);
        assert!(metric.score > 0.0);
    }

    #[test]
    fn convergence_synthesizes_titles() {
        let ideas = vec![CreativeIdea::new(
            "idea-1",
            "body",
            CreativityDialect::Poetic,
        )];
        let planner = ConvergencePlanner;
        let result = planner.synthesize(&ideas);
        assert!(result.contains("idea-1"));
    }
}
