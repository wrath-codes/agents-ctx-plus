use anyhow::bail;

use crate::cli::GlobalFlags;
use crate::cli::subcommands::CompatCommands;
use crate::context::AppContext;

/// Handle `znt compat`.
pub async fn handle(
    _action: &CompatCommands,
    _ctx: &mut AppContext,
    _flags: &GlobalFlags,
) -> anyhow::Result<()> {
    bail!("znt compat is not implemented yet")
}
