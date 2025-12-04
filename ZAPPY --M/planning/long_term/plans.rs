use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::long_term::{PlanScore, StrategicPlan};

/// Metadata stored for historical analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanRecord {
    /// Plan id.
    pub plan_id: Uuid,
    /// Objective id.
    pub objective_id: Uuid,
    /// ROI logged.
    pub roi: f32,
    /// Risk logged.
    pub risk: f32,
    /// Timestamp when archived.
    pub archived_at: DateTime<Utc>,
}

/// In-memory archive of past plans.
#[derive(Debug, Default)]
pub struct PlanArchive {
    records: Vec<PlanRecord>,
    capacity: usize,
}

impl PlanArchive {
    /// Creates archive with capacity.
    #[must_use]
    pub fn new(capacity: usize) -> Self {
        Self {
            records: Vec::with_capacity(capacity),
            capacity,
        }
    }

    /// Pushes plan metadata into archive.
    pub fn push(&mut self, plan: &StrategicPlan, score: &PlanScore) {
        if self.records.len() >= self.capacity {
            self.records.remove(0);
        }
        self.records.push(PlanRecord {
            plan_id: Uuid::new_v4(),
            objective_id: plan.objective.id,
            roi: score.roi,
            risk: score.risk,
            archived_at: Utc::now(),
        });
    }

    /// Returns last N records for an objective.
    #[must_use]
    pub fn history_for(&self, objective_id: Uuid, limit: usize) -> Vec<PlanRecord> {
        self.records
            .iter()
            .rev()
            .filter(|record| record.objective_id == objective_id)
            .take(limit)
            .cloned()
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::long_term::{PlanPhase, StrategicObjective};
    use indexmap::indexmap;

    #[test]
    fn archive_tracks_history() {
        let mut archive = PlanArchive::new(4);
        let plan = StrategicPlan {
            objective: StrategicObjective::new("test", 50, 12),
            phases: vec![PlanPhase {
                label: "p1".into(),
                start_week: 0,
                end_week: 4,
                resources: indexmap! {},
                risk_multiplier: 1.0,
            }],
            risk_score: 0.4,
            expected_roi: 0.6,
            generated_at: Utc::now(),
        };
        let score = PlanScore {
            roi: 0.6,
            risk: 0.3,
            confidence: 0.7,
        };
        archive.push(&plan, &score);
        assert_eq!(archive.history_for(plan.objective.id, 1).len(), 1);
    }
}
