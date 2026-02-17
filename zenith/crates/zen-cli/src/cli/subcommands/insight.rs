use clap::Subcommand;

/// Insight entity commands.
#[derive(Clone, Debug, Subcommand)]
pub enum InsightCommands {
    /// Create an insight.
    Create {
        #[arg(long)]
        content: String,
        #[arg(long)]
        category: Option<String>,
    },
    /// Update an insight.
    Update {
        id: String,
        #[arg(long)]
        content: Option<String>,
        #[arg(long)]
        category: Option<String>,
    },
    /// List insights.
    List {
        #[arg(long)]
        search: Option<String>,
        #[arg(long)]
        category: Option<String>,
        #[arg(long)]
        limit: Option<u32>,
    },
    /// Get an insight by ID.
    Get { id: String },
}
