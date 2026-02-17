use anyhow::bail;
use serde::Serialize;

use crate::cli::GlobalFlags;
use crate::output::output;

#[derive(Debug, Serialize)]
struct PreCommitResponse {
    valid: bool,
    files_checked: usize,
    operations_checked: usize,
    errors: Vec<zen_hooks::TrailValidationError>,
}

pub fn run(flags: &GlobalFlags) -> anyhow::Result<()> {
    let project_root = std::env::current_dir()?;
    let report = zen_hooks::validate_staged_trail_files(&project_root)?;
    let valid = report.is_valid();
    let response = PreCommitResponse {
        valid,
        files_checked: report.files_checked,
        operations_checked: report.operations_checked,
        errors: report.errors,
    };

    output(&response, flags.format)?;
    if !valid {
        bail!("hook pre-commit: trail validation failed");
    }
    Ok(())
}
