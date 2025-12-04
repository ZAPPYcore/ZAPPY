//! Learning orchestration runtime tying classical, deep, combining, and subsidiary loops together.

use crate::{
    classical_ml::{editor::Dataset, reporter::TrainingReport, ClassicalMlPipeline},
    combining::{
        combining::CombinationEngine, func::normalize_weights, reviewer::CombinationReviewer,
    },
    deep_learning::{reporter::DlReport, DeepLearningPipeline},
    modules::{LearningModuleDescriptor, LearningModuleRegistry},
    subsidiary::{
        define::{SubsidiaryPlan, SubsidiaryTask},
        submodels::SubsidiaryModel,
        SubsidiaryLearningRuntime,
    },
    telemetry::LearningTelemetry,
};

use crate::classical_ml::submodel::SubModelManager;
use crate::combining::combining::CombinationResult;
use serde_json::{json, Value};
use shared_logging::LogLevel;

/// Top-level runtime coordinating every learning pipeline.
pub struct LearningRuntime {
    registry: LearningModuleRegistry,
    classical: ClassicalMlPipeline,
    deep: DeepLearningPipeline,
    combination: CombinationEngine,
    subsidiary: SubsidiaryLearningRuntime,
    telemetry: Option<LearningTelemetry>,
}

impl LearningRuntime {
    /// Creates a new runtime with default components.
    #[must_use]
    pub fn new() -> Self {
        Self {
            registry: LearningModuleRegistry::default(),
            classical: ClassicalMlPipeline::default(),
            deep: DeepLearningPipeline::default(),
            combination: CombinationEngine::new(CombinationReviewer::default()),
            subsidiary: SubsidiaryLearningRuntime::default(),
            telemetry: None,
        }
    }

    /// Attaches telemetry sinks for structured logging/events.
    pub fn with_telemetry(mut self, telemetry: LearningTelemetry) -> Self {
        self.telemetry = Some(telemetry);
        self
    }

    /// Sets telemetry after construction.
    pub fn set_telemetry(&mut self, telemetry: LearningTelemetry) {
        self.telemetry = Some(telemetry);
    }

    /// Registers a module descriptor.
    pub fn register_module(&self, descriptor: LearningModuleDescriptor) {
        self.registry.register(descriptor);
    }

    /// Runs the classical ML pipeline.
    pub fn run_classical(&self, dataset: Dataset) -> anyhow::Result<TrainingReport> {
        self.classical
            .run_with_telemetry(dataset, self.telemetry.as_ref())
    }

    /// Runs the deep learning pipeline.
    pub fn run_deep(&mut self) -> anyhow::Result<DlReport> {
        self.deep.run_with_telemetry(self.telemetry.as_ref())
    }

    /// Combines predictions from submodels.
    pub fn combine_predictions(
        &self,
        mut manager: SubModelManager,
        features: &[Vec<f32>],
    ) -> anyhow::Result<CombinationResult> {
        normalize_weights(&mut manager.models);
        self.log(
            LogLevel::Debug,
            "combine_predictions",
            json!({ "submodels": manager.models.len(), "batch": features.len() }),
        );
        self.event(
            "learning.combine.invoked",
            json!({ "submodels": manager.models.len(), "batch": features.len() }),
        );
        self.combination.combine(&manager, features)
    }

    /// Adds a subsidiary task + model for planning.
    pub fn add_subsidiary_task(&self, task: SubsidiaryTask) {
        let domain = task.domain.clone();
        let priority = task.priority;
        self.subsidiary.add_task(task);
        self.log(
            LogLevel::Info,
            "subsidiary_task_added",
            json!({ "domain": domain, "priority": priority }),
        );
        self.event(
            "learning.subsidiary.task_added",
            json!({ "domain": domain, "priority": priority }),
        );
    }

    /// Adds a subsidiary model.
    pub fn add_subsidiary_model(&mut self, model: SubsidiaryModel) {
        let domain = model.domain.clone();
        let capability = model.capability.clone();
        self.subsidiary.add_model(model);
        self.log(
            LogLevel::Info,
            "subsidiary_model_added",
            json!({ "domain": domain, "capability": capability }),
        );
        self.event(
            "learning.subsidiary.model_added",
            json!({ "domain": domain, "capability": capability }),
        );
    }

    /// Plans subsidiary work for a domain.
    pub fn plan_subsidiary(&self, domain: &str) -> Vec<SubsidiaryPlan> {
        let plans = self.subsidiary.plan(domain, 5);
        self.log(
            LogLevel::Info,
            "subsidiary_plan_generated",
            json!({ "domain": domain, "plans": plans.len() }),
        );
        self.event(
            "learning.subsidiary.plan_generated",
            json!({ "domain": domain, "plans": plans.len() }),
        );
        plans
    }

    fn log(&self, level: LogLevel, message: &str, metadata: Value) {
        if let Some(telemetry) = self.telemetry.as_ref() {
            let _ = telemetry.log(level, message, metadata);
        }
    }

    fn event(&self, event_type: &str, payload: Value) {
        if let Some(telemetry) = self.telemetry.as_ref() {
            let _ = telemetry.event(event_type, payload);
        }
    }
}
