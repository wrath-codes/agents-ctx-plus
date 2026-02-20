use std::path::{Path, PathBuf};

use anyhow::Context;
use serde::Serialize;
use zen_db::service::ZenService;
use zen_db::trail::replayer::TrailReplayer;

use crate::cli::GlobalFlags;
use crate::cli::root_commands::RebuildArgs;
use crate::context::AppContext;
use crate::output::output;

#[derive(Debug, Serialize)]
struct RebuildDryRunResponse {
    dry_run: bool,
    strict: bool,
    trail_dir: String,
    trail_files: usize,
    operations_detected: usize,
}

pub async fn run(
    args: &RebuildArgs,
    ctx: &mut AppContext,
    flags: &GlobalFlags,
) -> anyhow::Result<()> {
    let zenith_dir = ctx.project_root.join(".zenith");
    let trail_dir = args
        .trail_dir
        .as_ref()
        .map(PathBuf::from)
        .unwrap_or_else(|| zenith_dir.join("trail"));
    let db_path = zenith_dir.join("zenith.db");

    if args.dry_run {
        let (trail_files, operations_detected) = count_trail_files_and_ops(&trail_dir)?;
        return output(
            &RebuildDryRunResponse {
                dry_run: true,
                strict: args.strict,
                trail_dir: trail_dir.to_string_lossy().to_string(),
                trail_files,
                operations_detected,
            },
            flags.format,
        );
    }

    remove_if_exists(&db_path)?;
    remove_if_exists(&PathBuf::from(format!("{}-wal", db_path.to_string_lossy())))?;
    remove_if_exists(&PathBuf::from(format!("{}-shm", db_path.to_string_lossy())))?;

    let mut service = ZenService::new_local(&db_path.to_string_lossy(), Some(trail_dir.clone()), None)
        .await
        .context("rebuild: failed to initialize zen service")?;

    let response = TrailReplayer::rebuild(&mut service, &trail_dir, args.strict)
        .await
        .context("rebuild: failed to replay trail files")?;

    output(&response, flags.format)
}

fn count_trail_files_and_ops(trail_dir: &Path) -> anyhow::Result<(usize, usize)> {
    let mut files = 0usize;
    let mut ops = 0usize;
    for entry in std::fs::read_dir(trail_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|v| v.to_str()) != Some("jsonl") {
            continue;
        }
        files += 1;
        let content = std::fs::read_to_string(path)?;
        ops += content
            .lines()
            .filter(|line| !line.trim().is_empty())
            .count();
    }
    Ok((files, ops))
}

fn remove_if_exists(path: &Path) -> anyhow::Result<()> {
    if path.exists() {
        std::fs::remove_file(path)
            .with_context(|| format!("rebuild: failed to remove {}", path.to_string_lossy()))?;
    }
    Ok(())
}
