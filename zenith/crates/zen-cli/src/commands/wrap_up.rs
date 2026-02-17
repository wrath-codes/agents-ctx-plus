#[path = "wrap_up/handle.rs"]
mod handle;
#[path = "wrap_up/summary.rs"]
mod summary;
#[path = "wrap_up/sync.rs"]
mod sync;

use crate::cli::GlobalFlags;
use crate::cli::root_commands::WrapUpArgs;
use crate::context::AppContext;

/// Handle `znt wrap-up`.
pub async fn handle(
    args: &WrapUpArgs,
    ctx: &mut AppContext,
    flags: &GlobalFlags,
) -> anyhow::Result<()> {
    handle::run(args, ctx, flags).await
}
