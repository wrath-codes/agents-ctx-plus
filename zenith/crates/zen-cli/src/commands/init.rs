use std::fs;
use std::path::Path;

use anyhow::Context;
use serde::Serialize;
use zen_db::service::ZenService;
use zen_lake::{SourceFileStore, ZenLake};

use crate::cli::GlobalFlags;
use crate::cli::root_commands::InitArgs;
use crate::output::output;

#[derive(Debug, Serialize)]
struct InitResponse {
    project: InitProject,
    dependencies: InitDependencies,
    session: InitSession,
    hooks: InitHooks,
}

#[derive(Debug, Serialize)]
struct InitProject {
    name: String,
    ecosystem: String,
    root_path: String,
    vcs: String,
}

#[derive(Debug, Serialize)]
struct InitDependencies {
    total: u32,
    indexed: u32,
}

#[derive(Debug, Serialize)]
struct InitSession {
    id: String,
    status: String,
}

#[derive(Debug, Serialize)]
struct InitHooks {
    installed: bool,
    note: String,
    warnings: Vec<String>,
}

/// Handle `znt init`.
pub async fn handle(args: &InitArgs, flags: &GlobalFlags) -> anyhow::Result<()> {
    let project_root = std::env::current_dir().context("failed to resolve current directory")?;
    let zenith_dir = project_root.join(".zenith");
    if zenith_dir.exists() {
        anyhow::bail!(
            "zenith already initialized at {}",
            zenith_dir.to_string_lossy()
        );
    }

    fs::create_dir_all(zenith_dir.join("trail"))?;
    fs::create_dir_all(zenith_dir.join("hooks"))?;

    let ecosystem = args
        .ecosystem
        .clone()
        .unwrap_or_else(|| detect_ecosystem(&project_root));
    let name = args.name.clone().unwrap_or_else(|| {
        detect_project_name(&project_root).unwrap_or_else(|| fallback_project_name(&project_root))
    });

    write_gitignore(&project_root)?;

    let service = ZenService::new_local(
        &zenith_dir.join("zenith.db").to_string_lossy(),
        Some(zenith_dir.join("trail")),
    )
    .await
    .context("failed to initialize zenith database")?;

    let _ = ZenLake::open_local(&zenith_dir.join("lake.duckdb").to_string_lossy())?;
    let _ = SourceFileStore::open(&zenith_dir.join("source_files.duckdb").to_string_lossy())?;

    service.set_meta("project_name", &name).await?;
    service.set_meta("ecosystem", &ecosystem).await?;
    service
        .set_meta("root_path", &project_root.to_string_lossy())
        .await?;
    service.set_meta("vcs", "git").await?;

    let (session, _) = service.start_session().await?;

    let hooks = if args.skip_hooks {
        InitHooks {
            installed: false,
            note: "hook installation skipped".to_string(),
            warnings: Vec::new(),
        }
    } else {
        match zen_hooks::install_hooks(&project_root, zen_hooks::HookInstallStrategy::Chain) {
            Ok(report) => InitHooks {
                installed: report.installed,
                note: if report.installed {
                    "hooks installed".to_string()
                } else {
                    "hooks partially installed".to_string()
                },
                warnings: report.warnings,
            },
            Err(error) => InitHooks {
                installed: false,
                note: format!("hook installation skipped: {error}"),
                warnings: vec![
                    "run `znt hook install --strategy chain` after resolving git hook conflicts"
                        .to_string(),
                ],
            },
        }
    };

    output(
        &InitResponse {
            project: InitProject {
                name,
                ecosystem,
                root_path: project_root.to_string_lossy().to_string(),
                vcs: "git".to_string(),
            },
            dependencies: InitDependencies {
                total: 0,
                indexed: 0,
            },
            session: InitSession {
                id: session.id,
                status: "active".to_string(),
            },
            hooks,
        },
        flags.format,
    )
}

fn detect_ecosystem(project_root: &Path) -> String {
    if project_root.join("Cargo.toml").is_file() {
        return "rust".to_string();
    }
    if project_root.join("package.json").is_file() {
        return "npm".to_string();
    }
    if project_root.join("pyproject.toml").is_file()
        || project_root.join("requirements.txt").is_file()
    {
        return "pypi".to_string();
    }
    if project_root.join("mix.exs").is_file() {
        return "hex".to_string();
    }
    if project_root.join("go.mod").is_file() {
        return "go".to_string();
    }
    if project_root.join("Gemfile").is_file() {
        return "ruby".to_string();
    }
    if project_root.join("composer.json").is_file() {
        return "php".to_string();
    }
    "rust".to_string()
}

fn detect_project_name(project_root: &Path) -> Option<String> {
    if let Ok(raw) = fs::read_to_string(project_root.join("package.json"))
        && let Ok(value) = serde_json::from_str::<serde_json::Value>(&raw)
        && let Some(name) = value.get("name").and_then(serde_json::Value::as_str)
    {
        return Some(name.to_string());
    }

    if let Ok(raw) = fs::read_to_string(project_root.join("Cargo.toml"))
        && let Ok(doc) = toml::from_str::<toml::Value>(&raw)
        && let Some(name) = doc
            .get("package")
            .and_then(toml::Value::as_table)
            .and_then(|pkg| pkg.get("name"))
            .and_then(toml::Value::as_str)
            .filter(|v| !v.trim().is_empty())
    {
        return Some(name.to_string());
    }

    if let Ok(raw) = fs::read_to_string(project_root.join("pyproject.toml"))
        && let Ok(doc) = toml::from_str::<toml::Value>(&raw)
    {
        if let Some(name) = doc
            .get("project")
            .and_then(toml::Value::as_table)
            .and_then(|project| project.get("name"))
            .and_then(toml::Value::as_str)
            .filter(|v| !v.trim().is_empty())
        {
            return Some(name.to_string());
        }

        if let Some(name) = doc
            .get("tool")
            .and_then(toml::Value::as_table)
            .and_then(|tool| tool.get("poetry"))
            .and_then(toml::Value::as_table)
            .and_then(|poetry| poetry.get("name"))
            .and_then(toml::Value::as_str)
            .filter(|v| !v.trim().is_empty())
        {
            return Some(name.to_string());
        }
    }

    None
}

fn fallback_project_name(project_root: &Path) -> String {
    project_root
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "zenith-project".to_string())
}

fn write_gitignore(project_root: &Path) -> anyhow::Result<()> {
    let path = project_root.join(".gitignore");
    let block = [
        "# Zenith generated files",
        ".zenith/zenith.db",
        ".zenith/lake.duckdb",
        ".zenith/source_files.duckdb",
        ".zenith/*.tmp",
    ];
    let mut existing = if path.exists() {
        fs::read_to_string(&path)?
    } else {
        String::new()
    };

    for entry in block {
        if !existing.lines().any(|line| line.trim() == entry) {
            if !existing.ends_with('\n') && !existing.is_empty() {
                existing.push('\n');
            }
            existing.push_str(entry);
            existing.push('\n');
        }
    }

    fs::write(path, existing)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::TempDir;

    use super::{detect_ecosystem, detect_project_name, write_gitignore};

    #[test]
    fn detects_rust_ecosystem_from_manifest() {
        let temp = TempDir::new().expect("tempdir");
        fs::write(
            temp.path().join("Cargo.toml"),
            "[package]\nname = \"demo\"\n",
        )
        .expect("write");
        assert_eq!(detect_ecosystem(temp.path()), "rust");
    }

    #[test]
    fn detects_project_name_from_package_json() {
        let temp = TempDir::new().expect("tempdir");
        fs::write(temp.path().join("package.json"), "{\"name\":\"web-app\"}").expect("write");
        assert_eq!(detect_project_name(temp.path()).as_deref(), Some("web-app"));
    }

    #[test]
    fn writes_gitignore_entries_idempotently() {
        let temp = TempDir::new().expect("tempdir");
        write_gitignore(temp.path()).expect("first write");
        write_gitignore(temp.path()).expect("second write");
        let contents = fs::read_to_string(temp.path().join(".gitignore")).expect("read");
        assert_eq!(
            contents
                .lines()
                .filter(|line| *line == ".zenith/zenith.db")
                .count(),
            1
        );
    }
}
