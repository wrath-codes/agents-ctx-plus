mod assume;
mod conclude;
mod create;
mod get;
mod list;
mod test;

use crate::cli::GlobalFlags;
use crate::cli::subcommands::StudyCommands;
use crate::context::AppContext;

/// Handle `znt study`.
pub async fn handle(
    action: &StudyCommands,
    ctx: &mut AppContext,
    flags: &GlobalFlags,
) -> anyhow::Result<()> {
    match action {
        StudyCommands::Create {
            topic,
            library,
            methodology,
            research,
        } => {
            create::run(
                topic,
                library.as_deref(),
                methodology.as_deref(),
                research.as_deref(),
                ctx,
                flags,
            )
            .await
        }
        StudyCommands::Assume { id, content } => assume::run(id, content, ctx, flags).await,
        StudyCommands::Test {
            id,
            assumption_id,
            result,
            evidence,
        } => test::run(id, assumption_id, result, evidence.as_deref(), ctx, flags).await,
        StudyCommands::Get { id } => get::run(id, ctx, flags).await,
        StudyCommands::Conclude { id, summary } => conclude::run(id, summary, ctx, flags).await,
        StudyCommands::List {
            status,
            library,
            limit,
        } => list::run(status.as_deref(), library.as_deref(), *limit, ctx, flags).await,
    }
}
