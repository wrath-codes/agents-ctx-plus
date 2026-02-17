use crate::cli::GlobalFlags;
use crate::output::output;

pub fn run(strategy: zen_hooks::HookInstallStrategy, flags: &GlobalFlags) -> anyhow::Result<()> {
    let project_root = std::env::current_dir()?;
    let report = zen_hooks::install_hooks(&project_root, strategy)?;
    output(&report, flags.format)
}
