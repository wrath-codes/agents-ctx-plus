use std::fs;
use std::path::Path;
use std::process::Command;

use serde::Serialize;

use crate::error::HookError;

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum PostMergeAction {
    Skip { reason: String },
    Rebuild { changed_files: Vec<String> },
    ConflictDetected { files: Vec<String> },
}

pub fn analyze_post_merge(project_root: &Path) -> Result<PostMergeAction, HookError> {
    let trail_dir = project_root.join(".zenith").join("trail");
    if !trail_dir.exists() {
        return Ok(PostMergeAction::Skip {
            reason: "trail directory not found".to_string(),
        });
    }

    let mut conflicts = Vec::new();
    for entry in fs::read_dir(&trail_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|v| v.to_str()) != Some("jsonl") {
            continue;
        }
        let content = fs::read_to_string(&path)?;
        if content.contains("<<<<<<<") || content.contains("=======") || content.contains(">>>>>>>")
        {
            conflicts.push(path.to_string_lossy().to_string());
        }
    }
    if !conflicts.is_empty() {
        return Ok(PostMergeAction::ConflictDetected { files: conflicts });
    }

    let output = Command::new("git")
        .args([
            "diff",
            "--name-only",
            "HEAD~1",
            "HEAD",
            "--",
            ".zenith/trail/",
        ])
        .current_dir(project_root)
        .output()
        .map_err(|e| HookError::Git(format!("run git diff: {e}")))?;
    if !output.status.success() {
        return Err(HookError::Git(
            String::from_utf8_lossy(&output.stderr).to_string(),
        ));
    }

    let changed_files = String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(ToString::to_string)
        .collect::<Vec<_>>();

    if changed_files.is_empty() {
        Ok(PostMergeAction::Skip {
            reason: "no trail changes".to_string(),
        })
    } else {
        Ok(PostMergeAction::Rebuild { changed_files })
    }
}
