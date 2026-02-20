use anyhow::Context;
use serde::Serialize;
use zen_core::enums::Visibility;

use crate::cli::GlobalFlags;
use crate::cli::root_commands::IndexArgs;
use crate::context::AppContext;
use crate::output::output;
use crate::pipeline::IndexingPipeline;

#[derive(Debug, Serialize)]
struct IndexResponse {
    path: String,
    ecosystem: String,
    package: String,
    version: String,
    visibility: String,
    files_parsed: i32,
    symbols_extracted: i32,
    doc_chunks_created: i32,
    source_files_cached: i32,
    r2_exported: bool,
    catalog_registered: bool,
}

/// Handle `znt index .`.
pub async fn handle(
    args: &IndexArgs,
    ctx: &mut AppContext,
    flags: &GlobalFlags,
) -> anyhow::Result<()> {
    let identity = ctx.identity.as_ref().ok_or_else(|| {
        anyhow::anyhow!("private indexing requires authentication — run `znt auth login`")
    })?;
    let user_id = identity.user_id.clone();

    let project_root = if args.path == "." {
        ctx.project_root.clone()
    } else {
        std::path::PathBuf::from(&args.path)
            .canonicalize()
            .context("failed to resolve index path")?
    };

    let package = ctx
        .service
        .get_meta("name")
        .await?
        .or_else(|| {
            project_root
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
        })
        .unwrap_or_else(|| "unnamed".to_string());

    let ecosystem = "local".to_string();
    let version = chrono::Utc::now().format("%Y%m%d").to_string();

    let index = IndexingPipeline::index_directory_with(
        &ctx.lake,
        &ctx.source_store,
        &project_root,
        &ecosystem,
        &package,
        &version,
        &mut ctx.embedder,
        true,
    )
    .context("indexing pipeline failed")?;

    let mut r2_exported = false;
    let mut catalog_registered = false;

    if ctx.config.r2.is_configured() {
        match ctx
            .lake
            .write_to_r2(
                &ctx.config.r2,
                &ecosystem,
                &package,
                &version,
                Visibility::Private,
            )
            .await
        {
            Ok(export) => {
                r2_exported = true;
                if let Some(symbols_path) = export.symbols_lance_path.as_deref()
                    && ctx.service.is_synced_replica()
                {
                    match ctx
                        .service
                        .register_catalog_data_file(
                            &ecosystem,
                            &package,
                            &version,
                            symbols_path,
                            Visibility::Private,
                            None,
                            Some(&user_id),
                        )
                        .await
                    {
                        Ok(()) => catalog_registered = true,
                        Err(error) => tracing::warn!(
                            %error,
                            "index: failed to register private dataset in catalog"
                        ),
                    }
                }
            }
            Err(error) => tracing::warn!(
                %error,
                "index: failed to export private dataset to R2"
            ),
        }
    }

    output(
        &IndexResponse {
            path: project_root.to_string_lossy().to_string(),
            ecosystem,
            package,
            version,
            visibility: "private".to_string(),
            files_parsed: index.file_count,
            symbols_extracted: index.symbol_count,
            doc_chunks_created: index.doc_chunk_count,
            source_files_cached: index.source_file_count,
            r2_exported,
            catalog_registered,
        },
        flags.format,
    )
}
