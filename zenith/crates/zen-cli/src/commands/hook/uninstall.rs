use std::path::Path;

use crate::cli::GlobalFlags;
use crate::output::output;

pub fn run(project_root: &Path, flags: &GlobalFlags) -> anyhow::Result<()> {
    let report = zen_hooks::uninstall_hooks(project_root)?;
    output(&report, flags.format)
}
