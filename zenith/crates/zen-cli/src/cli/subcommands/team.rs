use clap::{Args, Subcommand};

/// Team management commands.
#[derive(Clone, Debug, Subcommand)]
pub enum TeamCommands {
    /// Invite a user to the current organization.
    Invite(TeamInviteArgs),
    /// List members of the current organization.
    List,
}

#[derive(Clone, Debug, Args)]
pub struct TeamInviteArgs {
    /// Email address to invite.
    pub email: String,
    /// Role to assign (default: org:member).
    #[arg(long, default_value = "org:member")]
    pub role: String,
}
