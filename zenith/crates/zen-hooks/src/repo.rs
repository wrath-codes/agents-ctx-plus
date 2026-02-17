use std::path::{Path, PathBuf};

use crate::error::HookError;

#[derive(Debug, Clone)]
pub struct RepoContext {
    pub root: PathBuf,
    pub git_dir: PathBuf,
    pub hooks_dir: PathBuf,
    pub zenith_hooks_dir: PathBuf,
    pub core_hooks_path: Option<String>,
}

pub fn discover_repo_context(project_root: &Path) -> Result<RepoContext, HookError> {
    let repo = gix::discover(project_root)
        .map_err(|_| HookError::NotGitRepo(project_root.to_path_buf()))?;
    let root = repo
        .work_dir()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| project_root.to_path_buf());
    let git_dir = repo.git_dir().to_path_buf();

    let core_hooks_path = repo
        .config_snapshot()
        .string("core.hooksPath")
        .map(|v| v.to_string());

    let hooks_dir = match core_hooks_path.as_deref() {
        Some(path) if !path.trim().is_empty() => {
            let configured = PathBuf::from(path);
            if configured.is_absolute() {
                configured
            } else {
                root.join(configured)
            }
        }
        _ => git_dir.join("hooks"),
    };

    Ok(RepoContext {
        root: root.clone(),
        git_dir,
        hooks_dir,
        zenith_hooks_dir: root.join(".zenith").join("hooks"),
        core_hooks_path,
    })
}
