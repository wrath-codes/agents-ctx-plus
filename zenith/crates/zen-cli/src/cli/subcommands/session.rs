use clap::Subcommand;

/// Session lifecycle commands.
#[derive(Clone, Debug, Subcommand)]
pub enum SessionCommands {
    /// Start a new active session.
    Start,
    /// End the active session.
    End {
        /// Optional human summary to store with session wrap-up.
        #[arg(long)]
        summary: Option<String>,
    },
    /// List sessions.
    List {
        /// Optional status filter.
        #[arg(long)]
        status: Option<String>,
        /// Maximum number of sessions.
        #[arg(long)]
        limit: Option<u32>,
    },
}
