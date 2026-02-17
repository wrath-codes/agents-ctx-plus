#[path = "issue/create.rs"]
mod create;
#[path = "issue/get.rs"]
mod get;
#[path = "issue/list.rs"]
mod list;
#[path = "issue/update.rs"]
mod update;

use crate::cli::GlobalFlags;
use crate::cli::subcommands::IssueCommands;
use crate::context::AppContext;

/// Handle `znt issue`.
pub async fn handle(
    action: &IssueCommands,
    ctx: &mut AppContext,
    flags: &GlobalFlags,
) -> anyhow::Result<()> {
    match action {
        IssueCommands::Create {
            title,
            issue_type,
            priority,
            description,
            parent,
        } => {
            create::run(
                title,
                issue_type.as_deref(),
                *priority,
                description.as_deref(),
                parent.as_deref(),
                ctx,
                flags,
            )
            .await
        }
        IssueCommands::Update {
            id,
            title,
            issue_type,
            description,
            status,
            priority,
        } => {
            update::run(
                update::Params {
                    id: id.clone(),
                    title: title.clone(),
                    issue_type: issue_type.clone(),
                    description: description.clone(),
                    status: status.clone(),
                    priority: *priority,
                },
                ctx,
                flags,
            )
            .await
        }
        IssueCommands::List {
            status,
            issue_type,
            search,
            limit,
        } => {
            list::run(
                status.as_deref(),
                issue_type.as_deref(),
                search.as_deref(),
                *limit,
                ctx,
                flags,
            )
            .await
        }
        IssueCommands::Get { id } => get::run(id, ctx, flags).await,
    }
}
