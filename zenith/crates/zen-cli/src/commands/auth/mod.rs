pub(crate) mod login;
mod logout;
mod status;
mod switch_org;

use crate::cli::GlobalFlags;
use crate::cli::subcommands::AuthCommands;

/// Handle `znt auth <subcommand>`.
pub async fn handle(action: &AuthCommands, flags: &GlobalFlags) -> anyhow::Result<()> {
    match action {
        AuthCommands::Login(args) => login::handle(args, flags).await,
        AuthCommands::Logout => logout::handle(flags).await,
        AuthCommands::Status => status::handle(flags).await,
        AuthCommands::SwitchOrg(args) => switch_org::handle(args, flags).await,
    }
}
