#[path = "compat/check.rs"]
mod check;
#[path = "compat/get.rs"]
mod get;
#[path = "compat/list.rs"]
mod list;

use crate::cli::GlobalFlags;
use crate::cli::subcommands::CompatCommands;
use crate::context::AppContext;

/// Handle `znt compat`.
pub async fn handle(
    action: &CompatCommands,
    ctx: &mut AppContext,
    flags: &GlobalFlags,
) -> anyhow::Result<()> {
    match action {
        CompatCommands::Check {
            package_a,
            package_b,
            status,
            conditions,
            finding,
        } => {
            check::run(
                package_a,
                package_b,
                status.as_deref(),
                conditions.clone(),
                finding.clone(),
                ctx,
                flags,
            )
            .await
        }
        CompatCommands::List {
            status,
            package,
            limit,
        } => list::run(status.as_deref(), package.as_deref(), *limit, ctx, flags).await,
        CompatCommands::Get { id } => get::run(id, ctx, flags).await,
    }
}
