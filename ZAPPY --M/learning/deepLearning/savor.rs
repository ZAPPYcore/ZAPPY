use std::collections::VecDeque;

use super::model::ModelSnapshot;

/// Keeps the top-K model snapshots based on lowest validation loss.
#[derive(Debug)]
pub struct SnapshotSavor {
    capacity: usize,
    snapshots: VecDeque<ModelSnapshot>,
}

impl SnapshotSavor {
    /// Creates a new savor with capacity.
    #[must_use]
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity: capacity.max(1),
            snapshots: VecDeque::new(),
        }
    }

    /// Attempts to store the snapshot.
    pub fn store(&mut self, snapshot: ModelSnapshot) {
        self.snapshots.push_back(snapshot);
        self.snapshots
            .make_contiguous()
            .sort_by(|a, b| a.val_loss.partial_cmp(&b.val_loss).unwrap());
        while self.snapshots.len() > self.capacity {
            self.snapshots.pop_back();
        }
    }

    /// Returns the best snapshot.
    #[must_use]
    pub fn best(&self) -> Option<&ModelSnapshot> {
        self.snapshots.front()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn savor_tracks_best_snapshot() {
        let mut savor = SnapshotSavor::new(2);
        savor.store(ModelSnapshot {
            id: uuid::Uuid::new_v4(),
            step: 1,
            val_loss: 0.9,
        });
        savor.store(ModelSnapshot {
            id: uuid::Uuid::new_v4(),
            step: 2,
            val_loss: 0.3,
        });
        assert!(savor.best().unwrap().val_loss < 0.5);
    }
}
