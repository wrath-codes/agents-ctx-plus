use clap::Subcommand;

/// Research entity commands.
#[derive(Clone, Debug, Subcommand)]
pub enum ResearchCommands {
    /// Create a research item.
    Create {
        #[arg(long)]
        topic: String,
        #[arg(long)]
        question: String,
        #[arg(long)]
        context: Option<String>,
    },
    /// Update a research item.
    Update {
        id: String,
        #[arg(long)]
        topic: Option<String>,
        #[arg(long)]
        question: Option<String>,
        #[arg(long)]
        context: Option<String>,
        #[arg(long)]
        status: Option<String>,
    },
    /// List research items.
    List {
        #[arg(long)]
        status: Option<String>,
        #[arg(long)]
        search: Option<String>,
        #[arg(long)]
        limit: Option<u32>,
    },
    /// Get a research item by ID.
    Get { id: String },
    /// Query package registries.
    Registry {
        query: String,
        #[arg(long)]
        ecosystem: Option<String>,
        #[arg(long)]
        limit: Option<u32>,
    },
}
