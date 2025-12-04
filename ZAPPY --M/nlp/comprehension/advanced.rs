use anyhow::Result;
use serde_json::json;

use crate::{
    comprehension::{
        comprehension::ComprehensionEngine, ComprehensionRequest, ComprehensionResult,
    },
    telemetry::NlpTelemetry,
};

/// Bundle describing a request and its origin.
#[derive(Clone)]
pub struct EvidenceBundle {
    /// Original request.
    pub request: ComprehensionRequest,
    /// Correlation id for tracing.
    pub correlation_id: String,
}

/// Controller orchestrating async comprehension.
pub struct AdvancedComprehensionController {
    engine: ComprehensionEngine,
    telemetry: Option<NlpTelemetry>,
}

impl AdvancedComprehensionController {
    /// Creates a new controller.
    #[must_use]
    pub fn new(engine: ComprehensionEngine, telemetry: Option<NlpTelemetry>) -> Self {
        Self { engine, telemetry }
    }

    /// Processes a batch concurrently.
    pub async fn process_batch(
        &self,
        bundles: Vec<EvidenceBundle>,
    ) -> Result<Vec<ComprehensionResult>> {
        self.log("nlp.comprehension.batch_start", bundles.len());
        let tasks = bundles.into_iter().map(|bundle| {
            let engine = self.engine.clone();
            let tel = self.telemetry.clone();
            tokio::task::spawn_blocking(move || {
                if let Some(t) = tel {
                    let _ = t.log(
                        shared_logging::LogLevel::Debug,
                        "nlp.comprehension.request",
                        json!({ "correlation_id": bundle.correlation_id }),
                    );
                }
                engine.analyze(&bundle.request)
            })
        });
        let mut results = Vec::new();
        for task in tasks {
            results.push(task.await?);
        }
        self.log("nlp.comprehension.batch_complete", results.len());
        Ok(results)
    }

    fn log(&self, message: &str, count: usize) {
        if let Some(tel) = &self.telemetry {
            let _ = tel.log(
                shared_logging::LogLevel::Info,
                message,
                json!({ "count": count }),
            );
        }
    }
}
