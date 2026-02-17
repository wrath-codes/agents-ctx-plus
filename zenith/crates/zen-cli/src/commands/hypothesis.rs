use anyhow::bail;

use crate::cli::GlobalFlags;
use crate::cli::subcommands::HypothesisCommands;
use crate::context::AppContext;

/// Handle `znt hypothesis`.
pub async fn handle(
    _action: &HypothesisCommands,
    _ctx: &mut AppContext,
    _flags: &GlobalFlags,
) -> anyhow::Result<()> {
    bail!("znt hypothesis is not implemented yet")
}
