//! Metacognition runtime orchestrating cognition, scripts, and reviewers.

use std::sync::Arc;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::json;
use shared_logging::LogLevel;
use tokio::sync::Mutex;

use crate::{
    cmd::CommandSynthesizer,
    cognition::SelfObservation,
    executor::{CommandInsight, ReflectionExecutor},
    metacognition::MetaCognitionKernel,
    methods::{ReflectionMethod, ReflectionPlanner},
    reviewer::MetaReviewer,
    script::ScriptEngine,
    telemetry::MetacognitionTelemetry,
};

/// Runtime offering high-level APIs for initiating metacognitive reflections.
#[derive(Clone)]
pub struct MetacognitionRuntime {
    kernel: Arc<Mutex<MetaCognitionKernel>>,
    reviewer: MetaReviewer,
    script_engine: ScriptEngine,
    telemetry: Option<MetacognitionTelemetry>,
}

impl MetacognitionRuntime {
    /// Creates a new runtime with default parameters.
    #[must_use]
    pub fn new() -> Self {
        Self {
            kernel: Arc::new(Mutex::new(MetaCognitionKernel::default())),
            reviewer: MetaReviewer::default(),
            script_engine: ScriptEngine::default(),
            telemetry: None,
        }
    }

    /// Attaches telemetry sinks.
    #[must_use]
    pub fn with_telemetry(mut self, telemetry: MetacognitionTelemetry) -> Self {
        self.telemetry = Some(telemetry);
        self
    }

    /// Sets telemetry after construction.
    pub fn set_telemetry(&mut self, telemetry: MetacognitionTelemetry) {
        self.telemetry = Some(telemetry);
    }

    /// Returns telemetry handle if configured.
    #[must_use]
    pub fn telemetry(&self) -> Option<&MetacognitionTelemetry> {
        self.telemetry.as_ref()
    }

    /// Ingests a self-observation and triggers a reflection plan.
    pub async fn reflect(
        &self,
        observation: SelfObservation,
        method: ReflectionMethod,
    ) -> Result<ReflectionDigest> {
        let planner = ReflectionPlanner::default();
        let plan = planner.plan(observation.clone(), method)?;
        if let Some(tel) = &self.telemetry {
            let _ = tel.log(
                LogLevel::Info,
                "metacognition.plan.generated",
                json!({ "observation": observation.description, "method": format!("{:?}", method) }),
            );
        }
        let script = self.script_engine.render(&plan)?;
        let commands = CommandSynthesizer::synthesize(&plan);
        let insight = ReflectionExecutor::execute(&plan, &commands);
        let mut kernel = self.kernel.lock().await;
        let outcome = kernel.execute(plan, script).await?;
        self.reviewer.review(&outcome, &insight)?;
        if let Some(tel) = &self.telemetry {
            let _ = tel.log(
                LogLevel::Info,
                "metacognition.reflection.completed",
                json!({
                    "plan_id": outcome.plan_id,
                    "resilience": insight.resiliency_score
                }),
            );
            let _ = tel.event(
                "metacognition.reflection.completed",
                json!({
                    "plan_id": outcome.plan_id,
                    "resilience": insight.resiliency_score
                }),
            );
        }
        Ok(ReflectionDigest {
            summary: outcome.summary,
            resilience: insight.resiliency_score,
            diagnostics: insight.diagnostics,
        })
    }
}

/// Structured reflection digest consumed by downstream modules.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReflectionDigest {
    /// Human readable summary.
    pub summary: String,
    /// Normalized resiliency score.
    pub resilience: f32,
    /// Command diagnostics.
    pub diagnostics: Vec<CommandInsight>,
}
