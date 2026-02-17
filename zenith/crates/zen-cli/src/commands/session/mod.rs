mod end;
mod list;
mod start;
mod types;

use crate::cli::GlobalFlags;
use crate::cli::subcommands::SessionCommands;
use crate::context::AppContext;

/// Handle `znt session`.
pub async fn handle(
    action: &SessionCommands,
    ctx: &mut AppContext,
    flags: &GlobalFlags,
) -> anyhow::Result<()> {
    match action {
        SessionCommands::Start => start::run(ctx, flags).await,
        SessionCommands::End { summary } => end::run(summary.as_deref(), ctx, flags).await,
        SessionCommands::List { status, limit } => {
            list::run(status.as_deref(), *limit, ctx, flags).await
        }
    }
}
