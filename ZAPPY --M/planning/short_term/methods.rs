/// Scheduling strategy used by the short-term planner.
#[derive(Debug, Clone, Copy)]
pub enum TacticalMethod {
    /// Kanban-style continuous flow.
    Kanban,
    /// Time-boxed sprint planning.
    Sprint,
}

impl TacticalMethod {
    /// Returns label for logging.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Kanban => "kanban",
            Self::Sprint => "sprint",
        }
    }

    /// Returns multiplier used for cadence calculations.
    #[must_use]
    pub fn cadence_multiplier(self) -> u32 {
        match self {
            Self::Kanban => 1,
            Self::Sprint => 2,
        }
    }
}
