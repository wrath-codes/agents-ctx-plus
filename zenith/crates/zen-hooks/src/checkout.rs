use std::path::Path;

use serde::Serialize;

use crate::error::HookError;

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum PostCheckoutAction {
    Skip { reason: String },
    Rebuild { changed_files: Vec<String> },
}

pub fn analyze_post_checkout(
    project_root: &Path,
    old_head: &str,
    new_head: &str,
) -> Result<PostCheckoutAction, HookError> {
    if old_head == new_head {
        return Ok(PostCheckoutAction::Skip {
            reason: "same commit".to_string(),
        });
    }

    let repo = gix::discover(project_root)
        .map_err(|_| HookError::NotGitRepo(project_root.to_path_buf()))?;

    let old_oid: gix::ObjectId = match old_head.parse() {
        Ok(v) => v,
        Err(_) => {
            return Ok(PostCheckoutAction::Skip {
                reason: "old HEAD is not a commit hash".to_string(),
            });
        }
    };
    let new_oid: gix::ObjectId = match new_head.parse() {
        Ok(v) => v,
        Err(_) => {
            return Ok(PostCheckoutAction::Skip {
                reason: "new HEAD is not a commit hash".to_string(),
            });
        }
    };

    let old_commit = match repo.find_commit(old_oid) {
        Ok(c) => c,
        Err(_) => {
            return Ok(PostCheckoutAction::Skip {
                reason: "old commit not found".to_string(),
            });
        }
    };
    let new_commit = match repo.find_commit(new_oid) {
        Ok(c) => c,
        Err(_) => {
            return Ok(PostCheckoutAction::Skip {
                reason: "new commit not found".to_string(),
            });
        }
    };

    let old_tree = old_commit
        .tree()
        .map_err(|e| HookError::Git(format!("load old tree: {e}")))?;
    let new_tree = new_commit
        .tree()
        .map_err(|e| HookError::Git(format!("load new tree: {e}")))?;

    let changes = repo
        .diff_tree_to_tree(Some(&old_tree), Some(&new_tree), None)
        .map_err(|e| HookError::Git(format!("diff tree to tree: {e}")))?;

    let changed_files = changes
        .iter()
        .filter_map(|change| {
            let path = change.location().to_string();
            if path.starts_with(".zenith/trail/") && path.ends_with(".jsonl") {
                Some(path)
            } else {
                None
            }
        })
        .collect::<Vec<_>>();

    if changed_files.is_empty() {
        Ok(PostCheckoutAction::Skip {
            reason: "no trail changes".to_string(),
        })
    } else {
        Ok(PostCheckoutAction::Rebuild { changed_files })
    }
}
