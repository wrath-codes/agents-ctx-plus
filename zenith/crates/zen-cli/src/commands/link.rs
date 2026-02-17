use anyhow::bail;

use crate::cli::GlobalFlags;
use crate::cli::root_commands::{LinkArgs, UnlinkArgs};
use crate::context::AppContext;

/// Handle `znt link`.
pub async fn handle_link(
    _args: &LinkArgs,
    _ctx: &mut AppContext,
    _flags: &GlobalFlags,
) -> anyhow::Result<()> {
    bail!("znt link is not implemented yet")
}

/// Handle `znt unlink`.
pub async fn handle_unlink(
    _args: &UnlinkArgs,
    _ctx: &mut AppContext,
    _flags: &GlobalFlags,
) -> anyhow::Result<()> {
    bail!("znt unlink is not implemented yet")
}
