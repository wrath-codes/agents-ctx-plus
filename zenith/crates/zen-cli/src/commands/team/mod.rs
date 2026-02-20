pub mod invite;
pub mod list;

use crate::cli::GlobalFlags;
use crate::cli::subcommands::team::TeamCommands;
use crate::context::AppContext;

pub async fn handle(
    action: &TeamCommands,
    ctx: &AppContext,
    flags: &GlobalFlags,
) -> anyhow::Result<()> {
    match action {
        TeamCommands::Invite(args) => invite::handle(args, ctx, flags).await,
        TeamCommands::List => list::handle(ctx, flags).await,
    }
}
