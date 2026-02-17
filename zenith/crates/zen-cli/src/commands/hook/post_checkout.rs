use serde::Serialize;

use crate::cli::GlobalFlags;
use crate::commands::hook::rebuild_trigger;
use crate::output::output;

#[derive(Debug, Serialize)]
struct PostCheckoutResponse {
    previous_head: Option<String>,
    new_head: Option<String>,
    branch_checkout: Option<bool>,
    action: String,
    changed_files: Vec<String>,
}

pub async fn run(
    old_head: Option<&str>,
    new_head: Option<&str>,
    is_branch_checkout: Option<&str>,
    flags: &GlobalFlags,
) -> anyhow::Result<()> {
    let project_root = std::env::current_dir()?;

    let old = old_head.unwrap_or("");
    let new = new_head.unwrap_or("");
    let action = zen_hooks::analyze_post_checkout(&project_root, old, new)?;

    let mut response = PostCheckoutResponse {
        previous_head: old_head.map(ToString::to_string),
        new_head: new_head.map(ToString::to_string),
        branch_checkout: is_branch_checkout.map(|v| v == "1"),
        action: "skip".to_string(),
        changed_files: Vec::new(),
    };

    match action {
        zen_hooks::PostCheckoutAction::Skip { reason } => {
            response.action = format!("skip: {reason}");
        }
        zen_hooks::PostCheckoutAction::Rebuild { changed_files } => {
            rebuild_trigger::rebuild_from_default_trail(&project_root, false).await?;
            response.action = "rebuild".to_string();
            response.changed_files = changed_files;
        }
    }

    output(&response, flags.format)
}
