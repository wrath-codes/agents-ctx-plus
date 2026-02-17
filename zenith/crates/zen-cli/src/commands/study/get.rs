use crate::cli::GlobalFlags;
use crate::context::AppContext;
use crate::output::output;
use serde_json::json;

pub async fn run(id: &str, ctx: &AppContext, flags: &GlobalFlags) -> anyhow::Result<()> {
    let state = ctx.service.get_study_full_state(id).await?;
    output(
        &json!({
            "study": state.study,
            "assumptions": state.assumptions.iter().map(|item| {
                json!({"id": item.id, "content": item.content, "status": item.status})
            }).collect::<Vec<_>>(),
            "findings": state.findings.iter().map(|item| {
                json!({"id": item.id, "content": item.content, "confidence": item.confidence})
            }).collect::<Vec<_>>(),
            "conclusions": state.conclusions.iter().map(|item| {
                json!({"id": item.id, "content": item.content, "confidence": item.confidence})
            }).collect::<Vec<_>>(),
        }),
        flags.format,
    )
}
