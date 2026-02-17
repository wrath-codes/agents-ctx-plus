use anyhow::bail;

use crate::cli::GlobalFlags;
use crate::cli::root_commands::OnboardArgs;
use crate::context::AppContext;

/// Handle `znt onboard`.
pub async fn handle(
    _args: &OnboardArgs,
    _ctx: &mut AppContext,
    _flags: &GlobalFlags,
) -> anyhow::Result<()> {
    bail!("znt onboard is not implemented yet")
}
