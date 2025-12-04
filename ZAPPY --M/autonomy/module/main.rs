//! Module coordination utilities for the autonomy kernel.

/// Smoothing and normalization helpers.
pub mod helper;
/// Improvisation strategies for directives.
pub mod improvise;
/// Lightweight neural abstractions.
pub mod neuron;

use std::{fmt, sync::Arc};

use chrono::{DateTime, Duration, Utc};
use helper::{normalize_scores, SignalSmoother};
use improvise::{ImprovisationEngine, ImprovisationHint};
use indexmap::IndexMap;
use neuron::{NeuronGraph, NeuronPulse};
use parking_lot::{Mutex, RwLock};
use rand::{distributions::Alphanumeric, thread_rng, Rng};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

/// Identifier assigned to every module.
pub type ModuleId = Uuid;

/// Errors surfaced by the module orchestration layer.
#[derive(Debug, Error, Clone)]
pub enum AutonomyError {
    /// The requested module does not exist.
    #[error("module not found: {0}")]
    ModuleNotFound(String),
    /// There is no module of the requested type.
    #[error("no module registered for kind {0:?}")]
    MissingKind(ModuleKind),
    /// Catch-all for internal issues.
    #[error("internal autonomy error: {0}")]
    Internal(String),
}

/// Scope associated with an autonomy signal.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SignalScope {
    /// Global AGI-wide signal.
    Global,
    /// Domain specific (e.g., finance, infra).
    Domain(String),
    /// Localized subsystem.
    Local(String),
}

/// Real-time telemetry emitted by modules, planners, or sensors.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutonomySignal {
    /// Unique identifier.
    pub id: Uuid,
    /// Timestamp for ordering.
    pub timestamp: DateTime<Utc>,
    /// Scope for routing.
    pub scope: SignalScope,
    /// Metric key/value pairs.
    pub metrics: IndexMap<String, f64>,
    /// Arbitrary tags.
    pub tags: IndexMap<String, String>,
    /// Human readable context.
    pub narrative: String,
}

impl AutonomySignal {
    /// Creates a new signal with default metadata.
    #[must_use]
    pub fn new(scope: SignalScope, narrative: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            scope,
            metrics: IndexMap::new(),
            tags: IndexMap::new(),
            narrative: narrative.into(),
        }
    }

    /// Adds or replaces a metric value.
    #[must_use]
    pub fn with_metric(mut self, key: impl Into<String>, value: f64) -> Self {
        self.metrics.insert(key.into(), value);
        self
    }

    /// Adds a tag for downstream filtering.
    #[must_use]
    pub fn with_tag(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.tags.insert(key.into(), value.into());
        self
    }

    /// Looks up a metric.
    #[must_use]
    pub fn metric(&self, key: &str) -> Option<f64> {
        self.metrics.get(key).copied()
    }
}

/// Directive priority class.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DirectivePriority {
    /// Routine activities.
    Routine,
    /// Elevated urgency.
    Elevated,
    /// Highest urgency.
    Critical,
}

impl fmt::Display for DirectivePriority {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Routine => write!(f, "routine"),
            Self::Elevated => write!(f, "elevated"),
            Self::Critical => write!(f, "critical"),
        }
    }
}

/// Target selection for directives.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ModuleTarget {
    /// Broadcast to every module.
    All,
    /// Single module identifier.
    Module(ModuleId),
    /// All modules of a specific kind.
    Kind(ModuleKind),
}

/// Taxonomy describing module responsibilities.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ModuleKind {
    /// Strategic planning entities.
    Planner,
    /// Low-level executors.
    Executor,
    /// Telemetry or sensing providers.
    Sensor,
    /// Long-term memory systems.
    Memory,
    /// Self-healing or resilience components.
    SelfHealing,
    /// Custom domain-specific module.
    Custom(String),
}

/// Operator instructions delivered to modules.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ControlDirective {
    /// Directive identifier.
    pub id: Uuid,
    /// Creation timestamp.
    pub issued_at: DateTime<Utc>,
    /// Target selection.
    pub target: ModuleTarget,
    /// Priority level.
    pub priority: DirectivePriority,
    /// Natural language instructions.
    pub instructions: String,
    /// Time to live for the directive.
    pub ttl: Duration,
    /// Additional metadata for automation.
    pub metadata: IndexMap<String, serde_json::Value>,
}

impl ControlDirective {
    /// Creates a new directive with routine priority.
    #[must_use]
    pub fn new(target: ModuleTarget, instructions: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            issued_at: Utc::now(),
            target,
            priority: DirectivePriority::Routine,
            instructions: instructions.into(),
            ttl: Duration::minutes(30),
            metadata: IndexMap::new(),
        }
    }

    /// Updates the priority.
    #[must_use]
    pub fn with_priority(mut self, priority: DirectivePriority) -> Self {
        self.priority = priority;
        self
    }

    /// Adds metadata.
    #[must_use]
    pub fn with_metadata(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.metadata.insert(key.into(), value);
        self
    }
}

/// Declares a module with capacity and health metadata.
#[derive(Debug, Clone)]
pub struct ModuleSpec {
    /// Unique identifier.
    pub id: ModuleId,
    /// Friendly name.
    pub name: String,
    /// Module kind.
    pub kind: ModuleKind,
    /// Relative capacity.
    pub capacity: u32,
    /// Health score between 0 and 1.
    pub health: f32,
}

impl ModuleSpec {
    /// Creates a new module spec.
    #[must_use]
    pub fn new(name: impl Into<String>, kind: ModuleKind) -> Self {
        Self {
            id: Uuid::new_v4(),
            name: name.into(),
            kind,
            capacity: 100,
            health: 0.9,
        }
    }
}

/// Registry tracking all modules known to the autonomy kernel.
#[derive(Debug, Default, Clone)]
pub struct ModuleRegistry {
    inner: Arc<RwLock<IndexMap<ModuleId, ModuleSpec>>>,
}

impl ModuleRegistry {
    /// Registers or replaces a module spec.
    pub fn upsert(&self, spec: ModuleSpec) {
        self.inner.write().insert(spec.id, spec);
    }

    /// Returns the number of registered modules.
    #[must_use]
    pub fn len(&self) -> usize {
        self.inner.read().len()
    }

    /// Snapshot of all specs.
    #[must_use]
    pub fn snapshot(&self) -> Vec<ModuleSpec> {
        self.inner.read().values().cloned().collect()
    }

    /// Fetches a specific module.
    pub fn get(&self, id: &ModuleId) -> Result<ModuleSpec, AutonomyError> {
        self.inner
            .read()
            .get(id)
            .cloned()
            .ok_or_else(|| AutonomyError::ModuleNotFound(id.to_string()))
    }

    /// Returns the healthiest module for the given kind.
    pub fn best_of_kind(&self, kind: &ModuleKind) -> Result<ModuleSpec, AutonomyError> {
        self.inner
            .read()
            .values()
            .filter(|spec| &spec.kind == kind)
            .max_by(|a, b| {
                a.health
                    .partial_cmp(&b.health)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .cloned()
            .ok_or_else(|| AutonomyError::MissingKind(kind.clone()))
    }
}

/// Aggregate describing the current module + neuron state.
#[derive(Debug, Clone)]
pub struct ModulePulse {
    /// Module metadata.
    pub spec: ModuleSpec,
    /// Normalized load between 0 and 1.
    pub load: f32,
    /// Neuron-level commentary.
    pub neuron_pulses: Vec<NeuronPulse>,
}

/// Issues directives while keeping modules aligned with signals.
#[derive(Debug, Clone)]
pub struct ModuleBroker {
    registry: ModuleRegistry,
    improvisor: Arc<Mutex<ImprovisationEngine>>,
    smoother: Arc<Mutex<SignalSmoother>>,
    neurons: Arc<NeuronGraph>,
}

impl ModuleBroker {
    /// Creates a new broker.
    #[must_use]
    pub fn new(registry: ModuleRegistry) -> Self {
        Self {
            registry,
            improvisor: Arc::new(Mutex::new(ImprovisationEngine::default())),
            smoother: Arc::new(Mutex::new(SignalSmoother::new(8))),
            neurons: Arc::new(NeuronGraph::default()),
        }
    }

    /// Returns the underlying registry.
    #[must_use]
    pub fn registry(&self) -> ModuleRegistry {
        self.registry.clone()
    }

    /// Processes a signal and returns a pulse for the healthiest planner.
    pub fn evaluate_signal(&self, signal: &AutonomySignal) -> Result<ModulePulse, AutonomyError> {
        let planner = self.registry.best_of_kind(&ModuleKind::Planner)?;
        let normalized = normalize_scores(&signal.metrics);
        let load = normalized.get("load").copied().unwrap_or(0.3) as f32;
        let mut smoother = self.smoother.lock();
        let smoothed = smoother.push(load.into());
        let pulses = self.neurons.pulse(&normalized);

        Ok(ModulePulse {
            spec: planner,
            load: smoothed as f32,
            neuron_pulses: pulses,
        })
    }

    /// Issues a deterministic directive with auto-generated instructions.
    #[must_use]
    pub fn issue_directive(
        &self,
        kind: ModuleKind,
        priority: DirectivePriority,
        description: impl Into<String>,
    ) -> ControlDirective {
        let instructions = format!(
            "[DIRECTIVE::{kind:?}] {} | token={}",
            description.into(),
            Self::token()
        );
        ControlDirective::new(ModuleTarget::Kind(kind), instructions).with_priority(priority)
    }

    /// Emits an improvisation hint that can be executed later.
    #[must_use]
    pub fn hint(
        &self,
        kind: ModuleKind,
        label: impl Into<String>,
        aggressiveness: f32,
    ) -> ImprovisationHint {
        ImprovisationHint::new(kind, label, aggressiveness)
    }

    /// Generates an improvisational directive immediately.
    #[must_use]
    pub fn improvise(&self, hint: &ImprovisationHint) -> ControlDirective {
        self.improvisor.lock().propose(hint)
    }

    fn token() -> String {
        thread_rng()
            .sample_iter(Alphanumeric)
            .take(8)
            .map(char::from)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_best_of_kind() {
        let registry = ModuleRegistry::default();
        registry.upsert(ModuleSpec {
            id: Uuid::new_v4(),
            name: "planner-A".into(),
            kind: ModuleKind::Planner,
            capacity: 100,
            health: 0.8,
        });
        registry.upsert(ModuleSpec {
            id: Uuid::new_v4(),
            name: "planner-B".into(),
            kind: ModuleKind::Planner,
            capacity: 100,
            health: 0.9,
        });

        let best = registry.best_of_kind(&ModuleKind::Planner).unwrap();
        assert_eq!(best.name, "planner-B");
    }

    #[test]
    fn broker_generates_directives() {
        let registry = ModuleRegistry::default();
        registry.upsert(ModuleSpec::new("planner", ModuleKind::Planner));
        let broker = ModuleBroker::new(registry);
        let directive = broker.issue_directive(
            ModuleKind::Executor,
            DirectivePriority::Critical,
            "scale compute",
        );
        assert_eq!(directive.priority, DirectivePriority::Critical);
    }
}
