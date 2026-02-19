use serde::Serialize;
use zen_core::entities::Issue;
use zen_core::enums::IssueType;

use crate::cli::GlobalFlags;
use crate::commands::shared::session::require_active_session_id;
use crate::context::AppContext;
use crate::output::output;

#[derive(Debug, Serialize)]
struct PrdCreateResponse {
    prd: Issue,
}

pub async fn run(
    title: &str,
    description: Option<&str>,
    ctx: &AppContext,
    flags: &GlobalFlags,
) -> anyhow::Result<()> {
    let session_id = require_active_session_id(ctx).await?;
    let prd = ctx
        .service
        .create_issue(&session_id, title, IssueType::Epic, 3, description, None)
        .await?;

    output(&PrdCreateResponse { prd }, flags.format)
}
