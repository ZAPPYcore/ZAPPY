use std::collections::VecDeque;

use anyhow::Result;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use crate::{
    model::{AnomalyEvent, RegionSnapshot, WorldModel, WorldState},
    telemetry::WorldTelemetry,
};

/// Job describing assimilation of new signals into the world model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssimilationJob {
    /// Source batch id.
    pub batch_id: Uuid,
    /// Region metrics keyed by region id.
    pub region_metrics: IndexMap<String, Value>,
}

/// Engine that applies assimilation jobs and tracks anomalies.
pub struct AssimilationEngine {
    model: WorldModel,
    telemetry: Option<WorldTelemetry>,
    history: VecDeque<WorldState>,
    threshold: f32,
}

impl AssimilationEngine {
    /// Creates a default engine.
    #[must_use]
    pub fn new(model: WorldModel, telemetry: Option<WorldTelemetry>) -> Self {
        Self {
            model,
            telemetry,
            history: VecDeque::with_capacity(16),
            threshold: 0.85,
        }
    }

    /// Processes a job and returns updated world state.
    pub fn assimilate(&mut self, job: AssimilationJob) -> Result<WorldState> {
        for (region, metrics_value) in &job.region_metrics {
            let metrics_map = extract_metrics(metrics_value)?;
            let snapshot = RegionSnapshot::from_metrics(region.clone(), metrics_map.clone());
            let delta = self.model.ingest(snapshot);
            let severity = metrics_map.get("load").copied().unwrap_or_default();
            if severity >= self.threshold {
                self.model.anomaly(AnomalyEvent::new(
                    region,
                    severity,
                    serde_json::json!({ "delta": delta }),
                ));
            }
        }
        let state = self.model.snapshot();
        self.history.push_back(state.clone());
        if self.history.len() > 16 {
            self.history.pop_front();
        }
        if let Some(tel) = &self.telemetry {
            let _ = tel.log(
                shared_logging::LogLevel::Info,
                "world.assimilation.completed",
                serde_json::json!({ "batch_id": job.batch_id, "regions": job.region_metrics.len() }),
            );
        }
        Ok(state)
    }

    /// Returns last known state.
    #[must_use]
    pub fn last_state(&self) -> Option<&WorldState> {
        self.history.back()
    }
}

fn extract_metrics(value: &Value) -> Result<IndexMap<String, f32>> {
    let mut map = IndexMap::new();
    if let Some(obj) = value.as_object() {
        for (key, val) in obj {
            map.insert(key.clone(), val.as_f64().unwrap_or_default() as f32);
        }
        Ok(map)
    } else {
        anyhow::bail!("metrics must be an object");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn engine_records_anomaly() {
        let mut engine = AssimilationEngine::new(WorldModel::new(), None);
        let mut regions = IndexMap::new();
        regions.insert("alpha".into(), json!({ "load": 0.9, "demand": 0.7 }));
        let job = AssimilationJob {
            batch_id: Uuid::new_v4(),
            region_metrics: regions,
        };
        let state = engine.assimilate(job).unwrap();
        assert!(!state.anomalies.is_empty());
    }
}
