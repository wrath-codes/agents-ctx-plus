use clap::Subcommand;

/// Local cache management.
#[derive(Clone, Debug, Subcommand)]
pub enum CacheCommands {
    /// List indexed packages in local cache.
    List,
    /// Clean local cache.
    Clean {
        /// Package name to clean (if omitted, clean all with --all).
        #[arg(long)]
        package: Option<String>,
        /// Ecosystem for package clean.
        #[arg(long)]
        ecosystem: Option<String>,
        /// Version for package clean.
        #[arg(long)]
        version: Option<String>,
        /// Remove all cached data.
        #[arg(long)]
        all: bool,
    },
    /// Show cache statistics.
    Stats,
}
