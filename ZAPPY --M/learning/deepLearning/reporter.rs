use serde::{Deserialize, Serialize};

use super::model::ModelSnapshot;

/// Report summarizing a deep learning experiment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DlReport {
    /// Experiment identifier.
    pub experiment: String,
    /// Snapshots recorded.
    pub snapshots: Vec<ModelSnapshot>,
}

impl DlReport {
    /// Returns the best (minimum loss) snapshot.
    #[must_use]
    pub fn best(&self) -> Option<&ModelSnapshot> {
        self.snapshots
            .iter()
            .min_by(|a, b| a.val_loss.partial_cmp(&b.val_loss).unwrap())
    }
}
