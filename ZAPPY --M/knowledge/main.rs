//! High-level orchestration for the knowledge pipeline.

use anyhow::Result;
use std::sync::Arc;

use crate::{
    editor::editor::{EditOperation, KnowledgeEditor},
    receiver::{KnowledgeArtifact, KnowledgeReceiver},
    saver::{KnowledgeRecord, KnowledgeStore},
    security::{KnowledgeGuard, SecurityPolicy},
    seeker::{KnowledgeQuery, KnowledgeSeeker},
    telemetry::KnowledgeTelemetry,
    websearcher::{LoopbackWebClient, WebSearcher},
};
use serde_json::{json, to_string_pretty};
use shared_logging::LogLevel;
use zappy_learning::pipeline::PipelineEnvelope;

/// Runtime for ingest-search-edit workflows.
#[derive(Debug, Clone)]
pub struct KnowledgeRuntime {
    store: KnowledgeStore,
    receiver: KnowledgeReceiver,
    seeker: KnowledgeSeeker,
    editor: KnowledgeEditor,
    searcher: WebSearcher,
    telemetry: Option<KnowledgeTelemetry>,
}

impl KnowledgeRuntime {
    /// Bootstraps the runtime with default components.
    #[must_use]
    pub fn bootstrap() -> Self {
        let store = KnowledgeStore::default();
        let guard = KnowledgeGuard::new(SecurityPolicy::default());
        let receiver = KnowledgeReceiver::new(store.clone(), guard);
        let seeker = KnowledgeSeeker::new(store.clone());
        let editor = KnowledgeEditor::new(store.clone());
        let searcher = WebSearcher::new(Arc::new(LoopbackWebClient));

        Self {
            store,
            receiver,
            seeker,
            editor,
            searcher,
            telemetry: None,
        }
    }

    /// Ingests an artifact via the receiver.
    pub fn ingest(&self, artifact: KnowledgeArtifact) -> Result<KnowledgeRecord> {
        if let Some(tel) = &self.telemetry {
            let _ = tel.log(
                LogLevel::Info,
                "knowledge.ingest.start",
                json!({ "source": artifact.source.clone(), "title": artifact.title.clone() }),
            );
        }
        let record = self.receiver.receive(artifact)?;
        if let Some(tel) = &self.telemetry {
            let _ = tel.log(
                LogLevel::Info,
                "knowledge.ingest.complete",
                json!({ "record_id": record.id, "length": record.body.len() }),
            );
            let _ = tel.event(
                "knowledge.record.ingested",
                json!({ "record_id": record.id }),
            );
        }
        Ok(record)
    }

    /// Runs a search over the local store.
    pub fn search(&self, query: KnowledgeQuery) -> Vec<crate::seeker::KnowledgeSnippet> {
        let text = query.text.clone();
        let domain = query.domain.clone();
        if let Some(tel) = &self.telemetry {
            let _ = tel.log(
                LogLevel::Debug,
                "knowledge.search.start",
                json!({ "query": text.clone(), "domain": domain }),
            );
        }
        let results = self.seeker.search(query);
        if let Some(tel) = &self.telemetry {
            let _ = tel.log(
                LogLevel::Debug,
                "knowledge.search.complete",
                json!({ "query": text, "results": results.len() }),
            );
        }
        results
    }

    /// Applies an edit to a record.
    pub fn edit(&self, operation: EditOperation) -> Result<KnowledgeRecord> {
        if let Some(tel) = &self.telemetry {
            let _ = tel.log(
                LogLevel::Info,
                "knowledge.edit.start",
                json!({ "record_id": operation.record_id }),
            );
        }
        let record = self.editor.apply(operation)?;
        if let Some(tel) = &self.telemetry {
            let _ = tel.log(
                LogLevel::Info,
                "knowledge.edit.complete",
                json!({ "record_id": record.id }),
            );
            let _ = tel.event("knowledge.record.edited", json!({ "record_id": record.id }));
        }
        Ok(record)
    }

    /// Executes an external web search.
    pub async fn search_web(&self, query: &str) -> Result<Vec<crate::websearcher::SearchResult>> {
        if let Some(tel) = &self.telemetry {
            let _ = tel.log(
                LogLevel::Debug,
                "knowledge.web_search.start",
                json!({ "query": query }),
            );
        }
        let results = self.searcher.search(query).await?;
        if let Some(tel) = &self.telemetry {
            let _ = tel.log(
                LogLevel::Debug,
                "knowledge.web_search.complete",
                json!({ "query": query, "results": results.len() }),
            );
        }
        Ok(results)
    }

    /// Runs a live web search, ingests fresh artifacts, and returns newly created records.
    pub async fn enrich_from_web(&self, query: &str) -> Result<Vec<KnowledgeRecord>> {
        let results = self.search_web(query).await?;
        let mut ingested = Vec::new();
        for result in results {
            let external_ref = format!("web::{}", result.url);
            if self.store.contains_external_ref(&external_ref) {
                continue;
            }
            let mut artifact = KnowledgeArtifact::new(
                &result.url,
                &result.title,
                format!("{}\n\nSource: {}", result.summary, result.url),
            );
            artifact.external_id = external_ref;
            artifact.category = Some("web".into());
            artifact.collected_at = result.fetched_at;
            let record = self.ingest(artifact)?;
            ingested.push(record);
        }
        Ok(ingested)
    }

    /// Attaches telemetry sinks.
    #[must_use]
    pub fn with_telemetry(mut self, telemetry: KnowledgeTelemetry) -> Self {
        self.telemetry = Some(telemetry);
        self
    }

    /// Sets telemetry for the runtime.
    pub fn set_telemetry(&mut self, telemetry: KnowledgeTelemetry) {
        self.telemetry = Some(telemetry);
    }

    /// Accesses the telemetry handle.
    #[must_use]
    pub fn telemetry(&self) -> Option<&KnowledgeTelemetry> {
        self.telemetry.as_ref()
    }

    /// Returns the underlying store (read-only clone).
    #[must_use]
    pub fn store(&self) -> KnowledgeStore {
        self.store.clone()
    }

    /// Ingests an experience hub envelope as a knowledge record, skipping duplicates.
    pub fn ingest_experience(
        &self,
        envelope: &PipelineEnvelope,
    ) -> Result<Option<KnowledgeRecord>> {
        let experience_id = envelope.id.to_string();
        if self.store.contains_external_ref(&experience_id) {
            return Ok(None);
        }
        let artifact = experience_to_artifact(envelope, experience_id)?;
        self.ingest(artifact).map(Some)
    }
}

fn experience_to_artifact(
    envelope: &PipelineEnvelope,
    external_ref: String,
) -> Result<KnowledgeArtifact> {
    let title = format!("{}::{}", envelope.module, envelope.signal);
    let body = to_string_pretty(&envelope.payload)?;
    let mut artifact = KnowledgeArtifact::new(&envelope.module, title, body);
    artifact.external_id = external_ref;
    artifact.source = envelope.module.clone();
    artifact.category = Some(envelope.signal.clone());
    artifact.collected_at = envelope.timestamp;
    Ok(artifact)
}

#[cfg(test)]
mod tests {
    use super::*;
    use zappy_learning::pipeline::PipelineEnvelope;

    #[test]
    fn runtime_ingests_and_searches() {
        let runtime = KnowledgeRuntime::bootstrap();
        let record = runtime
            .ingest(KnowledgeArtifact::new(
                "web",
                "Knowledge Ops",
                "Detailed description of operations pipeline",
            ))
            .unwrap();
        let results = runtime.search(KnowledgeQuery::new("operations"));
        assert!(!results.is_empty());

        let updated = runtime
            .edit(EditOperation {
                record_id: record.id,
                new_body: "Updated operations pipeline description".into(),
                rationale: "Added clarity".into(),
            })
            .unwrap();
        assert!(updated.body.contains("Updated"));
    }

    #[test]
    fn runtime_ingests_experience_once() {
        let runtime = KnowledgeRuntime::bootstrap();
        let envelope = PipelineEnvelope {
            id: uuid::Uuid::new_v4(),
            module: "planning".into(),
            signal: "plan.generated".into(),
            payload: serde_json::json!({ "objective": "stabilize" }),
            timestamp: chrono::Utc::now(),
        };
        let first = runtime.ingest_experience(&envelope).unwrap();
        assert!(first.is_some());
        let second = runtime.ingest_experience(&envelope).unwrap();
        assert!(second.is_none());
    }
}
