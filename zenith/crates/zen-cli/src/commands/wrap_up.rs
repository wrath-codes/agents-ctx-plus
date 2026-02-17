use anyhow::bail;

use crate::cli::GlobalFlags;
use crate::cli::root_commands::WrapUpArgs;
use crate::context::AppContext;

/// Handle `znt wrap-up`.
pub async fn handle(
    _args: &WrapUpArgs,
    _ctx: &mut AppContext,
    _flags: &GlobalFlags,
) -> anyhow::Result<()> {
    bail!("znt wrap-up is not implemented yet")
}
