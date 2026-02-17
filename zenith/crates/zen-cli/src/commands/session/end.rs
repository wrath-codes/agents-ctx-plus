use crate::cli::GlobalFlags;
use crate::commands::shared::session::require_active_session_id;
use crate::context::AppContext;
use crate::output::output;

pub async fn run(
    summary: Option<&str>,
    ctx: &AppContext,
    flags: &GlobalFlags,
) -> anyhow::Result<()> {
    let session_id = require_active_session_id(ctx).await?;
    let summary = summary.unwrap_or_default();
    let ended = ctx.service.end_session(&session_id, summary).await?;
    output(&ended, flags.format)
}
