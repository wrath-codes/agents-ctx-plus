use clap::Subcommand;

/// Compatibility check commands.
#[derive(Clone, Debug, Subcommand)]
pub enum CompatCommands {
    /// Create or update compatibility status for a package pair.
    Check {
        package_a: String,
        package_b: String,
        #[arg(long)]
        status: String,
        #[arg(long)]
        conditions: Option<String>,
        #[arg(long)]
        finding: Option<String>,
    },
    /// List compatibility records.
    List {
        #[arg(long)]
        status: Option<String>,
        #[arg(long)]
        package: Option<String>,
        #[arg(long)]
        limit: Option<u32>,
    },
    /// Get a compatibility record by ID.
    Get { id: String },
}
