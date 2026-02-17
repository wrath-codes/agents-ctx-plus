use anyhow::bail;

use crate::cli::GlobalFlags;
use crate::cli::subcommands::IssueCommands;
use crate::context::AppContext;

/// Handle `znt issue`.
pub async fn handle(
    _action: &IssueCommands,
    _ctx: &mut AppContext,
    _flags: &GlobalFlags,
) -> anyhow::Result<()> {
    bail!("znt issue is not implemented yet")
}
