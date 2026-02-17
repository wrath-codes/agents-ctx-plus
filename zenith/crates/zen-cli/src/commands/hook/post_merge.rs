use serde::Serialize;

use crate::cli::GlobalFlags;
use crate::commands::hook::rebuild_trigger;
use crate::output::output;

#[derive(Debug, Serialize)]
struct PostMergeResponse {
    squash: Option<bool>,
    action: String,
    changed_files: Vec<String>,
    conflict_files: Vec<String>,
}

pub async fn run(squash: Option<&str>, flags: &GlobalFlags) -> anyhow::Result<()> {
    let project_root = std::env::current_dir()?;
    let action = zen_hooks::analyze_post_merge(&project_root)?;

    let mut response = PostMergeResponse {
        squash: squash.map(|v| v == "1"),
        action: "skip".to_string(),
        changed_files: Vec::new(),
        conflict_files: Vec::new(),
    };

    match action {
        zen_hooks::PostMergeAction::Skip { reason } => {
            response.action = format!("skip: {reason}");
        }
        zen_hooks::PostMergeAction::Rebuild { changed_files } => {
            rebuild_trigger::rebuild_from_default_trail(&project_root, false).await?;
            response.action = "rebuild".to_string();
            response.changed_files = changed_files;
        }
        zen_hooks::PostMergeAction::ConflictDetected { files } => {
            response.action = "conflict".to_string();
            response.conflict_files = files;
        }
    }

    output(&response, flags.format)
}
