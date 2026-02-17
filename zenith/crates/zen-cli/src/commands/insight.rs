use anyhow::bail;

use crate::cli::GlobalFlags;
use crate::cli::subcommands::InsightCommands;
use crate::context::AppContext;

/// Handle `znt insight`.
pub async fn handle(
    _action: &InsightCommands,
    _ctx: &mut AppContext,
    _flags: &GlobalFlags,
) -> anyhow::Result<()> {
    bail!("znt insight is not implemented yet")
}
