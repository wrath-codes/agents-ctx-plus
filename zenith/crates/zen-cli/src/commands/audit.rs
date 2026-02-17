use anyhow::bail;

use crate::cli::GlobalFlags;
use crate::cli::root_commands::AuditArgs;
use crate::context::AppContext;

/// Handle `znt audit`.
pub async fn handle(
    _args: &AuditArgs,
    _ctx: &mut AppContext,
    _flags: &GlobalFlags,
) -> anyhow::Result<()> {
    bail!("znt audit is not implemented yet")
}
