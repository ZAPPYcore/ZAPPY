use indexmap::IndexMap;
use parking_lot::RwLock;

use super::define::{SubsidiaryPlan, SubsidiaryTask};

/// Persistence layer for subsidiary tasks and plans.
#[derive(Debug, Default, Clone)]
pub struct SubsidiaryStore {
    tasks: std::sync::Arc<RwLock<IndexMap<uuid::Uuid, SubsidiaryTask>>>,
    plans: std::sync::Arc<RwLock<Vec<SubsidiaryPlan>>>,
}

impl SubsidiaryStore {
    /// Adds a task.
    pub fn add_task(&self, task: SubsidiaryTask) {
        self.tasks.write().insert(task.id, task);
    }

    /// Adds a plan.
    pub fn add_plan(&self, plan: SubsidiaryPlan) {
        self.plans.write().push(plan);
    }

    /// Returns all tasks filtered by domain.
    #[must_use]
    pub fn tasks_by_domain(&self, domain: &str) -> Vec<SubsidiaryTask> {
        self.tasks
            .read()
            .values()
            .filter(|task| task.domain == domain)
            .cloned()
            .collect()
    }

    /// Returns all plans.
    #[must_use]
    pub fn plans(&self) -> Vec<SubsidiaryPlan> {
        self.plans.read().clone()
    }
}
