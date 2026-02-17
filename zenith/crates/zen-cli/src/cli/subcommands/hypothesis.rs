use clap::Subcommand;

/// Hypothesis entity commands.
#[derive(Clone, Debug, Subcommand)]
pub enum HypothesisCommands {
    /// Create a hypothesis.
    Create {
        #[arg(long)]
        content: String,
        #[arg(long)]
        research: Option<String>,
        #[arg(long)]
        finding: Option<String>,
    },
    /// Update a hypothesis.
    Update {
        id: String,
        #[arg(long)]
        content: Option<String>,
        #[arg(long)]
        reason: Option<String>,
        #[arg(long)]
        status: Option<String>,
    },
    /// List hypotheses.
    List {
        #[arg(long)]
        status: Option<String>,
        #[arg(long)]
        research: Option<String>,
        #[arg(long)]
        search: Option<String>,
        #[arg(long)]
        limit: Option<u32>,
    },
    /// Get a hypothesis by ID.
    Get { id: String },
}
