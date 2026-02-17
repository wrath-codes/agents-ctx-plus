use clap::Subcommand;

/// Task entity commands.
#[derive(Clone, Debug, Subcommand)]
pub enum TaskCommands {
    /// Create a task.
    Create {
        #[arg(long)]
        title: String,
        #[arg(long)]
        description: Option<String>,
        #[arg(long)]
        issue: Option<String>,
        #[arg(long)]
        priority: Option<String>,
    },
    /// Update a task.
    Update {
        id: String,
        #[arg(long)]
        title: Option<String>,
        #[arg(long)]
        description: Option<String>,
        #[arg(long)]
        status: Option<String>,
        #[arg(long)]
        priority: Option<String>,
        #[arg(long)]
        issue: Option<String>,
    },
    /// List tasks.
    List {
        #[arg(long)]
        status: Option<String>,
        #[arg(long)]
        issue: Option<String>,
        #[arg(long)]
        search: Option<String>,
        #[arg(long)]
        limit: Option<u32>,
    },
    /// Get a task by ID.
    Get { id: String },
    /// Mark a task completed.
    Complete { id: String },
}
