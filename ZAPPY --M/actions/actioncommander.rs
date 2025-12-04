use std::sync::Arc;

use serde_json::json;
use shared_logging::LogLevel;
use tokio::sync::oneshot;

use crate::{
    actions::{ActionError, ActionJournal, ActionOutcome, ActionRequest},
    agents::{AgentRegistry, ExecutionContext},
    commandgeneration::{CommandGenerator, HeuristicCommandGenerator},
    security_link::SecurityLink,
    telemetry::ActionTelemetry,
};

/// Builder used to configure an [`ActionCommander`].
pub struct ActionCommanderBuilder {
    registry: AgentRegistry,
    generator: Arc<dyn CommandGenerator>,
    security: SecurityLink,
    telemetry: Option<ActionTelemetry>,
}

impl Default for ActionCommanderBuilder {
    fn default() -> Self {
        Self {
            registry: AgentRegistry::production_default(),
            generator: Arc::new(HeuristicCommandGenerator::default()),
            security: SecurityLink::builder().build(),
            telemetry: None,
        }
    }
}

impl ActionCommanderBuilder {
    /// Overrides the agent registry.
    #[must_use]
    pub fn registry(mut self, registry: AgentRegistry) -> Self {
        self.registry = registry;
        self
    }

    /// Overrides the command generator.
    #[must_use]
    pub fn generator(mut self, generator: Arc<dyn CommandGenerator>) -> Self {
        self.generator = generator;
        self
    }

    /// Overrides the security link.
    #[must_use]
    pub fn security(mut self, security: SecurityLink) -> Self {
        self.security = security;
        self
    }

    /// Attaches telemetry sinks.
    #[must_use]
    pub fn telemetry(mut self, telemetry: ActionTelemetry) -> Self {
        self.telemetry = Some(telemetry);
        self
    }

    /// Finalizes the builder returning an [`ActionCommander`].
    #[must_use]
    pub fn build(self) -> ActionCommander {
        ActionCommander {
            registry: self.registry,
            generator: self.generator,
            security: self.security,
            journal: ActionJournal::new(),
            telemetry: self.telemetry,
        }
    }
}

/// Orchestrates security, planning, and agent execution.
pub struct ActionCommander {
    registry: AgentRegistry,
    generator: Arc<dyn CommandGenerator>,
    security: SecurityLink,
    journal: ActionJournal,
    telemetry: Option<ActionTelemetry>,
}

impl ActionCommander {
    /// Creates a builder with hardened defaults.
    #[must_use]
    pub fn builder() -> ActionCommanderBuilder {
        ActionCommanderBuilder::default()
    }

    /// Accesses the journal for observability.
    #[must_use]
    pub fn journal(&self) -> ActionJournal {
        self.journal.clone()
    }

    /// Submits an action for execution.
    pub async fn submit(&self, request: ActionRequest) -> Result<ExecutionHandle, ActionError> {
        self.log(
            LogLevel::Info,
            "actions.request.accepted",
            json!({
                "action_id": request.id,
                "domain": request.domain.label(),
                "intent": request.intent.label(),
                "priority": format!("{:?}", request.priority)
            }),
        );
        self.event(
            "actions.request.accepted",
            json!({
                "action_id": request.id,
                "domain": request.domain.label(),
                "intent": request.intent.label(),
                "priority": format!("{:?}", request.priority)
            }),
        );

        let verdict = self.security.enforce(&request).await?;
        self.log(
            LogLevel::Info,
            "actions.security.verdict",
            json!({
                "action_id": request.id,
                "grade": format!("{:?}", verdict.grade),
                "domain": request.domain.label()
            }),
        );
        self.event(
            "actions.security.verdict",
            json!({
                "action_id": request.id,
                "grade": format!("{:?}", verdict.grade)
            }),
        );

        let plan = self.generator.synthesize(&request).await?;
        self.log(
            LogLevel::Info,
            "actions.plan.generated",
            json!({
                "action_id": request.id,
                "plan_id": plan.id,
                "steps": plan.steps.len(),
                "risk": plan.blended_risk()
            }),
        );
        self.event(
            "actions.plan.generated",
            json!({
                "action_id": request.id,
                "plan_id": plan.id,
                "steps": plan.steps.len()
            }),
        );
        let agent = self
            .registry
            .resolve(&request.domain)
            .ok_or_else(|| ActionError::Infrastructure("no agent for domain".into()))?;

        let ctx = ExecutionContext {
            journal: self.journal.clone(),
            security_grade: verdict.grade,
        };

        self.log(
            LogLevel::Info,
            "actions.agent.resolved",
            json!({
                "action_id": request.id,
                "agent": agent.name(),
                "domain": agent.domain().label()
            }),
        );
        self.event(
            "actions.agent.resolved",
            json!({
                "action_id": request.id,
                "agent": agent.name()
            }),
        );

        let (tx, rx) = oneshot::channel();
        let telemetry = self.telemetry.clone();
        let action_id = request.id;
        tokio::spawn(async move {
            let result = agent.execute(request, plan, ctx).await;
            if let Some(tel) = telemetry {
                match &result {
                    Ok(outcome) => {
                        let _ = tel.log(
                            LogLevel::Info,
                            "actions.agent.completed",
                            json!({
                                "action_id": action_id,
                                "summary": outcome.summary,
                            }),
                        );
                        let _ = tel.event(
                            "actions.agent.completed",
                            json!({
                                "action_id": action_id,
                                "status": "success"
                            }),
                        );
                    }
                    Err(err) => {
                        let _ = tel.log(
                            LogLevel::Error,
                            "actions.agent.failed",
                            json!({
                                "error": err.to_string(),
                            }),
                        );
                        let _ =
                            tel.event("actions.agent.failed", json!({ "error": err.to_string() }));
                    }
                }
            }
            let _ = tx.send(result);
        });

        Ok(ExecutionHandle { rx })
    }
}

/// Handle returned to await action completion.
pub struct ExecutionHandle {
    rx: oneshot::Receiver<Result<ActionOutcome, ActionError>>,
}

impl ExecutionHandle {
    /// Awaits the final outcome.
    pub async fn outcome(self) -> Result<ActionOutcome, ActionError> {
        self.rx
            .await
            .map_err(|err| ActionError::Infrastructure(err.to_string()))?
    }
}

impl ActionCommander {
    fn log(&self, level: LogLevel, message: &str, metadata: serde_json::Value) {
        if let Some(tel) = &self.telemetry {
            let _ = tel.log(level, message, metadata);
        }
    }

    fn event(&self, event_type: &str, payload: serde_json::Value) {
        if let Some(tel) = &self.telemetry {
            let _ = tel.event(event_type, payload);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actions::{
        ActionDomain, ActionIntent, ActionPayload, ActionRequest, PayloadAttachment,
    };

    #[tokio::test]
    async fn commander_executes_programming_request() {
        let payload = ActionPayload {
            summary: "Refactor module".into(),
            narrative: "Improve maintainability".into(),
            attachments: vec![PayloadAttachment {
                label: "code_context".into(),
                content_type: "application/json".into(),
                content: serde_json::json!({
                    "path": "src/lib.rs",
                    "original": "fn old() {}",
                    "proposed": "fn new_func() {}"
                }),
            }],
        };

        let request =
            ActionRequest::builder(ActionDomain::Programming, ActionIntent::Program, payload)
                .build();

        let commander = ActionCommander::builder().build();
        let handle = commander.submit(request).await.unwrap();
        let outcome = handle.outcome().await.unwrap();
        assert!(outcome.summary.contains("Prepared"));
    }
}
