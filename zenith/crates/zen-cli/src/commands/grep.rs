use anyhow::bail;

use crate::cli::GlobalFlags;
use crate::cli::root_commands::GrepArgs;
use crate::context::AppContext;

/// Handle `znt grep`.
pub async fn handle(
    _args: &GrepArgs,
    _ctx: &mut AppContext,
    _flags: &GlobalFlags,
) -> anyhow::Result<()> {
    bail!("znt grep is not implemented yet")
}
