use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use crate::comprehension::helper::{normalize, split_sentences};

/// Score assigned to a sentence.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SentenceScore {
    /// Raw sentence.
    pub sentence: String,
    /// Score between 0-1.
    pub score: f32,
}

/// Ranks sentences using a simple TF heuristic against the query tokens.
pub fn rank_sentences(context: &str, query: &str) -> Vec<SentenceScore> {
    let sentences = split_sentences(context);
    let query_vocab = to_vocab(&tokenize(query));
    let mut ranked = Vec::new();
    for sentence in sentences {
        let tokens = tokenize(&sentence);
        let mut match_count = 0f32;
        for token in &tokens {
            if query_vocab.contains(token) {
                match_count += 1.0;
            }
        }
        let score = if tokens.is_empty() {
            0.0
        } else {
            (match_count / tokens.len() as f32).clamp(0.0, 1.0)
        };
        ranked.push(SentenceScore { sentence, score });
    }
    ranked.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
    ranked
}

fn tokenize(text: &str) -> Vec<String> {
    let norm = normalize(text);
    norm.split(' ')
        .filter(|token| !token.is_empty())
        .map(|token| token.to_string())
        .collect()
}

fn to_vocab(tokens: &[String]) -> HashSet<String> {
    tokens.iter().cloned().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ranking_prioritizes_matching_sentences() {
        let ranked = rank_sentences(
            "Rust is fast. Rust has a borrow checker. Python is dynamic.",
            "borrow checker",
        );
        assert_eq!(
            ranked.first().unwrap().sentence,
            "Rust has a borrow checker."
        );
    }
}
