use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

/// Snapshot describing a specific region in the simulated world.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegionSnapshot {
    /// Region identifier.
    pub region_id: String,
    /// Primary metrics (cpu load, demand, etc).
    pub metrics: IndexMap<String, f32>,
    /// Last update timestamp (epoch seconds).
    pub updated_at: i64,
}

impl RegionSnapshot {
    /// Creates a snapshot from metrics.
    #[must_use]
    pub fn from_metrics(region_id: impl Into<String>, metrics: IndexMap<String, f32>) -> Self {
        Self {
            region_id: region_id.into(),
            metrics,
            updated_at: chrono::Utc::now().timestamp(),
        }
    }
}

/// Event representing anomalies discovered in the world.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnomalyEvent {
    /// Event id.
    pub id: Uuid,
    /// Region impacted.
    pub region_id: String,
    /// Severity 0-1.
    pub severity: f32,
    /// Diagnostics metadata.
    pub metadata: Value,
}

impl AnomalyEvent {
    /// Creates a new anomaly event.
    #[must_use]
    pub fn new(region_id: impl Into<String>, severity: f32, metadata: Value) -> Self {
        Self {
            id: Uuid::new_v4(),
            region_id: region_id.into(),
            severity,
            metadata,
        }
    }
}

/// Full world state with multiple regions and anomaly timeline.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WorldState {
    /// Region snapshots.
    pub regions: IndexMap<String, RegionSnapshot>,
    /// Anomaly events timeline.
    pub anomalies: Vec<AnomalyEvent>,
}

impl WorldState {
    /// Applies a region snapshot.
    pub fn apply_snapshot(&mut self, snapshot: RegionSnapshot) {
        self.regions.insert(snapshot.region_id.clone(), snapshot);
    }

    /// Records an anomaly, keeping last 100.
    pub fn record_anomaly(&mut self, event: AnomalyEvent) {
        self.anomalies.push(event);
        if self.anomalies.len() > 100 {
            self.anomalies.remove(0);
        }
    }

    /// Returns highest severity anomaly.
    #[must_use]
    pub fn highest_severity(&self) -> Option<&AnomalyEvent> {
        self.anomalies
            .iter()
            .max_by(|a, b| a.severity.partial_cmp(&b.severity).unwrap())
    }
}

/// World model persists state and emits derived metrics.
#[derive(Debug, Default)]
pub struct WorldModel {
    state: WorldState,
}

impl WorldModel {
    /// Creates a new model.
    #[must_use]
    pub fn new() -> Self {
        Self {
            state: WorldState::default(),
        }
    }

    /// Ingests a region snapshot, returns delta metrics.
    pub fn ingest(&mut self, snapshot: RegionSnapshot) -> IndexMap<String, f32> {
        let prev = self.state.regions.get(&snapshot.region_id).cloned();
        self.state.apply_snapshot(snapshot.clone());
        match prev {
            Some(prev_snapshot) => diff_metrics(&prev_snapshot.metrics, &snapshot.metrics),
            None => snapshot.metrics,
        }
    }

    /// Adds anomaly event.
    pub fn anomaly(&mut self, event: AnomalyEvent) {
        self.state.record_anomaly(event);
    }

    /// Returns a copy of current state.
    #[must_use]
    pub fn snapshot(&self) -> WorldState {
        self.state.clone()
    }
}

fn diff_metrics(
    prev: &IndexMap<String, f32>,
    current: &IndexMap<String, f32>,
) -> IndexMap<String, f32> {
    let mut delta = IndexMap::new();
    for (key, value) in current {
        let prev_value = prev.get(key).copied().unwrap_or_default();
        delta.insert(key.clone(), value - prev_value);
    }
    delta
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn world_model_tracks_regions() {
        let mut model = WorldModel::new();
        let mut metrics = IndexMap::new();
        metrics.insert("load".into(), 0.6);
        model.ingest(RegionSnapshot::from_metrics("us-east", metrics.clone()));

        metrics.insert("load".into(), 0.8);
        let delta = model.ingest(RegionSnapshot::from_metrics("us-east", metrics));
        assert!((delta.get("load").copied().unwrap_or_default() - 0.2).abs() < 1e-6);
        assert!(model.snapshot().regions.contains_key("us-east"));
    }
}
