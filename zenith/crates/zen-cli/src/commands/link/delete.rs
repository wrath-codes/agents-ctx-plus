use serde_json::json;

use crate::cli::GlobalFlags;
use crate::cli::root_commands::UnlinkArgs;
use crate::commands::shared::session::require_active_session_id;
use crate::context::AppContext;
use crate::output::output;

pub async fn run(args: &UnlinkArgs, ctx: &AppContext, flags: &GlobalFlags) -> anyhow::Result<()> {
    let session_id = require_active_session_id(ctx).await?;
    ctx.service.delete_link(&session_id, &args.link_id).await?;
    output(
        &json!({"deleted": true, "link_id": args.link_id}),
        flags.format,
    )
}
