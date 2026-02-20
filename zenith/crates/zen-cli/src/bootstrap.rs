use std::path::PathBuf;

use anyhow::Context;

use crate::cli::GlobalFlags;

pub async fn load_config(flags: &GlobalFlags) -> anyhow::Result<zen_config::ZenConfig> {
    load_project_dotenv(flags)?;

    let env_overrides = match zen_secrets::load_env_overrides().await {
        Ok(zen_secrets::SecretOverrides::Disabled) => Vec::new(),
        Ok(zen_secrets::SecretOverrides::Values(values)) => values,
        Err(error) => {
            if is_ci() {
                return Err(anyhow::anyhow!(
                    "failed to load configured secret backend in CI: {error}"
                ));
            }

            tracing::warn!(%error, "failed to load external secrets; continuing with local config");
            Vec::new()
        }
    };

    zen_config::ZenConfig::load_with_env_overrides(&env_overrides).map_err(anyhow::Error::from)
}

fn is_ci() -> bool {
    std::env::var("CI")
        .map(|value| value.eq_ignore_ascii_case("true"))
        .unwrap_or(false)
}

fn load_project_dotenv(flags: &GlobalFlags) -> anyhow::Result<()> {
    if let Some(project) = &flags.project {
        let project_path = PathBuf::from(project);
        let root = if project_path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name == ".zenith")
        {
            project_path
                .parent()
                .map(std::path::Path::to_path_buf)
                .unwrap_or(project_path.clone())
        } else {
            project_path
        };

        let env_path = root.join(".env");
        if env_path.exists() {
            dotenvy::from_path(&env_path)
                .with_context(|| format!("failed to load dotenv file at {}", env_path.display()))?;
            return Ok(());
        }
    }

    let cwd = std::env::current_dir().context("failed to determine current directory")?;

    // Monorepo-friendly fallback: if current directory has no local dotenv and
    // exactly one direct child Zenith project exists, prefer that child's dotenv.
    if !cwd.join(".env").exists()
        && let Some(child_root) = crate::context::find_single_child_project_root(&cwd)
    {
        let child_env = child_root.join(".env");
        if child_env.exists() {
            dotenvy::from_path(&child_env).with_context(|| {
                format!("failed to load dotenv file at {}", child_env.display())
            })?;
            return Ok(());
        }
    }

    if let Some(project_root) = crate::context::find_project_root_or_child(&cwd) {
        let env_path = project_root.join(".env");
        if env_path.exists() {
            dotenvy::from_path(&env_path)
                .with_context(|| format!("failed to load dotenv file at {}", env_path.display()))?;
            return Ok(());
        }
    }

    dotenvy::dotenv().ok();
    Ok(())
}
