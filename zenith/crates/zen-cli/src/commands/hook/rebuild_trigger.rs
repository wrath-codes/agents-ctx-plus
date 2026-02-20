use std::path::{Path, PathBuf};

use anyhow::Context;
use zen_db::service::ZenService;
use zen_db::trail::replayer::TrailReplayer;

pub async fn rebuild_from_default_trail(project_root: &Path, strict: bool) -> anyhow::Result<()> {
    let zenith_dir = project_root.join(".zenith");
    let trail_dir = zenith_dir.join("trail");
    let db_path = zenith_dir.join("zenith.db");
    reset_database_files(&db_path)?;

    let mut service = open_service(&db_path, &trail_dir).await?;
    let _ = TrailReplayer::rebuild(&mut service, &trail_dir, strict).await?;
    Ok(())
}

fn reset_database_files(db_path: &Path) -> anyhow::Result<()> {
    remove_if_exists(db_path)?;
    remove_if_exists(&PathBuf::from(format!("{}-wal", db_path.to_string_lossy())))?;
    remove_if_exists(&PathBuf::from(format!("{}-shm", db_path.to_string_lossy())))?;
    Ok(())
}

fn remove_if_exists(path: &Path) -> anyhow::Result<()> {
    if path.exists() {
        std::fs::remove_file(path)
            .with_context(|| format!("failed to remove file {}", path.to_string_lossy()))?;
    }
    Ok(())
}

async fn open_service(db_path: &Path, trail_dir: &Path) -> anyhow::Result<ZenService> {
    ZenService::new_local(&db_path.to_string_lossy(), Some(trail_dir.to_path_buf()), None)
        .await
        .context("failed to open zen service for rebuild")
}
