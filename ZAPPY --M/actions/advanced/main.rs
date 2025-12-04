mod advancedallfuncs;

use crate::actions::{ActionPlan, ActionRequest};
pub use advancedallfuncs::{ScenarioQuantizer, ScenarioSummary};

/// Toolkit that performs advanced plan analytics and transformations.
#[derive(Debug, Default)]
pub struct AdvancedActionToolkit {
    quantizer: ScenarioQuantizer,
}

impl AdvancedActionToolkit {
    /// Creates a new toolkit.
    #[must_use]
    pub fn new() -> Self {
        Self {
            quantizer: ScenarioQuantizer::default(),
        }
    }

    /// Produces scenario summaries for operator dashboards.
    #[must_use]
    pub fn summarize(&self, plan: &ActionPlan) -> Vec<ScenarioSummary> {
        self.quantizer.quantize(plan)
    }

    /// Generates an accelerated plan for crisis response.
    #[must_use]
    pub fn accelerated(&self, plan: &ActionPlan) -> ActionPlan {
        self.quantizer.accelerated_plan(plan)
    }

    /// Validates the plan against the originating request.
    #[must_use]
    pub fn validate_alignment(&self, plan: &ActionPlan, request: &ActionRequest) -> bool {
        plan.hypothesis.contains(&request.payload.summary)
    }
}
