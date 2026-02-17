use anyhow::bail;

use crate::cli::GlobalFlags;
use crate::cli::root_commands::SearchArgs;
use crate::context::AppContext;

/// Handle `znt search`.
pub async fn handle(
    _args: &SearchArgs,
    _ctx: &mut AppContext,
    _flags: &GlobalFlags,
) -> anyhow::Result<()> {
    bail!("znt search is not implemented yet")
}
