use anyhow::bail;

use crate::cli::GlobalFlags;
use crate::cli::subcommands::FindingCommands;
use crate::context::AppContext;

/// Handle `znt finding`.
pub async fn handle(
    _action: &FindingCommands,
    _ctx: &mut AppContext,
    _flags: &GlobalFlags,
) -> anyhow::Result<()> {
    bail!("znt finding is not implemented yet")
}
