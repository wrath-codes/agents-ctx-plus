use clap::{Subcommand, ValueEnum};

#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum HookInstallStrategyArg {
    Chain,
    Refuse,
}

/// Hook entrypoints called by git hook shell wrappers.
#[derive(Clone, Debug, Subcommand)]
pub enum HookCommands {
    /// Install Zenith hook scripts and wire git hooks.
    Install {
        #[arg(long, value_enum, default_value_t = HookInstallStrategyArg::Chain)]
        strategy: HookInstallStrategyArg,
    },
    /// Show hook installation status.
    Status,
    /// Remove Zenith-managed hook wiring.
    Uninstall,
    /// Validate staged `.zenith/trail/*.jsonl` files.
    #[command(name = "pre-commit")]
    PreCommit,
    /// React to checkout trail diffs.
    #[command(name = "post-checkout")]
    PostCheckout {
        old_head: Option<String>,
        new_head: Option<String>,
        is_branch_checkout: Option<String>,
    },
    /// React to merge trail updates.
    #[command(name = "post-merge")]
    PostMerge { squash: Option<String> },
}
