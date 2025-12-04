use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::cognition::SelfObservation;

/// Available reflection methods controlling scope and intensity.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum ReflectionMethod {
    /// Quick, lightweight reflection.
    RapidReview,
    /// Structured deep dive.
    StructuredAnalysis,
    /// Multi-step, multi-signal integration.
    ComprehensiveAudit,
}

/// Reflection plan describing steps to execute.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReflectionPlan {
    /// Observation being addressed.
    pub observation: SelfObservation,
    /// Method selected.
    pub method: ReflectionMethod,
    /// Steps in order.
    pub steps: Vec<String>,
    /// Deadline for completion.
    pub deadline: DateTime<Utc>,
}

/// Planner that produces reflection plans.
#[derive(Debug, Default)]
pub struct ReflectionPlanner;

impl ReflectionPlanner {
    /// Generates a plan from the observation/method pair.
    pub fn plan(
        &self,
        observation: SelfObservation,
        method: ReflectionMethod,
    ) -> anyhow::Result<ReflectionPlan> {
        let steps = match method {
            ReflectionMethod::RapidReview => vec![
                "Summarize observation".into(),
                "List immediate mitigation".into(),
            ],
            ReflectionMethod::StructuredAnalysis => vec![
                "Summarize observation".into(),
                "Gather supporting signals".into(),
                "Draft counterfactuals".into(),
                "Propose interventions".into(),
            ],
            ReflectionMethod::ComprehensiveAudit => vec![
                "Summarize observation".into(),
                "Aggregate multi-domain context".into(),
                "Run risk simulations".into(),
                "Present final remediation strategy".into(),
            ],
        };

        Ok(ReflectionPlan {
            observation,
            method,
            steps,
            deadline: Utc::now() + chrono::Duration::minutes(15),
        })
    }
}
