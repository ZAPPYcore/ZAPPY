use std::sync::Arc;

use async_trait::async_trait;
use serde_json::json;
use tokio::runtime::Builder;

use crate::create::{CreativeIdea, CreativePortfolio};

/// Finding returned by a reviewer.
#[derive(Debug, Clone)]
pub struct ReviewFinding {
    /// Reviewer name.
    pub reviewer: String,
    /// Score between 0 and 1.
    pub score: f32,
    /// Notes for audit logs.
    pub notes: String,
}

/// Trait implemented by all creative reviewers.
#[async_trait]
pub trait CreativeReviewer: Send + Sync {
    /// Human-readable identifier used in metadata.
    fn name(&self) -> &str;
    /// Produces a review finding for the given idea.
    async fn evaluate(&self, idea: &CreativeIdea) -> ReviewFinding;
}

struct OriginalityReviewer;

#[async_trait]
impl CreativeReviewer for OriginalityReviewer {
    fn name(&self) -> &str {
        "originality"
    }

    async fn evaluate(&self, idea: &CreativeIdea) -> ReviewFinding {
        let rare_tokens = ["zero-gravity", "guild", "lattice"];
        let hits = rare_tokens
            .iter()
            .filter(|token| idea.body.contains(*token))
            .count() as f32;
        ReviewFinding {
            reviewer: self.name().into(),
            score: (hits / rare_tokens.len() as f32).clamp(0.0, 1.0),
            notes: format!("rare tokens detected: {hits}"),
        }
    }
}

struct ImpactReviewer;

#[async_trait]
impl CreativeReviewer for ImpactReviewer {
    fn name(&self) -> &str {
        "impact"
    }

    async fn evaluate(&self, idea: &CreativeIdea) -> ReviewFinding {
        let length = idea.body.len() as f32;
        let score = (length / 800.0).min(1.0);
        ReviewFinding {
            reviewer: self.name().into(),
            score,
            notes: format!("length-derived score {score:.2}"),
        }
    }
}

/// Aggregates reviewers and applies averaged scores.
#[derive(Clone)]
pub struct CreativeReviewBoard {
    reviewers: Vec<Arc<dyn CreativeReviewer>>,
}

impl std::fmt::Debug for CreativeReviewBoard {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CreativeReviewBoard")
            .field("reviewers", &self.reviewers.len())
            .finish()
    }
}

impl Default for CreativeReviewBoard {
    fn default() -> Self {
        Self {
            reviewers: vec![Arc::new(OriginalityReviewer), Arc::new(ImpactReviewer)],
        }
    }
}

impl CreativeReviewBoard {
    /// Adds a reviewer.
    pub fn with_reviewer(mut self, reviewer: Arc<dyn CreativeReviewer>) -> Self {
        self.reviewers.push(reviewer);
        self
    }

    /// Evaluates ranked ideas, returns polished portfolio.
    pub fn evaluate(&self, ideas: Vec<CreativeIdea>) -> CreativePortfolio {
        Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("runtime")
            .block_on(self.evaluate_async(ideas))
    }

    async fn evaluate_async(&self, ideas: Vec<CreativeIdea>) -> CreativePortfolio {
        let mut portfolio = CreativePortfolio::default();
        for mut idea in ideas {
            let mut total = 0.0;
            for reviewer in &self.reviewers {
                let finding = reviewer.evaluate(&idea).await;
                total += finding.score;
                idea = idea.with_metadata(
                    format!("review:{}", finding.reviewer),
                    json!({
                        "score": finding.score,
                        "notes": finding.notes
                    }),
                );
            }
            let avg = total / self.reviewers.len() as f32;
            portfolio.push(idea.with_score(avg));
        }
        portfolio
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::create::CreativityDialect;

    #[test]
    fn review_board_scores_ideas() {
        let board = CreativeReviewBoard::default();
        let idea = CreativeIdea::new(
            "title",
            "zero-gravity story",
            CreativityDialect::Experimental,
        );
        let portfolio = board.evaluate(vec![idea]);
        assert_eq!(portfolio.len(), 1);
        assert!(portfolio.ranked()[0].score > 0.0);
    }
}
