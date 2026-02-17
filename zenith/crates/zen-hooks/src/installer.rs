use std::fs;
use std::path::Path;

use serde::Serialize;

use crate::error::HookError;
use crate::repo::{RepoContext, discover_repo_context};
use crate::scripts::write_default_scripts;

const HOOK_NAMES: [&str; 3] = ["pre-commit", "post-checkout", "post-merge"];
const ZENITH_CHAIN_MARKER: &str = "# Zenith managed hook (chain)";

#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum HookInstallStrategy {
    Chain,
    Refuse,
}

#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum HookInstallMode {
    Symlink,
    Copy,
    Chain,
    None,
    Mixed,
}

#[derive(Debug, Serialize)]
pub struct HookInstallationReport {
    pub installed: bool,
    pub mode: HookInstallMode,
    pub hooks_installed: Vec<String>,
    pub hooks_skipped: Vec<String>,
    pub warnings: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct HookStatus {
    pub name: String,
    pub script_exists: bool,
    pub script_executable: bool,
    pub git_hook_exists: bool,
    pub git_hook_type: String,
    pub git_hook_points_to: Option<String>,
    pub wired: bool,
    pub status: String,
    pub message: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct HookStatusReport {
    pub project: HookStatusProject,
    pub installation: HookStatusInstallation,
    pub git: HookStatusGit,
    pub hooks: Vec<HookStatus>,
    pub summary: HookStatusSummary,
    pub next_steps: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct HookStatusProject {
    pub root: String,
    pub is_git_repo: bool,
    pub zenith_dir: String,
    pub hooks_dir: String,
}

#[derive(Debug, Serialize)]
pub struct HookStatusInstallation {
    pub installed: bool,
    pub mode: HookInstallMode,
    pub health: String,
    pub issues: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct HookStatusGit {
    pub git_hooks_dir: String,
    pub core_hooks_path: Option<String>,
    pub core_hooks_path_conflict: bool,
}

#[derive(Debug, Serialize)]
pub struct HookStatusSummary {
    pub required_hooks: usize,
    pub ok: usize,
    pub missing: usize,
    pub miswired: usize,
    pub warnings: usize,
}

pub fn install_hooks(
    project_root: &Path,
    strategy: HookInstallStrategy,
) -> Result<HookInstallationReport, HookError> {
    let repo = discover_repo_context(project_root)?;
    write_default_scripts(&repo.zenith_hooks_dir)?;
    fs::create_dir_all(&repo.hooks_dir)?;

    let mut warnings = Vec::new();
    if let Some(path) = &repo.core_hooks_path {
        warnings.push(format!("core.hooksPath is set to '{path}'"));
    }

    let mut hooks_installed = Vec::new();
    let mut hooks_skipped = Vec::new();
    let mut modes = Vec::new();

    for hook in HOOK_NAMES {
        let source = repo.zenith_hooks_dir.join(hook);
        let target = repo.hooks_dir.join(hook);
        match install_single_hook(&repo, hook, &source, &target, strategy)? {
            Some(mode) => {
                hooks_installed.push(hook.to_string());
                modes.push(mode);
            }
            None => hooks_skipped.push(hook.to_string()),
        }
    }

    let mode = aggregate_mode(&modes);
    Ok(HookInstallationReport {
        installed: hooks_skipped.is_empty(),
        mode,
        hooks_installed,
        hooks_skipped,
        warnings,
    })
}

pub fn uninstall_hooks(project_root: &Path) -> Result<HookInstallationReport, HookError> {
    let repo = discover_repo_context(project_root)?;
    let mut hooks_installed = Vec::new();
    let mut hooks_skipped = Vec::new();

    for hook in HOOK_NAMES {
        let target = repo.hooks_dir.join(hook);
        let backup = repo.hooks_dir.join(format!("{hook}.user"));

        if target.exists() {
            if is_symlink_to_zenith_hook(&target, hook)? {
                fs::remove_file(&target)?;
                hooks_installed.push(hook.to_string());
                continue;
            }

            let content = fs::read_to_string(&target).unwrap_or_default();
            if content.contains(ZENITH_CHAIN_MARKER) {
                fs::remove_file(&target)?;
                if backup.exists() {
                    fs::rename(&backup, &target)?;
                }
                hooks_installed.push(hook.to_string());
                continue;
            }

            hooks_skipped.push(hook.to_string());
            continue;
        }

        if backup.exists() {
            hooks_skipped.push(hook.to_string());
        }
    }

    Ok(HookInstallationReport {
        installed: hooks_skipped.is_empty(),
        mode: HookInstallMode::None,
        hooks_installed,
        hooks_skipped,
        warnings: Vec::new(),
    })
}

pub fn status_hooks(project_root: &Path) -> Result<HookStatusReport, HookError> {
    let repo = discover_repo_context(project_root)?;

    let mut hooks = Vec::new();
    let mut ok = 0usize;
    let mut issues = Vec::new();
    let mut modes = Vec::new();
    let mut missing = 0usize;
    let mut miswired = 0usize;
    let mut warning_count = 0usize;

    for hook in HOOK_NAMES {
        let status = status_for_hook(&repo, hook)?;
        if status.wired {
            ok += 1;
        }
        match status.status.as_str() {
            "missing_script" | "missing_git_hook" => missing += 1,
            "miswired" => miswired += 1,
            "ok" => {}
            _ => warning_count += 1,
        }
        if status.status != "ok" {
            issues.push(format!("hook {hook}: {}", status.status));
        }
        modes.push(status.git_hook_type.clone());
        hooks.push(status);
    }

    let installation_mode = if modes.iter().all(|m| m == "symlink") {
        HookInstallMode::Symlink
    } else if modes.iter().all(|m| m == "copy") {
        HookInstallMode::Copy
    } else if modes.iter().all(|m| m == "chain") {
        HookInstallMode::Chain
    } else if modes.iter().all(|m| m == "missing") {
        HookInstallMode::None
    } else {
        HookInstallMode::Mixed
    };

    let installation_health = if ok == HOOK_NAMES.len() {
        "ok".to_string()
    } else if ok == 0 {
        "not_installed".to_string()
    } else {
        "degraded".to_string()
    };

    let mut next_steps = Vec::new();
    if ok != HOOK_NAMES.len() {
        next_steps.push("run: znt hook install --strategy chain".to_string());
    }

    Ok(HookStatusReport {
        project: HookStatusProject {
            root: repo.root.to_string_lossy().to_string(),
            is_git_repo: true,
            zenith_dir: repo.root.join(".zenith").to_string_lossy().to_string(),
            hooks_dir: repo.zenith_hooks_dir.to_string_lossy().to_string(),
        },
        installation: HookStatusInstallation {
            installed: ok == HOOK_NAMES.len(),
            mode: installation_mode,
            health: installation_health,
            issues,
        },
        git: HookStatusGit {
            git_hooks_dir: repo.hooks_dir.to_string_lossy().to_string(),
            core_hooks_path: repo.core_hooks_path.clone(),
            core_hooks_path_conflict: false,
        },
        hooks,
        summary: HookStatusSummary {
            required_hooks: HOOK_NAMES.len(),
            ok,
            missing,
            miswired,
            warnings: warning_count,
        },
        next_steps,
    })
}

fn status_for_hook(repo: &RepoContext, hook: &str) -> Result<HookStatus, HookError> {
    let script = repo.zenith_hooks_dir.join(hook);
    let target = repo.hooks_dir.join(hook);
    let script_exists = script.is_file();
    let script_executable = is_executable(&script)?;
    let git_hook_exists = target.exists();

    let (git_hook_type, points_to, wired, status, message) = if !git_hook_exists {
        (
            "missing".to_string(),
            None,
            false,
            "missing_git_hook".to_string(),
            Some("hook file not installed in git hooks directory".to_string()),
        )
    } else if is_symlink_to_zenith_hook(&target, hook)? {
        (
            "symlink".to_string(),
            Some(format!(".zenith/hooks/{hook}")),
            true,
            "ok".to_string(),
            None,
        )
    } else {
        let content = fs::read_to_string(&target).unwrap_or_default();
        if content.contains(ZENITH_CHAIN_MARKER) {
            ("chain".to_string(), None, true, "ok".to_string(), None)
        } else if content.contains("znt hook") {
            ("copy".to_string(), None, true, "ok".to_string(), None)
        } else {
            (
                "other".to_string(),
                None,
                false,
                "miswired".to_string(),
                Some("existing hook is not managed by zenith".to_string()),
            )
        }
    };

    let final_status = if !script_exists {
        "missing_script".to_string()
    } else if !script_executable {
        "not_executable".to_string()
    } else {
        status
    };

    Ok(HookStatus {
        name: hook.to_string(),
        script_exists,
        script_executable,
        git_hook_exists,
        git_hook_type,
        git_hook_points_to: points_to,
        wired,
        status: final_status,
        message,
    })
}

fn install_single_hook(
    _repo: &RepoContext,
    hook_name: &str,
    source: &Path,
    target: &Path,
    strategy: HookInstallStrategy,
) -> Result<Option<HookInstallMode>, HookError> {
    if !target.exists() {
        return install_fresh_hook(source, target);
    }

    if is_symlink_to_zenith_hook(target, hook_name)? {
        return Ok(Some(HookInstallMode::Symlink));
    }

    let content = fs::read_to_string(target).unwrap_or_default();
    if content.contains(ZENITH_CHAIN_MARKER) {
        return Ok(Some(HookInstallMode::Chain));
    }

    match strategy {
        HookInstallStrategy::Refuse => Ok(None),
        HookInstallStrategy::Chain => {
            install_chain_wrapper(target, hook_name)?;
            Ok(Some(HookInstallMode::Chain))
        }
    }
}

fn install_fresh_hook(source: &Path, target: &Path) -> Result<Option<HookInstallMode>, HookError> {
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(source, target)?;
        return Ok(Some(HookInstallMode::Symlink));
    }
    #[cfg(not(unix))]
    {
        fs::copy(source, target)?;
        return Ok(Some(HookInstallMode::Copy));
    }
}

fn install_chain_wrapper(target: &Path, hook_name: &str) -> Result<(), HookError> {
    let backup = target
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join(format!("{hook_name}.user"));
    if !backup.exists() {
        fs::rename(target, &backup)?;
    }

    let wrapper = format!(
        "#!/bin/bash\n{ZENITH_CHAIN_MARKER}\nif [ -x \"$(dirname \"$0\")/{hook_name}.user\" ]; then\n    \"$(dirname \"$0\")/{hook_name}.user\" \"$@\" || exit $?\nfi\nexec znt hook {hook_name} \"$@\"\n"
    );
    fs::write(target, wrapper)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(target)?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(target, perms)?;
    }
    Ok(())
}

fn is_executable(path: &Path) -> Result<bool, HookError> {
    if !path.exists() {
        return Ok(false);
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = fs::metadata(path)?.permissions().mode();
        return Ok(mode & 0o111 != 0);
    }
    #[cfg(not(unix))]
    {
        return Ok(true);
    }
}

fn is_symlink_to_zenith_hook(target: &Path, hook_name: &str) -> Result<bool, HookError> {
    if !target.exists() {
        return Ok(false);
    }

    #[cfg(unix)]
    {
        if target.symlink_metadata()?.file_type().is_symlink() {
            let link = fs::read_link(target)?;
            let link_text = link.to_string_lossy();
            let windows_path = format!(".zenith/hooks/{hook_name}").replace('/', "\\");
            return Ok(link_text.ends_with(&format!(".zenith/hooks/{hook_name}"))
                || link_text.ends_with(&windows_path));
        }
    }

    Ok(false)
}

fn aggregate_mode(modes: &[HookInstallMode]) -> HookInstallMode {
    if modes.is_empty() {
        return HookInstallMode::None;
    }
    let first = modes[0];
    if modes.iter().all(|m| *m == first) {
        first
    } else {
        HookInstallMode::Mixed
    }
}
