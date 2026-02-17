use anyhow::bail;

use crate::cli::GlobalFlags;
use crate::cli::subcommands::TaskCommands;
use crate::context::AppContext;

/// Handle `znt task`.
pub async fn handle(
    _action: &TaskCommands,
    _ctx: &mut AppContext,
    _flags: &GlobalFlags,
) -> anyhow::Result<()> {
    bail!("znt task is not implemented yet")
}
