use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Light-weight descriptor for a subsidiary submodel.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubsidiaryModel {
    /// Identifier.
    pub id: Uuid,
    /// Domain.
    pub domain: String,
    /// Capability label.
    pub capability: String,
    /// Performance metric.
    pub score: f32,
}

impl SubsidiaryModel {
    /// Creates a new descriptor.
    #[must_use]
    pub fn new(domain: impl Into<String>, capability: impl Into<String>, score: f32) -> Self {
        Self {
            id: Uuid::new_v4(),
            domain: domain.into(),
            capability: capability.into(),
            score,
        }
    }
}

/// Repository of subsidiary models.
#[derive(Debug, Default, Clone)]
pub struct SubsidiaryModelRegistry {
    models: Vec<SubsidiaryModel>,
}

impl SubsidiaryModelRegistry {
    /// Inserts a model.
    pub fn insert(&mut self, model: SubsidiaryModel) {
        self.models.push(model);
    }

    /// Finds the best model for a domain.
    #[must_use]
    pub fn best_for_domain(&self, domain: &str) -> Option<SubsidiaryModel> {
        self.models
            .iter()
            .filter(|model| model.domain == domain)
            .max_by(|a, b| a.score.partial_cmp(&b.score).unwrap())
            .cloned()
    }
}
