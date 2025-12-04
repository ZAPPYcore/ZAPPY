//! Subsidiary learning orchestration modules.

/// Task definitions and planning primitives.
pub mod define;
/// Persistence utilities for subsidiary tasks.
pub mod saver;
/// Search utilities for prioritizing tasks.
pub mod searching;
/// Registry of subsidiary models.
pub mod submodels;

use define::{SubsidiaryPlan, SubsidiaryTask};
use saver::SubsidiaryStore;
use searching::TaskSearcher;
use submodels::{SubsidiaryModel, SubsidiaryModelRegistry};

/// Runtime that assigns tasks to subsidiary models.
#[derive(Debug, Default)]
pub struct SubsidiaryLearningRuntime {
    store: SubsidiaryStore,
    models: SubsidiaryModelRegistry,
}

impl SubsidiaryLearningRuntime {
    /// Registers a new task.
    pub fn add_task(&self, task: SubsidiaryTask) {
        self.store.add_task(task);
    }

    /// Registers a model.
    pub fn add_model(&mut self, model: SubsidiaryModel) {
        self.models.insert(model);
    }

    /// Generates plans by matching top tasks with best models.
    pub fn plan(&self, domain: &str, min_priority: u8) -> Vec<SubsidiaryPlan> {
        let searcher = TaskSearcher::new(self.store.clone());
        let tasks = searcher.search(domain, min_priority);
        let mut plans = Vec::new();
        for result in tasks {
            if let Some(model) = self.models.best_for_domain(domain) {
                let plan = SubsidiaryPlan {
                    task_id: result.task.id,
                    submodel_id: model.id,
                    notes: format!("score={:.2}", result.score),
                };
                self.store.add_plan(plan.clone());
                plans.push(plan);
            }
        }
        plans
    }
}
