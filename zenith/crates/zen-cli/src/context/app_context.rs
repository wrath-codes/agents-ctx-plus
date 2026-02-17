use std::path::PathBuf;

use anyhow::Context;
use zen_config::ZenConfig;
use zen_db::service::ZenService;
use zen_embeddings::EmbeddingEngine;
use zen_lake::{SourceFileStore, ZenLake};
use zen_registry::RegistryClient;

/// Shared application resources initialized once at startup.
pub struct AppContext {
    pub service: ZenService,
    pub config: ZenConfig,
    pub lake: ZenLake,
    pub source_store: SourceFileStore,
    pub embedder: EmbeddingEngine,
    pub registry: RegistryClient,
    pub project_root: PathBuf,
}

impl AppContext {
    /// Initialize all shared resources using the discovered project root.
    pub async fn init(project_root: PathBuf, config: ZenConfig) -> anyhow::Result<Self> {
        let zenith_dir = project_root.join(".zenith");
        let db_path = zenith_dir.join("zenith.db");
        let trail_dir = zenith_dir.join("trail");
        let lake_path = zenith_dir.join("lake.duckdb");
        let source_path = zenith_dir.join("source_files.duckdb");

        let db_path_str = db_path.to_string_lossy();
        let lake_path_str = lake_path.to_string_lossy();
        let source_path_str = source_path.to_string_lossy();

        let service = ZenService::new_local(&db_path_str, Some(trail_dir))
            .await
            .context("failed to initialize zen-db service")?;
        let lake = ZenLake::open_local(&lake_path_str).context("failed to open local zen lake")?;
        let source_store =
            SourceFileStore::open(&source_path_str).context("failed to open source file store")?;
        let embedder = EmbeddingEngine::new().context("failed to initialize embedding engine")?;
        let registry = RegistryClient::new();

        Ok(Self {
            service,
            config,
            lake,
            source_store,
            embedder,
            registry,
            project_root,
        })
    }
}
