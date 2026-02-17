#[path = "rebuild/handle.rs"]
mod handle;

use crate::cli::GlobalFlags;
use crate::cli::root_commands::RebuildArgs;
use crate::context::AppContext;

/// Handle `znt rebuild`.
pub async fn handle(
    args: &RebuildArgs,
    ctx: &mut AppContext,
    flags: &GlobalFlags,
) -> anyhow::Result<()> {
    handle::run(args, ctx, flags).await
}
