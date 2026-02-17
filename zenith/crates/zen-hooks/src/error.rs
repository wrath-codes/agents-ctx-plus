use std::path::PathBuf;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum HookError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("git error: {0}")]
    Git(String),
    #[error("json parse error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("schema validation failed: {0}")]
    Schema(String),
    #[error("not a git repository: {0}")]
    NotGitRepo(PathBuf),
    #[error("hook conflict at '{path}': {reason}")]
    HookConflict { path: PathBuf, reason: String },
}
