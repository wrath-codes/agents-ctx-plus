use clap::Subcommand;

/// Hook entrypoints called by git hook shell wrappers.
#[derive(Clone, Debug, Subcommand)]
pub enum HookCommands {
    /// Validate staged `.zenith/trail/*.jsonl` files.
    #[command(name = "pre-commit")]
    PreCommit,
    /// React to checkout trail diffs.
    #[command(name = "post-checkout")]
    PostCheckout,
    /// React to merge trail updates.
    #[command(name = "post-merge")]
    PostMerge,
}
