use std::path::PathBuf;

use anyhow::Context;
use zen_config::ZenConfig;
use zen_core::identity::AuthIdentity;
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
pub identity: Option<AuthIdentity>,
pub auth_token: Option<String>,
}

impl AppContext {
    /// Initialize all shared resources using the discovered project root.
    pub async fn init(project_root: PathBuf, config: ZenConfig) -> anyhow::Result<Self> {
        let zenith_dir = project_root.join(".zenith");
        let db_path = zenith_dir.join("zenith.db");
        let synced_path = zenith_dir.join("zenith-synced.db");
        let trail_dir = zenith_dir.join("trail");
        let lake_path = zenith_dir.join("lake.duckdb");
        let source_path = zenith_dir.join("source_files.duckdb");

        let db_path_str = db_path.to_string_lossy();
        let synced_path_str = synced_path.to_string_lossy();
        let lake_path_str = lake_path.to_string_lossy();
        let source_path_str = source_path.to_string_lossy();

        // Resolve auth token (tiers 1-3 via zen-auth, tier 4 via config fallback)
        let (auth_token, identity) = resolve_auth(&config).await;

        let service = if config.turso.is_configured() {
            let replica_path: &str = if config.turso.has_local_replica() {
                &config.turso.local_replica_path
            } else {
                &synced_path_str
            };

            // Use auth-resolved token, fall back to config.turso.auth_token (tier 4)
            let token = auth_token
                .as_deref()
                .unwrap_or(&config.turso.auth_token);

            if token.is_empty() {
                ZenService::new_local(&db_path_str, Some(trail_dir), identity.clone())
                    .await
                    .context("failed to initialize zen-db service")?
            } else {
                match ZenService::new_synced(
                    replica_path,
                    &config.turso.url,
                    token,
                    Some(trail_dir.clone()),
                    identity.clone(),
                )
                .await
                {
                    Ok(service) => service,
                    Err(error) => {
                        tracing::warn!(
                            %error,
                            "failed to initialize synced zen-db service; falling back to local"
                        );
                        ZenService::new_local(&db_path_str, Some(trail_dir), identity.clone())
                            .await
                            .context("failed to initialize zen-db service")?
                    }
                }
            }
        } else {
            ZenService::new_local(&db_path_str, Some(trail_dir), identity.clone())
                .await
                .context("failed to initialize zen-db service")?
        };

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
            identity,
            auth_token,
        })
    }
}

/// Resolve auth token with optional JWKS validation.
///
/// Returns `(Option<raw_token>, Option<identity>)`.
/// - If `secret_key` is configured: validates via JWKS, extracts identity.
/// - If `secret_key` is empty: best-effort expiry check, no identity.
async fn resolve_auth(config: &ZenConfig) -> (Option<String>, Option<AuthIdentity>) {
    let secret_key = &config.clerk.secret_key;

    if secret_key.is_empty() {
        // No Clerk secret key — try raw token from zen-auth tiers or config fallback.
        // Cannot validate via JWKS, so identity is always None.
        let raw = zen_auth::resolve_token().or_else(|| {
            let t = &config.turso.auth_token;
            if t.is_empty() { None } else { Some(t.clone()) }
        });

        // Best-effort expiry check on unverified token
        if let Some(ref token) = raw {
            match zen_auth::refresh::decode_expiry(token) {
                Ok(expires_at) if expires_at <= chrono::Utc::now() => {
                    tracing::warn!("auth token appears expired — running in local mode");
                    return (None, None);
                }
                Ok(_) => {
                    // Only warn for JWT-shaped tokens (3 dot-separated segments).
                    // Platform API tokens are not JWTs and don't need this warning.
                    tracing::debug!(
                        "token found but ZENITH_CLERK__SECRET_KEY not configured — \
                         identity unavailable, expiry checks are best-effort"
                    );
                }
                Err(_) => {} // Not a JWT format — pass through as-is (e.g., Platform API token)
            }
        }

        return (raw, None);
    }

    match zen_auth::resolve_and_validate(secret_key).await {
        Ok(Some(claims)) => {
            let identity = claims.to_identity();
            (Some(claims.raw_jwt), Some(identity))
        }
        Ok(None) => {
            tracing::debug!("no Clerk auth token found via keyring/env/file; running in local mode");
            (None, None)
        }
        Err(error) => {
            tracing::warn!(%error, "auth token validation failed; running in local mode");
            (None, None)
        }
    }
}
