use std::fmt;

use chrono::{DateTime, Utc};
use indexmap::IndexMap;
use rand::{rngs::SmallRng, seq::SliceRandom, Rng, SeedableRng};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

use crate::helpermethod::IdeaTransformer;

/// Unique identifier assigned to each generated idea.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct CreativeIdeaId(Uuid);

impl CreativeIdeaId {
    /// Creates a random identifier.
    #[must_use]
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl fmt::Display for CreativeIdeaId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Supported creative dialects.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum CreativityDialect {
    /// Highly expressive lyrical tone.
    Poetic,
    /// Data-backed analytical tone.
    Analytical,
    /// Lighthearted, humorous narrative.
    Playful,
    /// Deeply technical, detail-heavy output.
    Technical,
    /// Bold, exploratory, futuristic tone.
    Experimental,
}

impl CreativityDialect {
    /// Returns a short descriptor used by downstream scoring.
    #[must_use]
    pub fn descriptor(&self) -> &'static str {
        match self {
            Self::Poetic => "poetic",
            Self::Analytical => "analytical",
            Self::Playful => "playful",
            Self::Technical => "technical",
            Self::Experimental => "experimental",
        }
    }
}

/// Constraints governing how the idea should be shaped.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CreativeConstraint {
    /// Maximum token length.
    pub max_length: Option<usize>,
    /// Keywords that must appear.
    pub required_keywords: Vec<String>,
    /// Concepts that must be avoided.
    pub avoid: Vec<String>,
    /// Intended audience.
    pub audience: Option<String>,
}

impl CreativeConstraint {
    /// Returns a coarse measure of how many constraint elements are active.
    #[must_use]
    pub fn complexity(&self) -> usize {
        let mut count = self.required_keywords.len() + self.avoid.len();
        if self.max_length.is_some() {
            count += 1;
        }
        if self.audience.is_some() {
            count += 1;
        }
        count
    }
}

/// High-level brief for the creativity engine.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreativeBrief {
    /// Project or campaign title.
    pub title: String,
    /// Objective or goal.
    pub objective: String,
    /// Dialect to employ.
    pub dialect: CreativityDialect,
    /// Constraints.
    pub constraints: CreativeConstraint,
    /// Optional seed statements.
    pub seed_ideas: Vec<String>,
}

impl CreativeBrief {
    /// Convenience constructor.
    #[must_use]
    pub fn new(
        title: impl Into<String>,
        objective: impl Into<String>,
        dialect: CreativityDialect,
    ) -> Self {
        Self {
            title: title.into(),
            objective: objective.into(),
            dialect,
            constraints: CreativeConstraint::default(),
            seed_ideas: Vec::new(),
        }
    }

    /// Adds a constraint.
    #[must_use]
    pub fn with_constraint(mut self, constraint: CreativeConstraint) -> Self {
        self.constraints = constraint;
        self
    }

    /// Adds a seed idea.
    #[must_use]
    pub fn with_seed(mut self, seed: impl Into<String>) -> Self {
        self.seed_ideas.push(seed.into());
        self
    }
}

/// Single creative idea with metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreativeIdea {
    /// Unique identifier.
    pub id: CreativeIdeaId,
    /// Short title.
    pub title: String,
    /// Full body text.
    pub body: String,
    /// Dialect used.
    pub dialect: CreativityDialect,
    /// Heuristic quality score (0-1).
    pub score: f32,
    /// Additional metadata for analytics.
    pub metadata: IndexMap<String, serde_json::Value>,
    /// Creation timestamp.
    pub created_at: DateTime<Utc>,
}

impl CreativeIdea {
    /// Creates a new idea.
    #[must_use]
    pub fn new(
        title: impl Into<String>,
        body: impl Into<String>,
        dialect: CreativityDialect,
    ) -> Self {
        Self {
            id: CreativeIdeaId::new(),
            title: title.into(),
            body: body.into(),
            dialect,
            score: 0.0,
            metadata: IndexMap::new(),
            created_at: Utc::now(),
        }
    }

    /// Attaches metadata.
    #[must_use]
    pub fn with_metadata(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.metadata.insert(key.into(), value);
        self
    }

    /// Sets the score.
    #[must_use]
    pub fn with_score(mut self, score: f32) -> Self {
        self.score = score.clamp(0.0, 1.0);
        self
    }
}

/// Collection of ideas.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CreativePortfolio {
    ideas: Vec<CreativeIdea>,
}

impl CreativePortfolio {
    /// Adds an idea to the portfolio.
    pub fn push(&mut self, idea: CreativeIdea) {
        self.ideas.push(idea);
    }

    /// Ranked ideas by score.
    #[must_use]
    pub fn ranked(&self) -> Vec<CreativeIdea> {
        let mut ideas = self.ideas.clone();
        ideas.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        ideas
    }

    /// Returns length.
    #[must_use]
    pub fn len(&self) -> usize {
        self.ideas.len()
    }

    /// Iterator over ideas.
    pub fn iter(&self) -> impl Iterator<Item = &CreativeIdea> {
        self.ideas.iter()
    }
}

impl From<Vec<CreativeIdea>> for CreativePortfolio {
    fn from(value: Vec<CreativeIdea>) -> Self {
        Self { ideas: value }
    }
}

/// Errors emitted by the ideation engine.
#[derive(Debug, Error)]
pub enum CreativityError {
    /// The brief was invalid.
    #[error("invalid brief: {0}")]
    InvalidBrief(String),
    /// Internal failure.
    #[error("ideation failure: {0}")]
    IdeationFailure(String),
}

/// Result of an ideation cycle.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdeationOutcome {
    /// Portfolio of generated ideas.
    pub portfolio: CreativePortfolio,
    /// Narrative summary.
    pub summary: String,
    /// Steps executed.
    pub steps: Vec<String>,
}

/// Engine responsible for generating and transforming ideas.
#[derive(Debug, Clone)]
pub struct IdeationEngine {
    rng: SmallRng,
    transformer: IdeaTransformer,
}

impl Default for IdeationEngine {
    fn default() -> Self {
        Self {
            rng: SmallRng::from_entropy(),
            transformer: IdeaTransformer::default(),
        }
    }
}

impl IdeationEngine {
    /// Creates an engine with a deterministic seed (useful for tests).
    #[must_use]
    pub fn seeded(seed: u64) -> Self {
        Self {
            rng: SmallRng::seed_from_u64(seed),
            transformer: IdeaTransformer::default(),
        }
    }

    /// Generates ideas from the provided brief.
    pub fn ideate(&mut self, brief: &CreativeBrief) -> Result<IdeationOutcome, CreativityError> {
        if brief.title.trim().is_empty() || brief.objective.trim().is_empty() {
            return Err(CreativityError::InvalidBrief(
                "brief title/objective cannot be empty".into(),
            ));
        }

        let raw_ideas = self.generate_raw_ideas(brief);
        let mut portfolio = CreativePortfolio::default();
        let mut steps = Vec::new();

        for (idx, mut idea) in raw_ideas.into_iter().enumerate() {
            steps.push(format!(
                "Idea {} crafted with {} dialect",
                idx + 1,
                idea.dialect.descriptor()
            ));
            idea = self.apply_constraints(idea, &brief.constraints);
            idea = self
                .transformer
                .polish(idea, brief.dialect.descriptor(), &brief.constraints);
            portfolio.push(idea);
        }

        let summary = format!(
            "Generated {} ideas for '{}' using {} dialect.",
            portfolio.len(),
            brief.title,
            brief.dialect.descriptor()
        );

        Ok(IdeationOutcome {
            portfolio,
            summary,
            steps,
        })
    }

    fn generate_raw_ideas(&mut self, brief: &CreativeBrief) -> Vec<CreativeIdea> {
        let mut ideas = Vec::new();
        let seeds = if brief.seed_ideas.is_empty() {
            vec![
                "Reframe the problem through a community lens".to_string(),
                "Inject speculative fiction to show long-term impact".to_string(),
            ]
        } else {
            brief.seed_ideas.clone()
        };

        let fragments = vec![
            "multi-sensory journey",
            "zero-gravity prototype",
            "self-healing narrative",
            "hyperlocal collective",
            "planetary resilience guild",
        ];

        for seed in seeds {
            let fragment = fragments.choose(&mut self.rng).unwrap().to_string();
            let title = format!("{} x {}", brief.title, fragment);
            let body = format!(
                "{}\n\nObjective: {}\nApproach: {} while amplifying {}.",
                seed,
                brief.objective,
                fragment,
                brief
                    .constraints
                    .audience
                    .clone()
                    .unwrap_or_else(|| "the collective imagination".into())
            );

            let score = self.rng.gen_range(0.55..0.98);
            ideas.push(
                CreativeIdea::new(title, body, brief.dialect.clone()).with_score(score as f32),
            );
        }

        ideas
    }

    fn apply_constraints(
        &self,
        mut idea: CreativeIdea,
        constraints: &CreativeConstraint,
    ) -> CreativeIdea {
        if let Some(max_length) = constraints.max_length {
            if idea.body.len() > max_length {
                idea.body.truncate(max_length);
            }
        }

        for keyword in &constraints.required_keywords {
            if !idea.body.contains(keyword) {
                idea.body.push_str(&format!("\nKey anchor: {}", keyword));
            }
        }

        for avoid in &constraints.avoid {
            if idea.body.contains(avoid) {
                idea.body = idea.body.replace(avoid, "[redacted]");
            }
        }

        idea
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ideation_engine_generates_portfolio() {
        let brief = CreativeBrief::new(
            "Aurora Grid",
            "Inspire climate action",
            CreativityDialect::Experimental,
        )
        .with_seed("Design a lunar-powered community ritual");
        let mut engine = IdeationEngine::seeded(42);
        let outcome = engine.ideate(&brief).unwrap();
        assert_eq!(outcome.portfolio.len(), 1);
        assert!(outcome.summary.contains("Aurora Grid"));
    }

    #[test]
    fn constraint_enforced() {
        let mut engine = IdeationEngine::seeded(7);
        let mut brief =
            CreativeBrief::new("Nova", "Reframe finance", CreativityDialect::Analytical);
        brief.constraints.max_length = Some(40);
        let outcome = engine.ideate(&brief).unwrap();
        assert!(outcome.portfolio.ranked()[0].body.len() <= 40);
    }
}
