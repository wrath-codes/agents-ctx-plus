use anyhow::bail;

use crate::cli::subcommands::HookCommands;

/// Handle `znt hook`.
pub async fn handle(action: &HookCommands) -> anyhow::Result<()> {
    bail!("znt hook is not implemented yet ({action:?})")
}
