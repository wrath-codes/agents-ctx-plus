use crate::cli::GlobalFlags;
use crate::commands::shared::session::require_active_session_id;
use crate::context::AppContext;
use crate::output::output;

pub async fn run(
    title: &str,
    description: Option<&str>,
    issue: Option<&str>,
    research: Option<&str>,
    ctx: &AppContext,
    flags: &GlobalFlags,
) -> anyhow::Result<()> {
    let session_id = require_active_session_id(ctx).await?;
    let task = ctx
        .service
        .create_task(&session_id, title, description, issue, research)
        .await?;
    output(&task, flags.format)
}
