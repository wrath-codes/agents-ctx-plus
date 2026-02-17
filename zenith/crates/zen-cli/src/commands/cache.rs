use anyhow::bail;

use crate::cli::GlobalFlags;
use crate::cli::subcommands::CacheCommands;
use crate::context::AppContext;

/// Handle `znt cache`.
pub async fn handle(
    _action: &CacheCommands,
    _ctx: &mut AppContext,
    _flags: &GlobalFlags,
) -> anyhow::Result<()> {
    bail!("znt cache is not implemented yet")
}
