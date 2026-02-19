use clap::Subcommand;

/// PRD (Product Requirements Document) workflow commands.
#[derive(Clone, Debug, Subcommand)]
pub enum PrdCommands {
    /// Create a new PRD (creates an epic issue).
    Create {
        #[arg(long)]
        title: String,
        #[arg(long)]
        description: Option<String>,
    },
    /// Update PRD content (sets epic description).
    Update {
        /// Epic issue ID.
        id: String,
        /// PRD markdown content.
        #[arg(long)]
        content: String,
    },
    /// Get full PRD with tasks, progress, findings, and open questions.
    Get {
        /// Epic issue ID.
        id: String,
    },
    /// Generate parent tasks for a PRD.
    Tasks {
        /// Epic issue ID.
        id: String,
        /// JSON array of task titles.
        #[arg(long)]
        tasks: String,
    },
    /// Generate sub-tasks for a parent task.
    Subtasks {
        /// Parent task ID.
        id: String,
        /// Epic issue ID.
        #[arg(long)]
        epic: String,
        /// JSON array of sub-task titles.
        #[arg(long)]
        tasks: String,
    },
    /// Mark a PRD as completed.
    Complete {
        /// Epic issue ID.
        id: String,
    },
    /// List all PRDs (epic issues).
    List {
        #[arg(long)]
        status: Option<String>,
        #[arg(long)]
        search: Option<String>,
        #[arg(long)]
        limit: Option<u32>,
    },
}
