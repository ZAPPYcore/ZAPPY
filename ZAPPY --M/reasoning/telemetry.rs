use std::{fmt, path::PathBuf, sync::Arc};

use anyhow::Result;
use serde_json::Value;
use shared_event_bus::{EventPublisher, EventRecord};
use shared_logging::{JsonLogger, LogLevel, LogRecord};
use tokio::runtime::{Handle, Runtime};
use uuid::Uuid;

/// Builder for reasoning telemetry sinks.
pub struct ReasoningTelemetryBuilder {
    module: String,
    log_path: Option<PathBuf>,
    event_publisher: Option<Arc<dyn EventPublisher>>,
}

impl ReasoningTelemetryBuilder {
    /// Creates the builder.
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

    /// Builds the telemetry handle.
    pub fn build(self) -> Result<ReasoningTelemetry> {
        ReasoningTelemetry::new(self.module, self.log_path, self.event_publisher)
    }
}

/// Telemetry handle shared across reasoning components.
#[derive(Clone)]
pub struct ReasoningTelemetry {
    inner: Arc<TelemetryInner>,
}

impl fmt::Debug for ReasoningTelemetry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ReasoningTelemetry")
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
    runtime: Runtime,
    publisher: Arc<dyn EventPublisher>,
}

impl EventHandle {
    fn new(publisher: Arc<dyn EventPublisher>) -> Result<Self> {
        Ok(Self {
            runtime: Runtime::new()?,
            publisher,
        })
    }

    fn publish(&self, record: EventRecord) -> Result<()> {
        if let Ok(handle) = Handle::try_current() {
            let publisher = Arc::clone(&self.publisher);
            handle.spawn(async move {
                if let Err(err) = publisher.publish(record).await {
                    eprintln!("telemetry event publish failed: {err:?}");
                }
            });
            Ok(())
        } else {
            self.runtime.block_on(self.publisher.publish(record))
        }
    }
}

impl ReasoningTelemetry {
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
    pub fn builder(module: impl Into<String>) -> ReasoningTelemetryBuilder {
        ReasoningTelemetryBuilder::new(module)
    }

    /// Logs structured metadata.
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

    /// Emits an event on the bus.
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use shared_event_bus::MemoryEventBus;
    use tempfile::tempdir;

    #[test]
    fn telemetry_writes_log_and_event() {
        let tmp = tempdir().unwrap();
        let path = tmp.path().join("reasoning.log");
        let bus = Arc::new(MemoryEventBus::new(16));
        let telemetry = ReasoningTelemetry::builder("reasoning")
            .log_path(&path)
            .event_publisher(bus.clone())
            .build()
            .unwrap();
        telemetry
            .log(LogLevel::Info, "reasoning.start", json!({ "signals": 3 }))
            .unwrap();
        telemetry
            .event("reasoning.completed", json!({ "hypotheses": 2 }))
            .unwrap();
        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("reasoning.start"));
        assert_eq!(bus.snapshot().len(), 1);
    }
}
