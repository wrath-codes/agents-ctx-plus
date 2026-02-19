#[path = "prd/complete.rs"]
mod complete;
#[path = "prd/create.rs"]
mod create;
#[path = "prd/get.rs"]
mod get;
#[path = "prd/list.rs"]
mod list;
#[path = "prd/subtasks.rs"]
mod subtasks;
#[path = "prd/tasks.rs"]
mod tasks;
#[path = "prd/update.rs"]
mod update;

use crate::cli::GlobalFlags;
use crate::cli::subcommands::PrdCommands;
use crate::context::AppContext;

/// Handle `znt prd`.
pub async fn handle(
    action: &PrdCommands,
    ctx: &mut AppContext,
    flags: &GlobalFlags,
) -> anyhow::Result<()> {
    match action {
        PrdCommands::Create { title, description } => {
            create::run(title, description.as_deref(), ctx, flags).await
        }
        PrdCommands::Update { id, content } => update::run(id, content, ctx, flags).await,
        PrdCommands::Get { id } => get::run(id, ctx, flags).await,
        PrdCommands::Tasks { id, tasks } => tasks::run(id, tasks, ctx, flags).await,
        PrdCommands::Subtasks { id, epic, tasks } => {
            subtasks::run(id, epic, tasks, ctx, flags).await
        }
        PrdCommands::Complete { id } => complete::run(id, ctx, flags).await,
        PrdCommands::List {
            status,
            search,
            limit,
        } => list::run(status.as_deref(), search.as_deref(), *limit, ctx, flags).await,
    }
}
