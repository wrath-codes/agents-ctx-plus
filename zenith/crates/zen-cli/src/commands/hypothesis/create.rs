use crate::cli::GlobalFlags;
use crate::commands::shared::session::require_active_session_id;
use crate::context::AppContext;
use crate::output::output;

pub async fn run(
    content: &str,
    research: Option<&str>,
    finding: Option<&str>,
    ctx: &AppContext,
    flags: &GlobalFlags,
) -> anyhow::Result<()> {
    let session_id = require_active_session_id(ctx).await?;
    let hypothesis = ctx
        .service
        .create_hypothesis(&session_id, content, research, finding)
        .await?;
    output(&hypothesis, flags.format)
}
