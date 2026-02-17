use anyhow::bail;

use crate::cli::GlobalFlags;
use crate::cli::root_commands::InitArgs;

/// Handle `znt init`.
pub async fn handle(args: &InitArgs, _flags: &GlobalFlags) -> anyhow::Result<()> {
    bail!("znt init is not implemented yet (name={:?})", args.name)
}
