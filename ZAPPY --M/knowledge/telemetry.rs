use std::{fmt, path::PathBuf, sync::Arc};

use anyhow::Result;
use serde_json::Value;
use shared_event_bus::{EventPublisher, EventRecord};
use shared_logging::{JsonLogger, LogLevel, LogRecord};
use tokio::runtime::{Handle, Runtime};
use uuid::Uuid;

/// Builder configuring telemetry for knowledge ingestion/curation.
pub struct KnowledgeTelemetryBuilder {
    module: String,
    log_path: Option<PathBuf>,
    event_publisher: Option<Arc<dyn EventPublisher>>,
}

impl KnowledgeTelemetryBuilder {
    /// Creates a new builder.
    #[must_use]
    pub fn new(module: impl Into<String>) -> Self {
        Self {
            module: module.into(),
            log_path: None,
            event_publisher: None,
        }
    }

    /// Sets the JSON log path.
    #[must_use]
    pub fn log_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.log_path = Some(path.into());
        self
    }

    /// Assigns the event publisher.
    #[must_use]
    pub fn event_publisher(mut self, publisher: Arc<dyn EventPublisher>) -> Self {
        self.event_publisher = Some(publisher);
        self
    }

    /// Finalizes the builder.
    pub fn build(self) -> Result<KnowledgeTelemetry> {
        KnowledgeTelemetry::new(self.module, self.log_path, self.event_publisher)
    }
}

/// Telemetry handle for knowledge workflows.
#[derive(Clone)]
pub struct KnowledgeTelemetry {
    inner: Arc<TelemetryInner>,
}

impl fmt::Debug for KnowledgeTelemetry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("KnowledgeTelemetry")
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

impl KnowledgeTelemetry {
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

    /// Returns a builder for this telemetry helper.
    #[must_use]
    pub fn builder(module: impl Into<String>) -> KnowledgeTelemetryBuilder {
        KnowledgeTelemetryBuilder::new(module)
    }

    /// Logs a structured record.
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

    /// Emits an event entry via the configured bus.
    pub fn event(&self, event_type: &str, payload: Value) -> Result<()> {
        if let Some(handle) = &self.inner.event {
            let record = EventRecord {
                id: format!("evt-{}", Uuid::new_v4()),
                source: self.inner.module.clone(),
                event_type: event_type.into(),
                timestamp: chrono::Utc::now().to_rfc3339(),
                payload,
            };
            handle.publish(record)?;
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
    fn telemetry_logs_and_emits() {
        let dir = tempdir().unwrap();
        let log_path = dir.path().join("knowledge.log");
        let bus = Arc::new(MemoryEventBus::new(8));
        let telemetry = KnowledgeTelemetry::builder("knowledge")
            .log_path(&log_path)
            .event_publisher(bus.clone())
            .build()
            .unwrap();
        telemetry
            .log(
                LogLevel::Info,
                "knowledge.test",
                json!({ "artifact": "sample" }),
            )
            .unwrap();
        telemetry
            .event("knowledge.test", json!({ "records": 1 }))
            .unwrap();
        let content = std::fs::read_to_string(log_path).unwrap();
        assert!(content.contains("knowledge.test"));
        assert_eq!(bus.snapshot().len(), 1);
    }
}
