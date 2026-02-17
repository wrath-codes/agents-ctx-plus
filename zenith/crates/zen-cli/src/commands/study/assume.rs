use serde::Serialize;

use crate::cli::GlobalFlags;
use crate::commands::shared::session::require_active_session_id;
use crate::context::AppContext;
use crate::output::output;

#[derive(Debug, Serialize)]
struct AssumeResponse {
    study_id: String,
    assumption_id: String,
}

pub async fn run(
    study_id: &str,
    content: &str,
    ctx: &AppContext,
    flags: &GlobalFlags,
) -> anyhow::Result<()> {
    let session_id = require_active_session_id(ctx).await?;
    let assumption_id = ctx
        .service
        .add_assumption(&session_id, study_id, content)
        .await?;
    output(
        &AssumeResponse {
            study_id: study_id.to_string(),
            assumption_id,
        },
        flags.format,
    )
}
