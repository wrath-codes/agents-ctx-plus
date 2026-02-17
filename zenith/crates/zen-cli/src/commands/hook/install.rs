use std::path::Path;

use crate::cli::GlobalFlags;
use crate::output::output;

pub fn run(
    project_root: &Path,
    strategy: zen_hooks::HookInstallStrategy,
    flags: &GlobalFlags,
) -> anyhow::Result<()> {
    let report = zen_hooks::install_hooks(project_root, strategy)?;
    output(&report, flags.format)
}
