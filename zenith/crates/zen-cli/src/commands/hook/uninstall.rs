use crate::cli::GlobalFlags;
use crate::output::output;

pub fn run(flags: &GlobalFlags) -> anyhow::Result<()> {
    let project_root = std::env::current_dir()?;
    let report = zen_hooks::uninstall_hooks(&project_root)?;
    output(&report, flags.format)
}
