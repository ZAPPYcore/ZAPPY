use chrono::{DateTime, Utc};
use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::create::{CreativeConstraint, CreativeIdea};

/// Utility that polishes and transforms ideas.
#[derive(Debug, Clone)]
pub struct IdeaTransformer {
    emphasis_regex: Regex,
}

impl Default for IdeaTransformer {
    fn default() -> Self {
        Self {
            emphasis_regex: Regex::new(r"(?i)innovation").unwrap(),
        }
    }
}

impl IdeaTransformer {
    /// Applies light-weight polishing to align with the requested dialect.
    #[must_use]
    pub fn polish(
        &self,
        mut idea: CreativeIdea,
        dialect_descriptor: &str,
        constraints: &CreativeConstraint,
    ) -> CreativeIdea {
        if self.emphasis_regex.is_match(&idea.body) {
            idea.body = self
                .emphasis_regex
                .replace_all(&idea.body, "bold innovation")
                .into_owned();
        }

        if let Some(audience) = &constraints.audience {
            idea.body
                .push_str(&format!("\nAudience resonance: {audience}"));
        }

        idea.with_metadata(
            "dialect",
            serde_json::json!({ "descriptor": dialect_descriptor }),
        )
    }
}

/// Narrative timeline that threads multiple ideas into a cohesive story.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NarrativeArc {
    /// Title of the arc.
    pub title: String,
    /// Ordered fragments.
    pub fragments: Vec<String>,
    /// Timestamp for auditing.
    pub generated_at: DateTime<Utc>,
}

/// Weaves narratives across a set of ideas.
#[derive(Debug, Default, Clone)]
pub struct NarrativeWeaver;

impl NarrativeWeaver {
    /// Creates a narrative arc from a set of ideas.
    #[must_use]
    pub fn weave(&self, ideas: &[CreativeIdea], label: impl Into<String>) -> NarrativeArc {
        let mut fragments = Vec::new();
        for idea in ideas {
            fragments.push(format!("{} → {}", idea.title, idea.dialect.descriptor()));
        }

        NarrativeArc {
            title: label.into(),
            fragments,
            generated_at: Utc::now(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::create::{CreativeIdea, CreativityDialect};

    #[test]
    fn transformer_adds_metadata() {
        let transformer = IdeaTransformer::default();
        let constraints = CreativeConstraint {
            max_length: None,
            required_keywords: vec![],
            avoid: vec![],
            audience: Some("climate stewards".into()),
        };
        let idea = CreativeIdea::new("test", "innovation surge", CreativityDialect::Experimental);
        let polished = transformer.polish(idea, "experimental", &constraints);
        assert!(polished.body.contains("Audience resonance"));
        assert!(polished.metadata.contains_key("dialect"));
    }

    #[test]
    fn narrative_weaver_threads_titles() {
        let ideas = vec![CreativeIdea::new(
            "Seed",
            "content",
            CreativityDialect::Poetic,
        )];
        let weaver = NarrativeWeaver::default();
        let arc = weaver.weave(&ideas, "story");
        assert_eq!(arc.fragments.len(), 1);
    }
}
