mod create;
mod get;
mod list;
mod update;

use crate::cli::GlobalFlags;
use crate::cli::subcommands::HypothesisCommands;
use crate::context::AppContext;

/// Handle `znt hypothesis`.
pub async fn handle(
    action: &HypothesisCommands,
    ctx: &mut AppContext,
    flags: &GlobalFlags,
) -> anyhow::Result<()> {
    match action {
        HypothesisCommands::Create {
            content,
            research,
            finding,
        } => create::run(content, research.as_deref(), finding.as_deref(), ctx, flags).await,
        HypothesisCommands::Update {
            id,
            content,
            reason,
            status,
        } => {
            update::run(
                id,
                content.as_deref(),
                reason.clone(),
                status.as_deref(),
                ctx,
                flags,
            )
            .await
        }
        HypothesisCommands::List {
            status,
            research,
            search,
            limit,
        } => {
            list::run(
                status.as_deref(),
                research.as_deref(),
                search.as_deref(),
                *limit,
                ctx,
                flags,
            )
            .await
        }
        HypothesisCommands::Get { id } => get::run(id, ctx, flags).await,
    }
}
