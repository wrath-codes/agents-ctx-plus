use anyhow::bail;

use crate::cli::GlobalFlags;
use crate::cli::root_commands::RebuildArgs;
use crate::context::AppContext;

/// Handle `znt rebuild`.
pub async fn handle(
    _args: &RebuildArgs,
    _ctx: &mut AppContext,
    _flags: &GlobalFlags,
) -> anyhow::Result<()> {
    bail!("znt rebuild is not implemented yet")
}
