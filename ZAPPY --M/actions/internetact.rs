use std::{collections::BTreeMap, sync::Arc};

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tokio::{sync::Semaphore, task::JoinSet};

use crate::actions::{ActionError, ActionOutcome, ActionPlan, ActionRequest};

/// Supported HTTP verbs for generated commands.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum HttpMethod {
    /// HTTP GET.
    Get,
    /// HTTP POST.
    Post,
    /// HTTP PUT.
    Put,
    /// HTTP DELETE.
    Delete,
}

/// Declarative representation of an outbound HTTP call.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpCommand {
    /// HTTP verb.
    pub method: HttpMethod,
    /// Fully qualified URL.
    pub url: String,
    /// Headers applied to the call.
    pub headers: BTreeMap<String, String>,
    /// JSON payload.
    pub body: serde_json::Value,
}

impl HttpCommand {
    /// Creates a JSON POST command.
    #[must_use]
    pub fn json_post(url: impl Into<String>, body: serde_json::Value) -> Self {
        Self {
            method: HttpMethod::Post,
            url: url.into(),
            headers: BTreeMap::new(),
            body,
        }
    }
}

/// Response metadata tracked for auditing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpResponse {
    /// Status code.
    pub status: u16,
    /// Body payload.
    pub body: serde_json::Value,
    /// Latency in milliseconds.
    pub latency_ms: u128,
}

/// Errors surfaced by network clients.
#[derive(Debug, thiserror::Error)]
pub enum NetworkError {
    /// Transport failure.
    #[error("transport error: {0}")]
    Transport(String),
}

/// Abstraction over concrete HTTP clients (reqwest, hyper, etc.).
#[async_trait]
pub trait NetworkClient: Send + Sync {
    /// Sends the HTTP command returning a response.
    async fn send(&self, command: HttpCommand) -> Result<HttpResponse, NetworkError>;
}

/// In-memory loopback client used for testing and offline simulation.
#[derive(Debug, Default)]
pub struct LoopbackNetworkClient;

#[async_trait]
impl NetworkClient for LoopbackNetworkClient {
    async fn send(&self, command: HttpCommand) -> Result<HttpResponse, NetworkError> {
        Ok(HttpResponse {
            status: 200,
            body: serde_json::json!({
                "echo": command.body,
                "url": command.url,
            }),
            latency_ms: 12,
        })
    }
}

/// Executes `ActionPlan`s that interact with internet-facing systems.
#[derive(Clone)]
pub struct InternetActionExecutor {
    client: Arc<dyn NetworkClient>,
    semaphore: Arc<Semaphore>,
}

impl InternetActionExecutor {
    /// Creates a new executor with the provided concurrency limit.
    #[must_use]
    pub fn new(client: Arc<dyn NetworkClient>, max_concurrency: usize) -> Self {
        Self {
            client,
            semaphore: Arc::new(Semaphore::new(max_concurrency.max(1))),
        }
    }

    /// Executes the plan, producing an outcome with HTTP artifacts.
    pub async fn execute_plan(
        &self,
        request: &ActionRequest,
        plan: &ActionPlan,
    ) -> Result<ActionOutcome, ActionError> {
        let mut set = JoinSet::new();
        for step in &plan.steps {
            let command = HttpCommand::json_post(
                format!(
                    "https://api.zappy/{}/{}",
                    request.domain.label(),
                    step.ordinal
                ),
                serde_json::json!({
                    "summary": request.payload.summary,
                    "step": step.description,
                    "intent": format!("{:?}", request.intent),
                }),
            );

            let client = Arc::clone(&self.client);
            let semaphore = Arc::clone(&self.semaphore);

            set.spawn(async move {
                let permit = semaphore.acquire_owned().await.expect("semaphore");
                let _hold = permit;
                client.send(command).await
            });
        }

        let mut responses = Vec::new();
        while let Some(result) = set.join_next().await {
            let response = result
                .map_err(|err| ActionError::Execution(err.to_string()))?
                .map_err(|err| ActionError::Execution(err.to_string()))?;
            responses.push(response);
        }

        let success = responses
            .iter()
            .filter(|r| (200..300).contains(&r.status))
            .count();

        let summary = format!(
            "Executed {} HTTP steps ({} successful)",
            plan.steps.len(),
            success
        );

        Ok(ActionOutcome::textual(
            summary,
            vec![crate::actions::ActionArtifact {
                label: "http_campaign".into(),
                importance: request.priority,
                content: crate::actions::ArtifactContent::Json(serde_json::json!(responses)),
            }],
        ))
    }
}
