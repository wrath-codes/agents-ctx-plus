use anyhow::bail;

use crate::cli::GlobalFlags;
use crate::cli::subcommands::ResearchCommands;
use crate::context::AppContext;

/// Handle `znt research`.
pub async fn handle(
    _action: &ResearchCommands,
    _ctx: &mut AppContext,
    _flags: &GlobalFlags,
) -> anyhow::Result<()> {
    bail!("znt research is not implemented yet")
}
