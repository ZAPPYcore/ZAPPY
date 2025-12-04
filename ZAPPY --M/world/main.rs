use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use indexmap::IndexMap;
use serde_json::{json, Value};
use uuid::Uuid;

use crate::{
    advanced::{AdvancedController, TrainingArtifact, TrainingConfig},
    feature_store::FeatureStore,
    feed_config::FeedsDocument,
    infoseeker::{InfoSeeker, InfoSeekerBuilder, InfoSignal},
    learning::{AssimilationEngine, AssimilationJob},
    model::{WorldModel, WorldState},
    telemetry::WorldTelemetry,
};

/// Runtime orchestrating world info seeker, learning, and advanced controller.
pub struct WorldRuntime {
    telemetry: Option<WorldTelemetry>,
    seeker: InfoSeeker,
    assimilation: AssimilationEngine,
    advanced: AdvancedController,
    feature_store: FeatureStore,
}

impl WorldRuntime {
    /// Returns a builder.
    #[must_use]
    pub fn builder() -> WorldRuntimeBuilder {
        WorldRuntimeBuilder::default()
    }

    /// Pulls latest signals via seeker and assimilates them.
    pub async fn refresh(&mut self) -> Result<WorldState> {
        let signals = self.seeker.collect().await?;
        if signals.is_empty() {
            bail!("no signals collected from any provider");
        }
        let batch_id = Uuid::new_v4();
        self.feature_store.persist_signals(&batch_id, &signals)?;
        let job = AssimilationJob {
            batch_id,
            region_metrics: aggregate_signals(&signals),
        };
        self.feature_store.persist_job(&job)?;
        self.ingest(job)
    }

    /// Ingests a prepared assimilation job.
    pub fn ingest(&mut self, job: AssimilationJob) -> Result<WorldState> {
        let state = self.assimilation.assimilate(job)?;
        let requires_action = self.advanced.review_state(&state)?;
        if requires_action {
            if let Some(tel) = &self.telemetry {
                let _ = tel.event(
                    "world.alert.triggered",
                    json!({ "anomalies": state.anomalies.len(), "regions": state.regions.len() }),
                );
            }
        }
        Ok(state)
    }

    /// Retrains predictive model.
    pub async fn retrain(&self, config: TrainingConfig) -> Result<TrainingArtifact> {
        self.advanced.retrain(config).await
    }

    /// Returns telemetry handle.
    #[must_use]
    pub fn telemetry(&self) -> Option<&WorldTelemetry> {
        self.telemetry.as_ref()
    }
}

/// Builder for `WorldRuntime`.
pub struct WorldRuntimeBuilder {
    telemetry: Option<WorldTelemetry>,
    baseline: IndexMap<String, f32>,
    seeker: Option<InfoSeeker>,
    feeds_document: Option<FeedsDocument>,
    feature_store: Option<FeatureStore>,
}

impl WorldRuntimeBuilder {
    /// Sets telemetry.
    #[must_use]
    pub fn telemetry(mut self, telemetry: WorldTelemetry) -> Self {
        self.telemetry = Some(telemetry);
        self
    }

    /// Sets baseline metrics.
    #[must_use]
    pub fn baseline(mut self, metrics: IndexMap<String, f32>) -> Self {
        self.baseline = metrics;
        self
    }

    /// Overrides info seeker.
    #[must_use]
    pub fn seeker(mut self, seeker: InfoSeeker) -> Self {
        self.seeker = Some(seeker);
        self
    }

    /// Loads seeker configuration from a feeds file.
    pub fn feeds_config_path(mut self, path: impl AsRef<Path>) -> Result<Self> {
        let document = FeedsDocument::load(path).context("loading world feeds configuration")?;
        self.feeds_document = Some(document);
        Ok(self)
    }

    /// Injects feature store instance.
    #[must_use]
    pub fn feature_store(mut self, store: FeatureStore) -> Self {
        self.feature_store = Some(store);
        self
    }

    /// Opens a feature store at the provided path.
    pub fn feature_store_path(mut self, path: impl Into<PathBuf>) -> Result<Self> {
        let store = FeatureStore::open(path)?;
        self.feature_store = Some(store);
        Ok(self)
    }

    /// Builds runtime.
    pub fn build(self) -> Result<WorldRuntime> {
        let telemetry = self.telemetry;
        let seeker = if let Some(seeker) = self.seeker {
            seeker
        } else if let Some(doc) = self.feeds_document {
            InfoSeeker::from_feeds_document(&doc, telemetry.clone())?
        } else {
            InfoSeekerBuilder::default()
                .telemetry_opt(telemetry.clone())
                .build()
        };
        let assimilation = AssimilationEngine::new(WorldModel::new(), telemetry.clone());
        let advanced = AdvancedController::new(self.baseline, telemetry.clone());
        let feature_store = self.feature_store.unwrap_or_else(FeatureStore::disabled);
        Ok(WorldRuntime {
            telemetry,
            seeker,
            assimilation,
            advanced,
            feature_store,
        })
    }
}

impl Default for WorldRuntimeBuilder {
    fn default() -> Self {
        let mut baseline = IndexMap::new();
        baseline.insert("load".into(), 0.5);
        baseline.insert("demand".into(), 0.4);
        Self {
            telemetry: None,
            baseline,
            seeker: None,
            feeds_document: None,
            feature_store: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn runtime_refreshes_state() {
        let mut runtime = WorldRuntime::builder().build().unwrap();
        let state = runtime.refresh().await.unwrap();
        assert!(!state.regions.is_empty());
    }

    #[tokio::test]
    async fn runtime_ingests_manual_job() {
        let mut runtime = WorldRuntime::builder().build().unwrap();
        let mut region_metrics = IndexMap::new();
        region_metrics.insert("alpha".into(), json!({ "load": 0.9 }));
        let state = runtime
            .ingest(AssimilationJob {
                batch_id: Uuid::new_v4(),
                region_metrics,
            })
            .unwrap();
        assert!(state.regions.contains_key("alpha"));
    }
}

fn aggregate_signals(signals: &[InfoSignal]) -> IndexMap<String, Value> {
    let mut region_metrics = IndexMap::new();
    for signal in signals {
        region_metrics.insert(signal.region_id.clone(), signal.metrics.clone());
    }
    region_metrics
}
