use rand::{rngs::SmallRng, Rng, SeedableRng};
use serde::{Deserialize, Serialize};

use super::{ControlDirective, DirectivePriority, ModuleKind, ModuleTarget};

/// Metadata used to improvise directives beyond the standard plan.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImprovisationHint {
    /// Module kind that should receive the directive.
    pub target_kind: ModuleKind,
    /// Context label shown to operators.
    pub label: String,
    /// Desired aggressiveness between 0 and 1.
    pub aggressiveness: f32,
}

impl ImprovisationHint {
    /// Creates a new hint.
    #[must_use]
    pub fn new(target_kind: ModuleKind, label: impl Into<String>, aggressiveness: f32) -> Self {
        Self {
            target_kind,
            label: label.into(),
            aggressiveness: aggressiveness.clamp(0.0, 1.0),
        }
    }
}

/// Generates improvisational directives when brittle plans need adaptation.
#[derive(Debug)]
pub struct ImprovisationEngine {
    rng: SmallRng,
}

impl Default for ImprovisationEngine {
    fn default() -> Self {
        Self {
            rng: SmallRng::from_entropy(),
        }
    }
}

impl ImprovisationEngine {
    /// Produces a speculative directive bound to the hint configuration.
    #[must_use]
    pub fn propose(&mut self, hint: &ImprovisationHint) -> ControlDirective {
        let urgency = if self.rng.gen_bool(f64::from(hint.aggressiveness)) {
            DirectivePriority::Critical
        } else {
            DirectivePriority::Elevated
        };

        let instructions = format!(
            "[IMPROVISE:{}] increase resilience buffers by {:.2}%",
            hint.label,
            10.0 + (hint.aggressiveness * 80.0)
        );

        ControlDirective::new(ModuleTarget::Kind(hint.target_kind.clone()), instructions)
            .with_priority(urgency)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::module::ModuleTarget;

    #[test]
    fn hint_generates_directive() {
        let mut engine = ImprovisationEngine::default();
        let hint = ImprovisationHint::new(ModuleKind::Executor, "stability", 0.7);
        let directive = engine.propose(&hint);
        match directive.target {
            ModuleTarget::Kind(kind) => assert_eq!(kind, ModuleKind::Executor),
            _ => panic!("expected kind target"),
        }
    }
}
