use clap::Subcommand;

/// Issue entity commands.
#[derive(Clone, Debug, Subcommand)]
pub enum IssueCommands {
    /// Create an issue.
    Create {
        #[arg(long)]
        title: String,
        #[arg(long = "type")]
        issue_type: Option<String>,
        #[arg(long)]
        description: Option<String>,
        #[arg(long)]
        parent: Option<String>,
    },
    /// Update an issue.
    Update {
        id: String,
        #[arg(long)]
        title: Option<String>,
        #[arg(long = "type")]
        issue_type: Option<String>,
        #[arg(long)]
        description: Option<String>,
        #[arg(long)]
        status: Option<String>,
    },
    /// List issues.
    List {
        #[arg(long)]
        status: Option<String>,
        #[arg(long = "type")]
        issue_type: Option<String>,
        #[arg(long)]
        search: Option<String>,
        #[arg(long)]
        limit: Option<u32>,
    },
    /// Get an issue by ID.
    Get { id: String },
}
