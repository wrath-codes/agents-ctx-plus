use crate::cli::GlobalFlags;
use crate::commands::shared::session::require_active_session_id;
use crate::context::AppContext;
use crate::output::output;

pub async fn run(
    study_id: &str,
    summary: &str,
    ctx: &AppContext,
    flags: &GlobalFlags,
) -> anyhow::Result<()> {
    let session_id = require_active_session_id(ctx).await?;
    let study = ctx
        .service
        .conclude_study(&session_id, study_id, summary)
        .await?;
    output(&study, flags.format)
}
