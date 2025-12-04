use std::{fmt, sync::Arc};

use chrono::{DateTime, Duration, Utc};
use indexmap::{IndexMap, IndexSet};
use parking_lot::RwLock;
use rand::{distributions::Alphanumeric, thread_rng, Rng};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

use crate::security_link::SecurityGrade;

/// Strongly typed identifier for every action that flows through the system.
pub type ActionId = Uuid;

/// Domains that the Tier-9 AGI can operate in.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ActionDomain {
    /// Planet-scale economic interventions.
    Economic,
    /// Defensive or offensive security operations.
    Security,
    /// High-stakes financial optimization.
    Financial,
    /// Global infrastructure orchestration.
    Infrastructure,
    /// AI research & deployment tasks.
    AiResearch,
    /// Cross-network telemetry and actuation.
    Network,
    /// Real-world manufacturing control.
    Manufacturing,
    /// Multi-national medical coordination.
    Medical,
    /// Simulation or digital twin actions.
    Simulation,
    /// Research and knowledge synthesis.
    Research,
    /// Code, automation, or tooling changes.
    Programming,
    /// Self-training and recursive improvement.
    SelfTraining,
    /// Custom domain supplied by operators.
    Custom(String),
}

impl ActionDomain {
    /// Returns a short human readable label.
    #[must_use]
    pub fn label(&self) -> &str {
        match self {
            Self::Economic => "economic",
            Self::Security => "security",
            Self::Financial => "financial",
            Self::Infrastructure => "infrastructure",
            Self::AiResearch => "ai_research",
            Self::Network => "network",
            Self::Manufacturing => "manufacturing",
            Self::Medical => "medical",
            Self::Simulation => "simulation",
            Self::Research => "research",
            Self::Programming => "programming",
            Self::SelfTraining => "self_training",
            Self::Custom(label) => label,
        }
    }
}

/// High-level intent that influences routing, risk scoring, and auditing.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ActionIntent {
    /// Collect or observe data without altering the environment.
    Observe,
    /// Run predictive or generative simulations.
    Simulate,
    /// Optimize a KPI or policy subject to constraints.
    Optimize,
    /// Execute a previously approved operational plan.
    Execute,
    /// Remediate failures, outages, or attacks.
    Remediate,
    /// Coordinate multi-agent plans and human approvals.
    Coordinate,
    /// Produce or refactor source code.
    Program,
    /// Self-train or fine-tune internal models.
    Learn,
    /// Audit for compliance, risk, or safety regressions.
    Audit,
}

impl ActionIntent {
    /// Returns a human readable label.
    #[must_use]
    pub fn label(&self) -> &'static str {
        match self {
            Self::Observe => "observe",
            Self::Simulate => "simulate",
            Self::Optimize => "optimize",
            Self::Execute => "execute",
            Self::Remediate => "remediate",
            Self::Coordinate => "coordinate",
            Self::Program => "program",
            Self::Learn => "learn",
            Self::Audit => "audit",
        }
    }
}

/// Priority level used by the global scheduler.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ActionPriority {
    /// Opportunistic or background work.
    Low,
    /// Default urgency.
    Normal,
    /// Time-sensitive requests.
    High,
    /// Immediate, potentially safety-related work.
    Critical,
}

impl Default for ActionPriority {
    fn default() -> Self {
        Self::Normal
    }
}

impl ActionPriority {
    /// Returns a comparable numeric score.
    #[must_use]
    pub fn as_score(&self) -> u8 {
        match self {
            Self::Low => 10,
            Self::Normal => 50,
            Self::High => 80,
            Self::Critical => 100,
        }
    }
}

/// Structured metadata attached to every action.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ActionMetadata {
    /// Optional operator or upstream system.
    pub origin: Option<String>,
    /// Arbitrary labels facilitating routing & analytics.
    pub tags: IndexSet<String>,
    /// References to previous actions enabling lineage tracking.
    pub lineage: IndexSet<ActionId>,
    /// Additional metadata payload.
    pub annotations: IndexMap<String, serde_json::Value>,
}

impl ActionMetadata {
    /// Adds a tag and returns self for chaining.
    #[must_use]
    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.insert(tag.into());
        self
    }

    /// Registers lineage and returns self for chaining.
    #[must_use]
    pub fn with_lineage(mut self, parent: ActionId) -> Self {
        self.lineage.insert(parent);
        self
    }

    /// Adds a structured annotation.
    #[must_use]
    pub fn with_annotation(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.annotations.insert(key.into(), value);
        self
    }
}

/// Attachment that retains structured content (JSON, table, documents).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PayloadAttachment {
    /// Operator supplied label.
    pub label: String,
    /// MIME-style hint.
    pub content_type: String,
    /// Raw content.
    pub content: serde_json::Value,
}

/// Typed payload evaluated by downstream agents.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionPayload {
    /// Short description for operator consoles.
    pub summary: String,
    /// Free-form instructions or context.
    pub narrative: String,
    /// Structured attachments.
    pub attachments: Vec<PayloadAttachment>,
}

impl ActionPayload {
    /// Convenience constructor for text-only requests.
    #[must_use]
    pub fn textual(summary: impl Into<String>, narrative: impl Into<String>) -> Self {
        Self {
            summary: summary.into(),
            narrative: narrative.into(),
            attachments: Vec::new(),
        }
    }
}

/// Safety guardrails tied to policy rules.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ActionSafetyClass {
    /// Safe sandbox operations.
    Green,
    /// Requires additional auditing.
    Yellow,
    /// Elevated risk, multi-party approval.
    Orange,
    /// Critical or existential risk.
    Red,
}

impl From<SecurityGrade> for ActionSafetyClass {
    fn from(grade: SecurityGrade) -> Self {
        match grade {
            SecurityGrade::Low => Self::Green,
            SecurityGrade::Medium => Self::Yellow,
            SecurityGrade::High => Self::Orange,
            SecurityGrade::Maximum => Self::Red,
        }
    }
}

impl From<ActionSafetyClass> for SecurityGrade {
    fn from(class: ActionSafetyClass) -> Self {
        match class {
            ActionSafetyClass::Green => Self::Low,
            ActionSafetyClass::Yellow => Self::Medium,
            ActionSafetyClass::Orange => Self::High,
            ActionSafetyClass::Red => Self::Maximum,
        }
    }
}

/// Regulatory, financial, and safety constraints.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionConstraints {
    /// Hard deadline.
    pub deadline: Option<DateTime<Utc>>,
    /// Acceptable compute cost in credits.
    pub max_compute_credits: Option<u64>,
    /// Acceptable financial exposure.
    pub max_financial_risk: Option<f64>,
    /// Jurisdictions that must be obeyed.
    pub jurisdictions: IndexSet<String>,
    /// Desired safety classification.
    pub safety: ActionSafetyClass,
}

impl Default for ActionConstraints {
    fn default() -> Self {
        Self {
            deadline: None,
            max_compute_credits: None,
            max_financial_risk: None,
            jurisdictions: IndexSet::new(),
            safety: ActionSafetyClass::Green,
        }
    }
}

/// Canonical representation of a requested action.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionRequest {
    /// Unique identifier.
    pub id: ActionId,
    /// Creation timestamp.
    pub created_at: DateTime<Utc>,
    /// Operational domain.
    pub domain: ActionDomain,
    /// High-level intent.
    pub intent: ActionIntent,
    /// Scheduling priority.
    pub priority: ActionPriority,
    /// Associated metadata.
    pub metadata: ActionMetadata,
    /// Work to be performed.
    pub payload: ActionPayload,
    /// Constraints & guardrails.
    pub constraints: ActionConstraints,
    /// Optional operator identity.
    pub requester: Option<String>,
    /// Correlation identifier for cross-system tracking.
    pub correlation_id: String,
}

impl ActionRequest {
    /// Creates a new builder for the request.
    #[must_use]
    pub fn builder(
        domain: ActionDomain,
        intent: ActionIntent,
        payload: ActionPayload,
    ) -> ActionRequestBuilder {
        ActionRequestBuilder {
            request: Self {
                id: ActionId::new_v4(),
                created_at: Utc::now(),
                domain,
                intent,
                priority: ActionPriority::default(),
                metadata: ActionMetadata::default(),
                payload,
                constraints: ActionConstraints::default(),
                requester: None,
                correlation_id: Self::generate_correlation_id(),
            },
        }
    }

    fn generate_correlation_id() -> String {
        thread_rng()
            .sample_iter(Alphanumeric)
            .take(16)
            .map(char::from)
            .collect()
    }
}

/// Builder used to construct complex requests fluently.
#[derive(Debug)]
pub struct ActionRequestBuilder {
    request: ActionRequest,
}

impl ActionRequestBuilder {
    /// Overrides the priority.
    #[must_use]
    pub fn priority(mut self, priority: ActionPriority) -> Self {
        self.request.priority = priority;
        self
    }

    /// Replaces the metadata block.
    #[must_use]
    pub fn metadata(mut self, metadata: ActionMetadata) -> Self {
        self.request.metadata = metadata;
        self
    }

    /// Applies constraints.
    #[must_use]
    pub fn constraints(mut self, constraints: ActionConstraints) -> Self {
        self.request.constraints = constraints;
        self
    }

    /// Records the requester.
    #[must_use]
    pub fn requester(mut self, requester: impl Into<String>) -> Self {
        self.request.requester = Some(requester.into());
        self
    }

    /// Supplies a correlation identifier.
    #[must_use]
    pub fn correlation_id(mut self, correlation_id: impl Into<String>) -> Self {
        self.request.correlation_id = correlation_id.into();
        self
    }

    /// Consumes the builder returning the finalized request.
    #[must_use]
    pub fn build(self) -> ActionRequest {
        self.request
    }
}

/// Fully elaborated action plan.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ActionPlan {
    /// Plan identifier for auditing.
    pub id: String,
    /// Hypothesis or guiding principle.
    pub hypothesis: String,
    /// Ordered steps.
    pub steps: Vec<ActionStep>,
    /// Risk profile and approvals.
    pub risk: PlanRiskProfile,
}

impl ActionPlan {
    /// Creates a minimal plan with autogenerated id.
    #[must_use]
    pub fn new(hypothesis: impl Into<String>, steps: Vec<ActionStep>) -> Self {
        Self {
            id: format!("plan-{}", ActionRequest::generate_correlation_id()),
            hypothesis: hypothesis.into(),
            steps,
            risk: PlanRiskProfile::default(),
        }
    }

    /// Calculates a blended risk score.
    #[must_use]
    pub fn blended_risk(&self) -> f32 {
        (self.risk.operational + self.risk.financial) / 2.0
    }
}

/// Individual step in a plan.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionStep {
    /// Ordinal index.
    pub ordinal: usize,
    /// Description of work.
    pub description: String,
    /// Domain responsible for the step.
    pub domain: ActionDomain,
    /// Capabilities required to execute the step.
    pub required_capabilities: IndexSet<String>,
    /// Estimated duration.
    pub estimated_duration: Duration,
    /// Dependencies referencing other ordinals.
    pub dependencies: Vec<usize>,
    /// Telemetry instrumentation hints.
    pub instrumentation: IndexMap<String, String>,
}

impl ActionStep {
    /// Creates a new step with no dependencies.
    #[must_use]
    pub fn atomic(
        ordinal: usize,
        description: impl Into<String>,
        domain: ActionDomain,
        estimated_duration: Duration,
    ) -> Self {
        Self {
            ordinal,
            description: description.into(),
            domain,
            required_capabilities: IndexSet::new(),
            estimated_duration,
            dependencies: Vec::new(),
            instrumentation: IndexMap::new(),
        }
    }
}

/// Quantified risk data for a plan.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanRiskProfile {
    /// Operational risk between 0 and 1.
    pub operational: f32,
    /// Financial risk between 0 and 1.
    pub financial: f32,
    /// Safety class mandated for execution.
    pub safety: ActionSafetyClass,
}

impl Default for PlanRiskProfile {
    fn default() -> Self {
        Self {
            operational: 0.2,
            financial: 0.2,
            safety: ActionSafetyClass::Green,
        }
    }
}

/// Execution status of an action.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ActionStatus {
    /// Received and awaiting validation.
    Pending(ActionPriority),
    /// Undergoing safety & policy checks.
    Validating,
    /// Validation rejected the action.
    Rejected(Vec<ValidationIssue>),
    /// Planning succeeded and produced a plan.
    Planned(ActionPlan),
    /// Currently executing with metrics.
    Executing(ExecutionWindow),
    /// Completed successfully.
    Completed(ActionOutcome),
    /// Failed with an error.
    Failed(ActionError),
    /// Cancelled by operator.
    Cancelled(String),
}

impl ActionStatus {
    /// Whether the status is terminal.
    #[must_use]
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            Self::Completed(_) | Self::Failed(_) | Self::Cancelled(_) | Self::Rejected(_)
        )
    }
}

/// Timeline information captured while executing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionWindow {
    /// Start timestamp.
    pub started_at: DateTime<Utc>,
    /// Optional completion timestamp.
    pub finished_at: Option<DateTime<Utc>>,
    /// Reported progress percentage.
    pub progress: f32,
    /// Real-time metrics snapshot.
    pub metrics: ExecutionMetrics,
}

impl ExecutionWindow {
    /// Creates a new window at the current time.
    #[must_use]
    pub fn start() -> Self {
        Self {
            started_at: Utc::now(),
            finished_at: None,
            progress: 0.0,
            metrics: ExecutionMetrics::default(),
        }
    }
}

/// Successful outcome of an action.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionOutcome {
    /// Narrative summary of the result.
    pub summary: String,
    /// Produced artifacts.
    pub artifacts: Vec<ActionArtifact>,
    /// Follow-up recommendations.
    pub follow_up: Vec<String>,
    /// Execution metrics.
    pub metrics: ExecutionMetrics,
}

impl ActionOutcome {
    /// Convenience constructor for text-based outcomes.
    #[must_use]
    pub fn textual(summary: impl Into<String>, artifacts: Vec<ActionArtifact>) -> Self {
        Self {
            summary: summary.into(),
            artifacts,
            follow_up: Vec::new(),
            metrics: ExecutionMetrics::default(),
        }
    }
}

/// Artifact generated by actions (report, code, dataset, etc.).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionArtifact {
    /// Artifact label.
    pub label: String,
    /// Importance classification.
    pub importance: ActionPriority,
    /// Underlying content.
    pub content: ArtifactContent,
}

/// Variants of artifact storage strategies.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ArtifactContent {
    /// Inline UTF-8 payload.
    Text(String),
    /// Structured JSON document.
    Json(serde_json::Value),
    /// External URI referencing object storage.
    ExternalUri(String),
}

/// Real-time metrics captured during execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionMetrics {
    /// CPU time consumed in milliseconds.
    pub cpu_time_ms: u64,
    /// Resident memory footprint in bytes.
    pub memory_bytes: u64,
    /// Network transfer in bytes.
    pub network_bytes: u64,
    /// Estimated energy draw in kWh.
    pub energy_kwh: f64,
}

impl Default for ExecutionMetrics {
    fn default() -> Self {
        Self {
            cpu_time_ms: 0,
            memory_bytes: 0,
            network_bytes: 0,
            energy_kwh: 0.0,
        }
    }
}

/// Errors surfaced throughout the lifecycle.
#[derive(Debug, Error, Clone, Serialize, Deserialize)]
pub enum ActionError {
    /// Input validation failed.
    #[error("invalid action: {0}")]
    Invalid(String),
    /// Security enforcement rejected the action.
    #[error("security violation ({grade:?}): {message}")]
    SecurityViolation {
        /// Grade that was violated.
        grade: SecurityGrade,
        /// Human readable detail.
        message: String,
    },
    /// Planner could not produce a viable strategy.
    #[error("planning failure: {0}")]
    Planning(String),
    /// Runtime execution failed.
    #[error("execution failure: {0}")]
    Execution(String),
    /// Action exceeded the permitted deadline.
    #[error("timeout after {0:?}")]
    Timeout(Duration),
    /// Internal infrastructure failure.
    #[error("infrastructure: {0}")]
    Infrastructure(String),
}

/// Structured validation issue.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationIssue {
    /// Field affected.
    pub field: String,
    /// Severity.
    pub severity: ActionSafetyClass,
    /// Description.
    pub detail: String,
}

/// Event emitted on any state transition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionEvent {
    /// Action identifier.
    pub id: ActionId,
    /// Timestamp of the event.
    pub timestamp: DateTime<Utc>,
    /// Status after transition.
    pub status: ActionStatus,
    /// Optional operator note.
    pub note: Option<String>,
}

impl fmt::Display for ActionEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}@{} -> {:?}", self.id, self.timestamp, self.status)
    }
}

/// Durable log of action transitions for auditing.
#[derive(Debug, Clone, Default)]
pub struct ActionJournal {
    entries: Arc<RwLock<Vec<ActionEvent>>>,
}

impl ActionJournal {
    /// Creates a new empty journal.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Appends an event to the journal.
    pub fn push(&self, event: ActionEvent) {
        self.entries.write().push(event);
    }

    /// Returns a snapshot of the log.
    #[must_use]
    pub fn snapshot(&self) -> Vec<ActionEvent> {
        self.entries.read().clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builder_assigns_defaults() {
        let payload = ActionPayload::textual("summ", "narrative");
        let request =
            ActionRequest::builder(ActionDomain::Programming, ActionIntent::Program, payload)
                .priority(ActionPriority::High)
                .requester("tester")
                .build();

        assert_eq!(request.priority, ActionPriority::High);
        assert!(request.correlation_id.len() >= 8);
        assert_eq!(request.domain, ActionDomain::Programming);
    }
}
