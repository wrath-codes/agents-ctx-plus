#[path = "task/complete.rs"]
mod complete;
#[path = "task/create.rs"]
mod create;
#[path = "task/get.rs"]
mod get;
#[path = "task/list.rs"]
mod list;
#[path = "task/update.rs"]
mod update;

use crate::cli::GlobalFlags;
use crate::cli::subcommands::TaskCommands;
use crate::context::AppContext;

/// Handle `znt task`.
pub async fn handle(
    action: &TaskCommands,
    ctx: &mut AppContext,
    flags: &GlobalFlags,
) -> anyhow::Result<()> {
    match action {
        TaskCommands::Create {
            title,
            description,
            issue,
            research,
        } => {
            create::run(
                title,
                description.as_deref(),
                issue.as_deref(),
                research.as_deref(),
                ctx,
                flags,
            )
            .await
        }
        TaskCommands::Update {
            id,
            title,
            description,
            status,
            research,
            issue,
        } => {
            update::run(
                update::Params {
                    id: id.clone(),
                    title: title.clone(),
                    description: description.clone(),
                    status: status.clone(),
                    research: research.clone(),
                    issue: issue.clone(),
                },
                ctx,
                flags,
            )
            .await
        }
        TaskCommands::List {
            status,
            issue,
            research,
            search,
            limit,
        } => {
            list::run(
                status.as_deref(),
                issue.as_deref(),
                research.as_deref(),
                search.as_deref(),
                *limit,
                ctx,
                flags,
            )
            .await
        }
        TaskCommands::Get { id } => get::run(id, ctx, flags).await,
        TaskCommands::Complete { id } => complete::run(id, ctx, flags).await,
    }
}
