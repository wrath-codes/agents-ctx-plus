use clap::{Args, Subcommand};

/// Authentication commands.
#[derive(Clone, Debug, Subcommand)]
pub enum AuthCommands {
    /// Log in via browser (or --api-key for CI).
    Login(AuthLoginArgs),
    /// Clear stored credentials.
    Logout,
    /// Show current auth status.
    Status,
    /// Switch to a different Clerk organization.
    SwitchOrg(AuthSwitchOrgArgs),
}

#[derive(Clone, Debug, Args)]
pub struct AuthLoginArgs {
    /// Use API key for CI/headless auth instead of browser.
    #[arg(long)]
    pub api_key: bool,
    /// Clerk user ID (required with --api-key).
    #[arg(long, requires = "api_key")]
    pub user_id: Option<String>,
}

#[derive(Clone, Debug, Args)]
pub struct AuthSwitchOrgArgs {
    /// Organization slug to switch to.
    pub org_slug: String,
}
