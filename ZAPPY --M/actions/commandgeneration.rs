use std::sync::Arc;

use async_trait::async_trait;
use chrono::Duration;
use rand::{seq::SliceRandom, thread_rng};

use crate::actions::{
    ActionDomain, ActionError, ActionIntent, ActionPlan, ActionRequest, ActionSafetyClass,
    ActionStep, PlanRiskProfile,
};

/// Generates executable plans from high-level requests.
#[async_trait]
pub trait CommandGenerator: Send + Sync {
    /// Produces a multi-step plan with risk annotations.
    async fn synthesize(&self, request: &ActionRequest) -> Result<ActionPlan, ActionError>;
}

/// Heuristic generator that blends templates with lightweight analysis.
#[derive(Debug)]
pub struct HeuristicCommandGenerator {
    risk_tolerance: f32,
    max_steps: usize,
    reviewers: Arc<Vec<String>>,
}

impl Default for HeuristicCommandGenerator {
    fn default() -> Self {
        Self {
            risk_tolerance: 0.35,
            max_steps: 8,
            reviewers: Arc::new(vec!["safety@zappy".into(), "ops@zappy".into()]),
        }
    }
}

impl HeuristicCommandGenerator {
    /// Creates a new generator with the provided risk tolerance.
    #[must_use]
    pub fn new(risk_tolerance: f32, max_steps: usize) -> Self {
        Self {
            risk_tolerance,
            max_steps,
            ..Self::default()
        }
    }

    /// Adds a reviewer identity.
    #[must_use]
    pub fn with_reviewer(mut self, reviewer: impl Into<String>) -> Self {
        Arc::get_mut(&mut self.reviewers)
            .expect("unique reference")
            .push(reviewer.into());
        self
    }
}

#[async_trait]
impl CommandGenerator for HeuristicCommandGenerator {
    async fn synthesize(&self, request: &ActionRequest) -> Result<ActionPlan, ActionError> {
        if request.payload.summary.len() < 3 {
            return Err(ActionError::Invalid(
                "summary must be at least three characters".into(),
            ));
        }

        let mut steps = Vec::new();
        let mut ordinal = 1;

        for template in templates_for(request.intent.clone(), &request.domain) {
            if ordinal > self.max_steps {
                break;
            }

            steps.push(ActionStep {
                ordinal,
                description: template.to_string(),
                domain: request.domain.clone(),
                required_capabilities: Default::default(),
                estimated_duration: Duration::minutes((ordinal as i64) * 5),
                dependencies: if ordinal == 1 {
                    Vec::new()
                } else {
                    vec![ordinal - 1]
                },
                instrumentation: Default::default(),
            });

            ordinal += 1;
        }

        if steps.is_empty() {
            return Err(ActionError::Planning("no plan template matched".into()));
        }

        steps.shrink_to_fit();

        let mut risk = PlanRiskProfile::default();
        risk.operational = (request.priority.as_score() as f32 / 100.0).min(1.0);
        risk.financial = self.risk_tolerance;
        risk.safety = match request.constraints.safety {
            ActionSafetyClass::Green => ActionSafetyClass::Green,
            ActionSafetyClass::Yellow => ActionSafetyClass::Yellow,
            ActionSafetyClass::Orange => ActionSafetyClass::Orange,
            ActionSafetyClass::Red => ActionSafetyClass::Red,
        };

        Ok(ActionPlan {
            id: format!("plan-{}", request.correlation_id),
            hypothesis: format!("{}::{:?}", request.payload.summary, request.intent),
            steps,
            risk,
        })
    }
}

fn templates_for(intent: ActionIntent, domain: &ActionDomain) -> Vec<&'static str> {
    let mut rng = thread_rng();
    let mut templates = match intent {
        ActionIntent::Observe => vec![
            "Capture telemetry baselines",
            "Normalize incoming signals",
            "Publish observation digest",
        ],
        ActionIntent::Simulate => vec![
            "Assemble model parameters",
            "Run coarse simulation",
            "Run fine-grained simulation",
            "Validate against historical data",
        ],
        ActionIntent::Optimize => vec![
            "Enumerate decision levers",
            "Score impact vs. risk",
            "Publish optimization recommendation",
        ],
        ActionIntent::Execute => vec![
            "Activate execution guardrail",
            "Apply change set",
            "Verify post-change stability",
        ],
        ActionIntent::Remediate => vec![
            "Contain active incident",
            "Backfill missing data",
            "Confirm remediation with stakeholders",
        ],
        ActionIntent::Coordinate => vec![
            "Assemble participant roster",
            "Distribute coordination brief",
            "Run synchronization checkpoint",
        ],
        ActionIntent::Program => vec![
            "Review code context",
            "Author patch",
            "Execute validation suite",
        ],
        ActionIntent::Learn => vec![
            "Gather training corpus",
            "Run training job",
            "Evaluate against holdout",
        ],
        ActionIntent::Audit => vec![
            "Collect audit evidence",
            "Score compliance posture",
            "Deliver audit findings",
        ],
    };

    match domain {
        ActionDomain::Security => templates.push("Engage security review board"),
        ActionDomain::Financial => templates.push("Perform risk ops approval"),
        ActionDomain::Programming => templates.push("Open change-management ticket"),
        ActionDomain::SelfTraining => templates.push("Update capability registry"),
        _ => {}
    }

    templates.shuffle(&mut rng);
    templates
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actions::{ActionMetadata, ActionPayload, ActionPriority, ActionRequest};

    #[tokio::test]
    async fn generates_plan_with_steps() {
        let payload = ActionPayload::textual("Upgrade net", "expand capacity");
        let request = ActionRequest::builder(
            ActionDomain::Infrastructure,
            ActionIntent::Optimize,
            payload,
        )
        .priority(ActionPriority::High)
        .metadata(ActionMetadata::default())
        .build();

        let generator = HeuristicCommandGenerator::default();
        let plan = generator.synthesize(&request).await.unwrap();

        assert!(!plan.steps.is_empty());
        assert_eq!(plan.id, format!("plan-{}", request.correlation_id));
    }
}
