use anyhow::bail;

use crate::cli::GlobalFlags;
use crate::cli::root_commands::LogArgs;
use crate::context::AppContext;

/// Handle `znt log`.
pub async fn handle(
    _args: &LogArgs,
    _ctx: &mut AppContext,
    _flags: &GlobalFlags,
) -> anyhow::Result<()> {
    bail!("znt log is not implemented yet")
}
