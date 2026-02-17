mod create;
mod get;
mod list;
mod tag;
mod untag;
mod update;

use crate::cli::GlobalFlags;
use crate::cli::subcommands::FindingCommands;
use crate::context::AppContext;

/// Handle `znt finding`.
pub async fn handle(
    action: &FindingCommands,
    ctx: &mut AppContext,
    flags: &GlobalFlags,
) -> anyhow::Result<()> {
    match action {
        FindingCommands::Create {
            content,
            source,
            confidence,
            research,
            tag,
        } => {
            create::run(
                content,
                source.as_deref(),
                confidence.as_deref(),
                research.as_deref(),
                tag,
                ctx,
                flags,
            )
            .await
        }
        FindingCommands::Update {
            id,
            content,
            source,
            confidence,
        } => {
            update::run(
                id,
                content.as_deref(),
                source.clone(),
                confidence.as_deref(),
                ctx,
                flags,
            )
            .await
        }
        FindingCommands::List {
            search,
            research,
            confidence,
            tag,
            limit,
        } => {
            list::run(
                search.as_deref(),
                research.as_deref(),
                confidence.as_deref(),
                tag.as_deref(),
                *limit,
                ctx,
                flags,
            )
            .await
        }
        FindingCommands::Get { id } => get::run(id, ctx, flags).await,
        FindingCommands::Tag { id, tag } => tag::run(id, tag, ctx, flags).await,
        FindingCommands::Untag { id, tag } => untag::run(id, tag, ctx, flags).await,
    }
}
