#[path = "hook/install.rs"]
mod install;
#[path = "hook/post_checkout.rs"]
mod post_checkout;
#[path = "hook/post_merge.rs"]
mod post_merge;
#[path = "hook/pre_commit.rs"]
mod pre_commit;
#[path = "hook/rebuild_trigger.rs"]
mod rebuild_trigger;
#[path = "hook/status.rs"]
mod status;
#[path = "hook/uninstall.rs"]
mod uninstall;

use crate::cli::GlobalFlags;
use crate::cli::subcommands::{HookCommands, HookInstallStrategyArg};

/// Handle `znt hook`.
pub async fn handle(action: &HookCommands, flags: &GlobalFlags) -> anyhow::Result<()> {
    let project_root = resolve_project_root(flags)?;

    match action {
        HookCommands::Install { strategy } => {
            let strategy = match strategy {
                HookInstallStrategyArg::Chain => zen_hooks::HookInstallStrategy::Chain,
                HookInstallStrategyArg::Refuse => zen_hooks::HookInstallStrategy::Refuse,
            };
            install::run(&project_root, strategy, flags)
        }
        HookCommands::Status => status::run(&project_root, flags),
        HookCommands::Uninstall => uninstall::run(&project_root, flags),
        HookCommands::PreCommit => pre_commit::run(&project_root, flags),
        HookCommands::PostCheckout {
            old_head,
            new_head,
            is_branch_checkout,
        } => {
            post_checkout::run(
                &project_root,
                old_head.as_deref(),
                new_head.as_deref(),
                is_branch_checkout.as_deref(),
                flags,
            )
            .await
        }
        HookCommands::PostMerge { squash } => {
            post_merge::run(&project_root, squash.as_deref(), flags).await
        }
    }
}

fn resolve_project_root(flags: &GlobalFlags) -> anyhow::Result<std::path::PathBuf> {
    match flags.project.as_deref() {
        Some(path) => Ok(std::path::PathBuf::from(path)),
        None => std::env::current_dir().map_err(Into::into),
    }
}
