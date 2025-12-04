use crate::{
    actioncommander::ActionCommander,
    actions::{ActionError, ActionOutcome, ActionRequest},
};

/// Boots a production-grade commander with hardened defaults.
#[must_use]
pub fn bootstrap_commander() -> ActionCommander {
    ActionCommander::builder().build()
}

/// Helper that executes a single action end-to-end.
pub async fn execute_action(request: ActionRequest) -> Result<ActionOutcome, ActionError> {
    let commander = bootstrap_commander();
    commander.submit(request).await?.outcome().await
}
