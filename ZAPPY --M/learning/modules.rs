use indexmap::IndexMap;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Status of a learning submodule.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ModuleStatus {
    /// Ready to serve traffic.
    Active,
    /// Currently training.
    Training,
    /// Blocked due to issues.
    Blocked(String),
}

/// Lightweight descriptor for every learning module.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearningModuleDescriptor {
    /// Unique identifier.
    pub id: Uuid,
    /// Human readable name.
    pub name: String,
    /// Domain or purpose (e.g., "vision", "finance").
    pub domain: String,
    /// Current status.
    pub status: ModuleStatus,
}

impl LearningModuleDescriptor {
    /// Creates a new descriptor.
    #[must_use]
    pub fn new(name: impl Into<String>, domain: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            name: name.into(),
            domain: domain.into(),
            status: ModuleStatus::Training,
        }
    }
}

/// Registry that keeps track of all learning modules.
#[derive(Debug, Clone, Default)]
pub struct LearningModuleRegistry {
    inner: std::sync::Arc<RwLock<IndexMap<Uuid, LearningModuleDescriptor>>>,
}

impl LearningModuleRegistry {
    /// Registers or replaces a module descriptor.
    pub fn register(&self, descriptor: LearningModuleDescriptor) {
        self.inner.write().insert(descriptor.id, descriptor);
    }

    /// Marks the module status.
    pub fn set_status(&self, id: &Uuid, status: ModuleStatus) {
        if let Some(entry) = self.inner.write().get_mut(id) {
            entry.status = status;
        }
    }

    /// Returns all modules for a given domain.
    #[must_use]
    pub fn by_domain(&self, domain: &str) -> Vec<LearningModuleDescriptor> {
        self.inner
            .read()
            .values()
            .filter(|module| module.domain == domain)
            .cloned()
            .collect()
    }

    /// Snapshot of all modules.
    #[must_use]
    pub fn snapshot(&self) -> Vec<LearningModuleDescriptor> {
        self.inner.read().values().cloned().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_tracks_modules() {
        let registry = LearningModuleRegistry::default();
        let module = LearningModuleDescriptor::new("vision-core", "vision");
        let id = module.id;
        registry.register(module);
        registry.set_status(&id, ModuleStatus::Active);
        let snapshot = registry.snapshot();
        assert_eq!(snapshot.len(), 1);
        assert!(matches!(snapshot[0].status, ModuleStatus::Active));
    }
}
