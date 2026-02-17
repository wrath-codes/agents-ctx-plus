#[path = "link/create.rs"]
mod create;
#[path = "link/delete.rs"]
mod delete_cmd;

use crate::cli::GlobalFlags;
use crate::cli::root_commands::{LinkArgs, UnlinkArgs};
use crate::context::AppContext;

/// Handle `znt link`.
pub async fn handle_link(
    args: &LinkArgs,
    ctx: &mut AppContext,
    flags: &GlobalFlags,
) -> anyhow::Result<()> {
    create::run(args, ctx, flags).await
}

/// Handle `znt unlink`.
pub async fn handle_unlink(
    args: &UnlinkArgs,
    ctx: &mut AppContext,
    flags: &GlobalFlags,
) -> anyhow::Result<()> {
    delete_cmd::run(args, ctx, flags).await
}
