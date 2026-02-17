use anyhow::bail;

use crate::cli::GlobalFlags;
use crate::cli::subcommands::SessionCommands;
use crate::context::AppContext;

/// Handle `znt session`.
pub async fn handle(
    _action: &SessionCommands,
    _ctx: &mut AppContext,
    _flags: &GlobalFlags,
) -> anyhow::Result<()> {
    bail!("znt session is not implemented yet")
}
