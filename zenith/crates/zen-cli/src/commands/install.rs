use anyhow::bail;

use crate::cli::GlobalFlags;
use crate::cli::root_commands::InstallArgs;
use crate::context::AppContext;

/// Handle `znt install`.
pub async fn handle(
    _args: &InstallArgs,
    _ctx: &mut AppContext,
    _flags: &GlobalFlags,
) -> anyhow::Result<()> {
    bail!("znt install is not implemented yet")
}
