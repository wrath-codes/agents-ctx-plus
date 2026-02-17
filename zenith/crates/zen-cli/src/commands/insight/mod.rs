mod create;
mod get;
mod list;
mod update;

use crate::cli::GlobalFlags;
use crate::cli::subcommands::InsightCommands;
use crate::context::AppContext;

/// Handle `znt insight`.
pub async fn handle(
    action: &InsightCommands,
    ctx: &mut AppContext,
    flags: &GlobalFlags,
) -> anyhow::Result<()> {
    match action {
        InsightCommands::Create {
            content,
            confidence,
            research,
        } => {
            create::run(
                content,
                confidence.as_deref(),
                research.as_deref(),
                ctx,
                flags,
            )
            .await
        }
        InsightCommands::Update {
            id,
            content,
            confidence,
        } => update::run(id, content.as_deref(), confidence.as_deref(), ctx, flags).await,
        InsightCommands::List {
            search,
            confidence,
            research,
            limit,
        } => {
            list::run(
                search.as_deref(),
                confidence.as_deref(),
                research.as_deref(),
                *limit,
                ctx,
                flags,
            )
            .await
        }
        InsightCommands::Get { id } => get::run(id, ctx, flags).await,
    }
}
