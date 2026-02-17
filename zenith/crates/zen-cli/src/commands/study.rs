use anyhow::bail;

use crate::cli::GlobalFlags;
use crate::cli::subcommands::StudyCommands;
use crate::context::AppContext;

/// Handle `znt study`.
pub async fn handle(
    _action: &StudyCommands,
    _ctx: &mut AppContext,
    _flags: &GlobalFlags,
) -> anyhow::Result<()> {
    bail!("znt study is not implemented yet")
}
