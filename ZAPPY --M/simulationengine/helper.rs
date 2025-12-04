use std::{fmt, path::PathBuf, sync::Arc};

use anyhow::Result;
use rand::{rngs::SmallRng, Rng, SeedableRng};
use serde_json::Value;
use shared_event_bus::{EventPublisher, EventRecord};
use shared_logging::{JsonLogger, LogLevel, LogRecord};
use tokio::runtime::Runtime;
use uuid::Uuid;

/// Telemetry builder for the simulation engine.
pub struct SimulationTelemetryBuilder {
    module: String,
    log_path: Option<PathBuf>,
    event_publisher: Option<Arc<dyn EventPublisher>>,
}

impl SimulationTelemetryBuilder {
    /// Creates a new builder scoped to a module label.
    #[must_use]
    pub fn new(module: impl Into<String>) -> Self {
        Self {
            module: module.into(),
            log_path: None,
            event_publisher: None,
        }
    }

    /// Sets the log path.
    #[must_use]
    pub fn log_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.log_path = Some(path.into());
        self
    }

    /// Sets the event publisher.
    #[must_use]
    pub fn event_publisher(mut self, publisher: Arc<dyn EventPublisher>) -> Self {
        self.event_publisher = Some(publisher);
        self
    }

    /// Builds telemetry.
    pub fn build(self) -> Result<SimulationTelemetry> {
        SimulationTelemetry::new(self.module, self.log_path, self.event_publisher)
    }
}

/// Telemetry handle shared across simulation components.
#[derive(Clone)]
pub struct SimulationTelemetry {
    inner: Arc<TelemetryInner>,
}

impl fmt::Debug for SimulationTelemetry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SimulationTelemetry")
            .field("module", &self.inner.module)
            .finish()
    }
}

struct TelemetryInner {
    module: String,
    logger: Option<JsonLogger>,
    event: Option<EventHandle>,
}

struct EventHandle {
    publisher: Arc<dyn EventPublisher>,
}

impl EventHandle {
    fn new(publisher: Arc<dyn EventPublisher>) -> Result<Self> {
        Ok(Self { publisher })
    }

    fn publish(&self, record: EventRecord) -> Result<()> {
        if tokio::runtime::Handle::try_current().is_ok() {
            let publisher = Arc::clone(&self.publisher);
            tokio::spawn(async move {
                let _ = publisher.publish(record).await;
            });
            Ok(())
        } else {
            Runtime::new()?.block_on(self.publisher.publish(record))
        }
    }
}

impl SimulationTelemetry {
    fn new(
        module: impl Into<String>,
        log_path: Option<PathBuf>,
        event_publisher: Option<Arc<dyn EventPublisher>>,
    ) -> Result<Self> {
        let logger = if let Some(path) = log_path {
            Some(JsonLogger::new(path)?)
        } else {
            None
        };
        let event = if let Some(publisher) = event_publisher {
            Some(EventHandle::new(publisher)?)
        } else {
            None
        };
        Ok(Self {
            inner: Arc::new(TelemetryInner {
                module: module.into(),
                logger,
                event,
            }),
        })
    }

    /// Returns a builder.
    #[must_use]
    pub fn builder(module: impl Into<String>) -> SimulationTelemetryBuilder {
        SimulationTelemetryBuilder::new(module)
    }

    /// Logs metadata.
    pub fn log(&self, level: LogLevel, message: &str, metadata: Value) -> Result<()> {
        if let Some(logger) = &self.inner.logger {
            let mut record = LogRecord::new(&self.inner.module, level, message);
            if let Some(obj) = metadata.as_object() {
                record.metadata = obj.clone();
            }
            logger.log(&record)?;
        }
        Ok(())
    }

    /// Emits events.
    pub fn event(&self, event_type: &str, payload: Value) -> Result<()> {
        if let Some(handle) = &self.inner.event {
            handle.publish(EventRecord {
                id: format!("evt-{}", Uuid::new_v4()),
                source: self.inner.module.clone(),
                event_type: event_type.into(),
                timestamp: chrono::Utc::now().to_rfc3339(),
                payload,
            })?;
        }
        Ok(())
    }
}

/// Generates a random seed for simulation runs.
#[must_use]
pub fn random_seed() -> u64 {
    rand::thread_rng().gen()
}

/// Returns a reproducible RNG.
#[must_use]
pub fn seeded_rng(seed: u64) -> SmallRng {
    SmallRng::seed_from_u64(seed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use shared_event_bus::MemoryEventBus;
    use tempfile::tempdir;

    #[test]
    fn telemetry_writes_log_and_event() {
        let tmp = tempdir().unwrap();
        let bus = Arc::new(MemoryEventBus::new(4));
        let log_path = tmp.path().join("sim.log");
        let telemetry = SimulationTelemetry::builder("simulation")
            .log_path(&log_path)
            .event_publisher(bus.clone())
            .build()
            .unwrap();
        telemetry
            .log(LogLevel::Info, "simulation.start", json!({ "seed": 1 }))
            .unwrap();
        telemetry
            .event("simulation.completed", json!({ "status": "ok" }))
            .unwrap();
        assert!(std::fs::read_to_string(&log_path)
            .unwrap()
            .contains("simulation.start"));
        assert_eq!(bus.snapshot().len(), 1);
    }
}
