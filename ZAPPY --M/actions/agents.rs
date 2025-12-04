use std::sync::Arc;

use async_trait::async_trait;
use chrono::Utc;
use indexmap::IndexMap;

use crate::{
    actions::{
        ActionDomain, ActionError, ActionEvent, ActionJournal, ActionOutcome, ActionPlan,
        ActionRequest, ActionStatus, ExecutionWindow,
    },
    internetact::{InternetActionExecutor, LoopbackNetworkClient, NetworkClient},
    offlineact::OfflineActionExecutor,
    programming::ProgrammingActionExecutor,
    programminghelper::ProgrammingHelper,
    security_link::SecurityGrade,
    self_trainact::{LoopbackTrainingInterface, SelfTrainingExecutor},
};

/// Trait implemented by every domain-specialized action executor.
#[async_trait]
pub trait ActionAgent: Send + Sync {
    /// Domain handled by this agent.
    fn domain(&self) -> ActionDomain;
    /// Human readable agent identifier.
    fn name(&self) -> &str;
    /// Executes the provided plan and returns an outcome.
    async fn execute(
        &self,
        request: ActionRequest,
        plan: ActionPlan,
        ctx: ExecutionContext,
    ) -> Result<ActionOutcome, ActionError>;
}

/// Context shared with agents when executing a plan.
#[derive(Debug, Clone)]
pub struct ExecutionContext {
    /// Journal for emitting lifecycle events.
    pub journal: ActionJournal,
    /// Security grade enforced for downstream operations.
    pub security_grade: SecurityGrade,
}

/// Registry that keeps track of available action agents.
#[derive(Default, Clone)]
pub struct AgentRegistry {
    agents: IndexMap<String, Arc<dyn ActionAgent>>,
    fallback: Option<Arc<dyn ActionAgent>>,
}

impl AgentRegistry {
    /// Builds a registry seeded with default hard-coded agents.
    #[must_use]
    pub fn production_default() -> Self {
        let mut registry = Self::default();
        let network_client: Arc<dyn NetworkClient> = Arc::new(LoopbackNetworkClient::default());
        let internet_agent = Arc::new(InternetOpsAgent {
            executor: InternetActionExecutor::new(Arc::clone(&network_client), 4),
            name: "internet_ops".into(),
        });
        registry.register(internet_agent);

        let offline_agent = Arc::new(OfflineOpsAgent {
            executor: OfflineActionExecutor::new(temp_dir(), 2),
            name: "offline_ops".into(),
        });
        registry.register(offline_agent);

        let programming_agent = Arc::new(ProgrammingOpsAgent {
            executor: ProgrammingActionExecutor::new(ProgrammingHelper::new(10_000)),
            name: "programming_ops".into(),
        });
        registry.register(programming_agent);

        let training_agent = Arc::new(SelfTrainingOpsAgent {
            executor: SelfTrainingExecutor::new(Arc::new(LoopbackTrainingInterface::default())),
            name: "self_training".into(),
        });
        registry.fallback = Some(training_agent);

        registry
    }

    /// Registers an agent for its declared domain label.
    pub fn register(&mut self, agent: Arc<dyn ActionAgent>) {
        self.agents
            .insert(agent.domain().label().to_string(), agent);
    }

    /// Resolves an agent for a given domain falling back when necessary.
    pub fn resolve(&self, domain: &ActionDomain) -> Option<Arc<dyn ActionAgent>> {
        self.agents
            .get(domain.label())
            .cloned()
            .or_else(|| self.fallback.clone())
    }
}

fn temp_dir() -> std::path::PathBuf {
    std::env::temp_dir().join("zappy_actions")
}

struct InternetOpsAgent {
    executor: InternetActionExecutor,
    name: String,
}

#[async_trait]
impl ActionAgent for InternetOpsAgent {
    fn domain(&self) -> ActionDomain {
        ActionDomain::Network
    }

    fn name(&self) -> &str {
        &self.name
    }

    async fn execute(
        &self,
        request: ActionRequest,
        plan: ActionPlan,
        ctx: ExecutionContext,
    ) -> Result<ActionOutcome, ActionError> {
        ctx.journal.push(ActionEvent {
            id: request.id,
            timestamp: Utc::now(),
            status: ActionStatus::Executing(ExecutionWindow::start()),
            note: Some("Internet agent executing".into()),
        });
        self.executor.execute_plan(&request, &plan).await
    }
}

struct OfflineOpsAgent {
    executor: OfflineActionExecutor,
    name: String,
}

#[async_trait]
impl ActionAgent for OfflineOpsAgent {
    fn domain(&self) -> ActionDomain {
        ActionDomain::Infrastructure
    }

    fn name(&self) -> &str {
        &self.name
    }

    async fn execute(
        &self,
        request: ActionRequest,
        plan: ActionPlan,
        ctx: ExecutionContext,
    ) -> Result<ActionOutcome, ActionError> {
        ctx.journal.push(ActionEvent {
            id: request.id,
            timestamp: Utc::now(),
            status: ActionStatus::Executing(ExecutionWindow::start()),
            note: Some("Offline agent executing".into()),
        });
        self.executor
            .execute_plan(&request, &plan, Vec::new())
            .await
    }
}

struct ProgrammingOpsAgent {
    executor: ProgrammingActionExecutor,
    name: String,
}

#[async_trait]
impl ActionAgent for ProgrammingOpsAgent {
    fn domain(&self) -> ActionDomain {
        ActionDomain::Programming
    }

    fn name(&self) -> &str {
        &self.name
    }

    async fn execute(
        &self,
        request: ActionRequest,
        plan: ActionPlan,
        ctx: ExecutionContext,
    ) -> Result<ActionOutcome, ActionError> {
        ctx.journal.push(ActionEvent {
            id: request.id,
            timestamp: Utc::now(),
            status: ActionStatus::Executing(ExecutionWindow::start()),
            note: Some("Programming agent executing".into()),
        });
        self.executor.execute_plan(&request, &plan).await
    }
}

struct SelfTrainingOpsAgent {
    executor: SelfTrainingExecutor,
    name: String,
}

#[async_trait]
impl ActionAgent for SelfTrainingOpsAgent {
    fn domain(&self) -> ActionDomain {
        ActionDomain::SelfTraining
    }

    fn name(&self) -> &str {
        &self.name
    }

    async fn execute(
        &self,
        request: ActionRequest,
        plan: ActionPlan,
        ctx: ExecutionContext,
    ) -> Result<ActionOutcome, ActionError> {
        ctx.journal.push(ActionEvent {
            id: request.id,
            timestamp: Utc::now(),
            status: ActionStatus::Executing(ExecutionWindow::start()),
            note: Some("Self-training agent executing".into()),
        });
        self.executor.execute_plan(&request, &plan).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn registry_resolves_default_agents() {
        let registry = AgentRegistry::production_default();
        assert!(registry.resolve(&ActionDomain::Programming).is_some());
    }
}
