use crate::cli::GlobalFlags;
use crate::cli::root_commands::LogArgs;
use crate::commands::shared::session::require_active_session_id;
use crate::context::AppContext;
use crate::output::output;

use super::parse_location::parse_location;

pub async fn run(args: &LogArgs, ctx: &AppContext, flags: &GlobalFlags) -> anyhow::Result<()> {
    let session_id = require_active_session_id(ctx).await?;
    let task_id = args
        .task
        .as_deref()
        .ok_or_else(|| anyhow::anyhow!("--task is required for znt log"))?;
    let parsed = parse_location(&args.location)?;

    let log = ctx
        .service
        .create_impl_log(
            &session_id,
            task_id,
            &parsed.file_path,
            parsed.start_line,
            parsed.end_line,
            args.description.as_deref(),
        )
        .await?;

    output(&log, flags.format)
}
