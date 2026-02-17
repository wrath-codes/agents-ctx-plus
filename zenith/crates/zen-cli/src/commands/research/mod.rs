mod create;
mod get;
mod list;
mod registry;
mod update;

use crate::cli::GlobalFlags;
use crate::cli::subcommands::ResearchCommands;
use crate::context::AppContext;

/// Handle `znt research`.
pub async fn handle(
    action: &ResearchCommands,
    ctx: &mut AppContext,
    flags: &GlobalFlags,
) -> anyhow::Result<()> {
    match action {
        ResearchCommands::Create { title, description } => {
            create::run(title, description.as_deref(), ctx, flags).await
        }
        ResearchCommands::Update {
            id,
            title,
            description,
            status,
        } => {
            update::run(
                id,
                title.as_deref(),
                description.clone(),
                status.as_deref(),
                ctx,
                flags,
            )
            .await
        }
        ResearchCommands::List {
            status,
            search,
            limit,
        } => list::run(status.as_deref(), search.as_deref(), *limit, ctx, flags).await,
        ResearchCommands::Get { id } => get::run(id, ctx, flags).await,
        ResearchCommands::Registry {
            query,
            ecosystem,
            limit,
        } => registry::run(query, ecosystem.as_deref(), *limit, ctx, flags).await,
    }
}
