use crate::{
    actioncommander::ActionCommander,
    actions::{ActionDomain, ActionIntent, ActionPayload, ActionRequest, PayloadAttachment},
};

/// Creates a sample request and executes it end-to-end.
pub async fn orchestrate_sample() -> anyhow::Result<()> {
    let payload = ActionPayload {
        summary: "Stabilize grid".into(),
        narrative: "Balance load between regions".into(),
        attachments: vec![PayloadAttachment {
            label: "code_context".into(),
            content_type: "application/json".into(),
            content: serde_json::json!({
                "path": "grid.rs",
                "original": "fn balance() {}",
                "proposed": "fn balance(grid: &mut Grid) {}"
            }),
        }],
    };
    let request =
        ActionRequest::builder(ActionDomain::Programming, ActionIntent::Program, payload).build();
    let commander = ActionCommander::builder().build();
    commander.submit(request).await?.outcome().await?;
    Ok(())
}
