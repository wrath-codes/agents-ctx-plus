use serde_json::json;

use crate::cli::GlobalFlags;
use crate::commands::shared::session::require_active_session_id;
use crate::context::AppContext;
use crate::output::output;

pub async fn run(id: &str, tag: &str, ctx: &AppContext, flags: &GlobalFlags) -> anyhow::Result<()> {
    let session_id = require_active_session_id(ctx).await?;
    ctx.service.untag_finding(&session_id, id, tag).await?;
    output(
        &json!({"untagged": true, "finding_id": id, "tag": tag}),
        flags.format,
    )
}
