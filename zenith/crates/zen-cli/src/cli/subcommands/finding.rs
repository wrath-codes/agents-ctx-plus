use clap::Subcommand;

/// Finding entity commands.
#[derive(Clone, Debug, Subcommand)]
pub enum FindingCommands {
    /// Create a finding.
    Create {
        #[arg(long)]
        content: String,
        #[arg(long)]
        source: Option<String>,
        #[arg(long)]
        confidence: Option<String>,
        #[arg(long)]
        research: Option<String>,
        #[arg(long)]
        tag: Vec<String>,
    },
    /// Update a finding.
    Update {
        id: String,
        #[arg(long)]
        content: Option<String>,
        #[arg(long)]
        source: Option<String>,
        #[arg(long)]
        confidence: Option<String>,
    },
    /// List findings.
    List {
        #[arg(long)]
        search: Option<String>,
        #[arg(long)]
        research: Option<String>,
        #[arg(long)]
        tag: Option<String>,
        #[arg(long)]
        limit: Option<u32>,
    },
    /// Get a finding by ID.
    Get { id: String },
    /// Add a tag.
    Tag { id: String, tag: String },
    /// Remove a tag.
    Untag { id: String, tag: String },
}
