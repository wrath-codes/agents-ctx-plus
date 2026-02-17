use clap::Subcommand;

/// Study workflow commands.
#[derive(Clone, Debug, Subcommand)]
pub enum StudyCommands {
    /// Create a study.
    Create {
        #[arg(long)]
        topic: String,
        #[arg(long)]
        library: Option<String>,
        #[arg(long)]
        methodology: Option<String>,
        #[arg(long)]
        summary: Option<String>,
    },
    /// Add an assumption to a study.
    Assume {
        id: String,
        #[arg(long)]
        content: String,
        #[arg(long)]
        evidence: Option<String>,
    },
    /// Record a test result for an assumption.
    Test {
        id: String,
        assumption_id: String,
        #[arg(long)]
        result: String,
        #[arg(long)]
        evidence: Option<String>,
    },
    /// Get full study state.
    Get { id: String },
    /// Conclude a study.
    Conclude {
        id: String,
        #[arg(long)]
        summary: String,
    },
    /// List studies.
    List {
        #[arg(long)]
        status: Option<String>,
        #[arg(long)]
        library: Option<String>,
        #[arg(long)]
        limit: Option<u32>,
    },
}
