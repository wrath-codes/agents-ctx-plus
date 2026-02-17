#[path = "log/create.rs"]
mod create;
#[path = "log/parse_location.rs"]
mod parse_location;

use crate::cli::GlobalFlags;
use crate::cli::root_commands::LogArgs;
use crate::context::AppContext;

/// Handle `znt log`.
pub async fn handle(
    args: &LogArgs,
    ctx: &mut AppContext,
    flags: &GlobalFlags,
) -> anyhow::Result<()> {
    create::run(args, ctx, flags).await
}
