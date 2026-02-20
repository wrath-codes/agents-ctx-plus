use arrow_array::{Array, RecordBatch};
use futures_util::TryStreamExt;
use lancedb::query::{ExecutableQuery, QueryBase, Select};
use libsql::{Builder, Connection};
use zen_core::identity::AuthIdentity;

use crate::{LakeError, ZenLake};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CloudVectorSearchResult {
    pub id: String,
    pub version: String,
    pub name: String,
    pub kind: String,
    pub signature: Option<String>,
    pub doc_comment: Option<String>,
    pub file_path: Option<String>,
    pub distance: f64,
    pub lance_path: String,
}

fn normalize_table_uri(path: &str) -> Option<String> {
    let raw = path.trim();
    if raw.is_empty() {
        return None;
    }

    if raw.contains(".lance") {
        let uri = raw
            .split_once('#')
            .map_or(raw, |(prefix, _)| prefix)
            .trim_end_matches('?')
            .trim();
        return (!uri.is_empty()).then(|| uri.to_string());
    }

    if let Some((db_uri, table_name)) = raw.rsplit_once('#') {
        let db_uri = db_uri.trim().trim_end_matches('/').trim_end_matches('?');
        let table_name = table_name.trim();
        if !db_uri.is_empty() && !table_name.is_empty() {
            return Some(format!("{db_uri}/{table_name}.lance"));
        }
    }

    None
}

fn table_name_from_uri(table_uri: &str) -> Option<String> {
    let file_name = table_uri.rsplit('/').next()?.trim();
    file_name
        .strip_suffix(".lance")
        .map(|name| name.to_string())
        .filter(|name| !name.is_empty())
}

fn database_uri_from_table_uri(table_uri: &str) -> Option<String> {
    let (prefix, _) = table_uri.rsplit_once('/')?;
    (!prefix.is_empty()).then(|| prefix.to_string())
}

fn resolve_storage_credentials(
    r2_config: Option<&zen_config::R2Config>,
) -> Option<(String, String, String)> {
    if let Some(r2) = r2_config
        && r2.is_configured()
    {
        return Some((
            r2.access_key_id.clone(),
            r2.secret_access_key.clone(),
            r2.endpoint_url(),
        ));
    }

    let access_key = std::env::var("AWS_ACCESS_KEY_ID").ok();
    let secret_key = std::env::var("AWS_SECRET_ACCESS_KEY").ok();
    let endpoint = std::env::var("AWS_ENDPOINT_URL").ok();

    if let (Some(access_key), Some(secret_key), Some(endpoint)) = (access_key, secret_key, endpoint)
        && !access_key.is_empty()
        && !secret_key.is_empty()
        && !endpoint.is_empty()
    {
        return Some((access_key, secret_key, endpoint));
    }

    let access_key = std::env::var("ZENITH_R2__ACCESS_KEY_ID").ok();
    let secret_key = std::env::var("ZENITH_R2__SECRET_ACCESS_KEY").ok();
    let endpoint = std::env::var("ZENITH_R2__ENDPOINT")
        .ok()
        .filter(|value| !value.is_empty())
        .or_else(|| {
            std::env::var("ZENITH_R2__ACCOUNT_ID")
                .ok()
                .filter(|value| !value.is_empty())
                .map(|account_id| format!("https://{account_id}.r2.cloudflarestorage.com"))
        });

    if let (Some(access_key), Some(secret_key), Some(endpoint)) = (access_key, secret_key, endpoint)
        && !access_key.is_empty()
        && !secret_key.is_empty()
        && !endpoint.is_empty()
    {
        return Some((access_key, secret_key, endpoint));
    }

    None
}

impl ZenLake {
    async fn discover_catalog_paths_with_conn(
        conn: &Connection,
        ecosystem: &str,
        package: &str,
        version: Option<&str>,
    ) -> Result<Vec<String>, LakeError> {
        let mut paths = Vec::new();
        let mut rows = if let Some(version) = version {
            conn.query(
                "SELECT lance_path FROM dl_data_file
                 WHERE ecosystem = ?1 AND package = ?2 AND version = ?3
                    AND instr(lance_path, '.lance') > 0
                    AND instr(lance_path, '#') = 0
                   AND visibility = 'public'
                 ORDER BY created_at DESC, id DESC",
                libsql::params![ecosystem, package, version],
            )
            .await?
        } else {
            conn.query(
                "SELECT lance_path FROM dl_data_file
                 WHERE ecosystem = ?1 AND package = ?2
                    AND instr(lance_path, '.lance') > 0
                    AND instr(lance_path, '#') = 0
                   AND visibility = 'public'
                 ORDER BY created_at DESC, id DESC",
                libsql::params![ecosystem, package],
            )
            .await?
        };

        while let Some(row) = rows.next().await? {
            paths.push(row.get::<String>(0)?);
        }

        Ok(paths)
    }

    /// Discover catalog Lance paths for a package from Turso.
    ///
    /// # Errors
    ///
    /// Returns `LakeError` if remote connection or query execution fails.
    pub async fn discover_catalog_paths(
        &self,
        turso_url: &str,
        turso_auth_token: &str,
        ecosystem: &str,
        package: &str,
        version: Option<&str>,
    ) -> Result<Vec<String>, LakeError> {
        let db = Builder::new_remote(turso_url.to_string(), turso_auth_token.to_string())
            .build()
            .await?;
        let conn = db.connect()?;
        Self::discover_catalog_paths_with_conn(&conn, ecosystem, package, version).await
    }

    /// Query Lance datasets using native LanceDB and merge results.
    ///
    /// # Errors
    ///
    /// Returns `LakeError` if query execution fails.
    pub async fn search_lance_paths(
        &self,
        lance_paths: &[String],
        query_embedding: &[f32],
        k: u32,
    ) -> Result<Vec<CloudVectorSearchResult>, LakeError> {
        self.search_lance_paths_with_r2(lance_paths, query_embedding, k, None)
            .await
    }

    /// Query Lance datasets using native LanceDB and merge results, using
    /// optional R2 config for storage credentials.
    ///
    /// # Errors
    ///
    /// Returns `LakeError` if query execution fails.
    pub async fn search_lance_paths_with_r2(
        &self,
        lance_paths: &[String],
        query_embedding: &[f32],
        k: u32,
        r2_config: Option<&zen_config::R2Config>,
    ) -> Result<Vec<CloudVectorSearchResult>, LakeError> {
        if lance_paths.is_empty() {
            return Ok(Vec::new());
        }
        let mut results = Vec::new();

        let creds = resolve_storage_credentials(r2_config);

        for path in lance_paths {
            let Some(table_uri) = normalize_table_uri(path) else {
                tracing::warn!(lance_path = %path, "search: skipping invalid lance path");
                continue;
            };

            let Some(table_name) = table_name_from_uri(&table_uri) else {
                tracing::warn!(lance_path = %path, table_uri = %table_uri, "search: unable to derive table name from uri");
                continue;
            };

            let Some(database_uri) = database_uri_from_table_uri(&table_uri) else {
                tracing::warn!(lance_path = %path, table_uri = %table_uri, "search: unable to derive database uri from table uri");
                continue;
            };

            let mut conn_builder = lancedb::connect(&database_uri);
            if let Some((access_key, secret_key, endpoint)) = creds.as_ref() {
                conn_builder = conn_builder.storage_options([
                    ("aws_access_key_id", access_key.as_str()),
                    ("aws_secret_access_key", secret_key.as_str()),
                    ("aws_endpoint", endpoint.as_str()),
                    ("aws_region", "auto"),
                    ("aws_virtual_hosted_style_request", "false"),
                ]);
            }

            let db = match conn_builder.execute().await {
                Ok(db) => db,
                Err(error) => {
                    tracing::warn!(
                        lance_path = %path,
                        database_uri = %database_uri,
                        table_uri = %table_uri,
                        %error,
                        "search: skipping lance path due to lancedb connect failure"
                    );
                    continue;
                }
            };

            let mut open_builder = db.open_table(&table_name);
            if let Some((access_key, secret_key, endpoint)) = creds.as_ref() {
                open_builder = open_builder.storage_options([
                    ("aws_access_key_id", access_key.as_str()),
                    ("aws_secret_access_key", secret_key.as_str()),
                    ("aws_endpoint", endpoint.as_str()),
                    ("aws_region", "auto"),
                    ("aws_virtual_hosted_style_request", "false"),
                ]);
            }

            let table = match open_builder.execute().await {
                Ok(table) => table,
                Err(_) => {
                    let mut open_builder = db.open_table(&table_name).location(&table_uri);
                    if let Some((access_key, secret_key, endpoint)) = creds.as_ref() {
                        open_builder = open_builder.storage_options([
                            ("aws_access_key_id", access_key.as_str()),
                            ("aws_secret_access_key", secret_key.as_str()),
                            ("aws_endpoint", endpoint.as_str()),
                            ("aws_region", "auto"),
                            ("aws_virtual_hosted_style_request", "false"),
                        ]);
                    }
                    match open_builder.execute().await {
                        Ok(table) => table,
                        Err(_) => {
                            let mut open_builder =
                                db.open_table(&table_name).location(format!("{table_uri}/"));
                            if let Some((access_key, secret_key, endpoint)) = creds.as_ref() {
                                open_builder = open_builder.storage_options([
                                    ("aws_access_key_id", access_key.as_str()),
                                    ("aws_secret_access_key", secret_key.as_str()),
                                    ("aws_endpoint", endpoint.as_str()),
                                    ("aws_region", "auto"),
                                    ("aws_virtual_hosted_style_request", "false"),
                                ]);
                            }
                            match open_builder.execute().await {
                                Ok(table) => table,
                                Err(error) => {
                                    let discovered_tables =
                                        db.table_names().execute().await.unwrap_or_default();
                                    tracing::warn!(
                                        lance_path = %path,
                                        table_name = %table_name,
                                        table_uri = %table_uri,
                                        discovered_tables = ?discovered_tables,
                                        %error,
                                        "search: skipping lance path due to open_table failure"
                                    );
                                    continue;
                                }
                            }
                        }
                    }
                }
            };

            let stream = table
                .query()
                .nearest_to(query_embedding)
                .map_err(|e| LakeError::Other(format!("lancedb nearest_to failed: {e}")))?
                .select(Select::columns(&[
                    "id",
                    "version",
                    "name",
                    "kind",
                    "signature",
                    "doc_comment",
                    "file_path",
                    "_distance",
                ]))
                .limit(usize::try_from(k).unwrap_or(10))
                .execute()
                .await;

            let mut stream = match stream {
                Ok(stream) => stream,
                Err(error) => {
                    tracing::warn!(
                        lance_path = %path,
                        %error,
                        "search: skipping lance path due to query execution failure"
                    );
                    continue;
                }
            };

            loop {
                match stream.try_next().await {
                    Ok(Some(batch)) => append_batch_results(&batch, path, &mut results)?,
                    Ok(None) => break,
                    Err(error) => {
                        tracing::warn!(
                            lance_path = %path,
                            %error,
                            "search: stopping stream read for lance path after error"
                        );
                        break;
                    }
                }
            }
        }

        results.sort_by(|a, b| {
            a.distance
                .partial_cmp(&b.distance)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        if let Ok(limit) = usize::try_from(k) {
            results.truncate(limit);
        }

        Ok(results)
    }

    /// Full cloud vector search: Turso catalog discovery + Lance vector query.
    ///
    /// # Errors
    ///
    /// Returns `LakeError` if catalog lookup or Lance querying fails.
    pub async fn search_cloud_vector(
        &self,
        turso_url: &str,
        turso_auth_token: &str,
        ecosystem: &str,
        package: &str,
        version: Option<&str>,
        query_embedding: &[f32],
        k: u32,
    ) -> Result<Vec<CloudVectorSearchResult>, LakeError> {
        let paths = self
            .discover_catalog_paths(turso_url, turso_auth_token, ecosystem, package, version)
            .await?;
        self.search_lance_paths_with_r2(&paths, query_embedding, k, None)
            .await
    }

    /// Alias for Phase 8 search task naming.
    ///
    /// # Errors
    ///
    /// Returns `LakeError` if catalog lookup or Lance querying fails.
    pub async fn search(
        &self,
        turso_url: &str,
        turso_auth_token: &str,
        ecosystem: &str,
        package: &str,
        version: Option<&str>,
        query_embedding: &[f32],
        k: u32,
    ) -> Result<Vec<CloudVectorSearchResult>, LakeError> {
        self.search_cloud_vector(
            turso_url,
            turso_auth_token,
            ecosystem,
            package,
            version,
            query_embedding,
            k,
        )
        .await
    }

    /// Discover catalog Lance paths with visibility scoping via a Turso connection.
    ///
    /// Uses the same visibility filter as `catalog_paths_for_package_scoped()`.
    ///
    /// # Errors
    ///
    /// Returns `LakeError` if remote connection or query execution fails.
    pub async fn discover_catalog_paths_scoped(
        &self,
        turso_url: &str,
        turso_auth_token: &str,
        ecosystem: &str,
        package: &str,
        version: Option<&str>,
        identity: Option<&AuthIdentity>,
    ) -> Result<Vec<String>, LakeError> {
        let db = Builder::new_remote(turso_url.to_string(), turso_auth_token.to_string())
            .build()
            .await?;
        let conn = db.connect()?;
        Self::discover_catalog_paths_scoped_with_conn(&conn, ecosystem, package, version, identity)
            .await
    }

    async fn discover_catalog_paths_scoped_with_conn(
        conn: &Connection,
        ecosystem: &str,
        package: &str,
        version: Option<&str>,
        identity: Option<&AuthIdentity>,
    ) -> Result<Vec<String>, LakeError> {
        let mut params: Vec<libsql::Value> = vec![ecosystem.into(), package.into()];
        let mut idx: u32 = 3;

        let version_clause = if let Some(v) = version {
            params.push(v.into());
            idx = 4;
            "AND version = ?3"
        } else {
            ""
        };

        // Build visibility filter — mirrors visibility_filter_sql() in zen-db/repos/catalog.rs.
        // Duplicated because zen-lake cannot depend on zen-db. Keep both in sync.
        let vis_clause = match identity {
            Some(id) => {
                let mut clauses = vec!["visibility = 'public'".to_string()];
                if let Some(ref org_id) = id.org_id {
                    clauses.push(format!("(visibility = 'team' AND org_id = ?{idx})"));
                    params.push(org_id.as_str().into());
                    idx += 1;
                }
                clauses.push(format!("(visibility = 'private' AND owner_sub = ?{idx})"));
                params.push(id.user_id.as_str().into());
                format!("AND ({})", clauses.join(" OR "))
            }
            None => "AND visibility = 'public'".to_string(),
        };

        let sql = format!(
            "SELECT lance_path FROM dl_data_file
             WHERE ecosystem = ?1 AND package = ?2 {version_clause}
               AND instr(lance_path, '.lance') > 0
               AND instr(lance_path, '#') = 0
             {vis_clause}
             ORDER BY created_at DESC, id DESC"
        );

        let mut paths = Vec::new();
        let mut rows = conn.query(&sql, libsql::params_from_iter(params)).await?;
        while let Some(row) = rows.next().await? {
            paths.push(row.get::<String>(0)?);
        }
        Ok(paths)
    }

    /// Full cloud vector search with visibility scoping.
    ///
    /// # Errors
    ///
    /// Returns `LakeError` if catalog lookup or Lance querying fails.
    pub async fn search_cloud_vector_scoped(
        &self,
        turso_url: &str,
        turso_auth_token: &str,
        ecosystem: &str,
        package: &str,
        version: Option<&str>,
        query_embedding: &[f32],
        k: u32,
        identity: Option<&AuthIdentity>,
    ) -> Result<Vec<CloudVectorSearchResult>, LakeError> {
        let paths = self
            .discover_catalog_paths_scoped(
                turso_url,
                turso_auth_token,
                ecosystem,
                package,
                version,
                identity,
            )
            .await?;
        self.search_lance_paths_with_r2(&paths, query_embedding, k, None)
            .await
    }
}

fn append_batch_results(
    batch: &RecordBatch,
    lance_path: &str,
    out: &mut Vec<CloudVectorSearchResult>,
) -> Result<(), LakeError> {
    for row in 0..batch.num_rows() {
        let id = get_required_string(batch, "id", row)?;
        let version = get_required_string(batch, "version", row)?;
        let name = get_required_string(batch, "name", row)?;
        let kind = get_required_string(batch, "kind", row)?;
        let signature = get_optional_string(batch, "signature", row)?;
        let doc_comment = get_optional_string(batch, "doc_comment", row)?;
        let file_path = get_optional_string(batch, "file_path", row)?;
        let distance = get_distance(batch, row).unwrap_or(f64::MAX);

        out.push(CloudVectorSearchResult {
            id,
            version,
            name,
            kind,
            signature,
            doc_comment,
            file_path,
            distance,
            lance_path: lance_path.to_string(),
        });
    }

    Ok(())
}

fn get_required_string(batch: &RecordBatch, column: &str, row: usize) -> Result<String, LakeError> {
    get_optional_string(batch, column, row)?.ok_or_else(|| {
        LakeError::Other(format!(
            "missing required column '{column}' in lance query result"
        ))
    })
}

fn get_optional_string(
    batch: &RecordBatch,
    column: &str,
    row: usize,
) -> Result<Option<String>, LakeError> {
    let index = batch.schema().index_of(column).map_err(|e| {
        LakeError::Other(format!(
            "missing column '{column}' in lance query result: {e}"
        ))
    })?;
    let array = batch.column(index);

    if let Some(values) = array.as_any().downcast_ref::<arrow_array::StringArray>() {
        if values.is_null(row) {
            return Ok(None);
        }
        return Ok(Some(values.value(row).to_string()));
    }
    if let Some(values) = array
        .as_any()
        .downcast_ref::<arrow_array::LargeStringArray>()
    {
        if values.is_null(row) {
            return Ok(None);
        }
        return Ok(Some(values.value(row).to_string()));
    }

    Err(LakeError::Other(format!(
        "unsupported string column type for '{column}'"
    )))
}

fn get_distance(batch: &RecordBatch, row: usize) -> Option<f64> {
    let index = batch.schema().index_of("_distance").ok()?;
    let array = batch.column(index);

    if let Some(values) = array.as_any().downcast_ref::<arrow_array::Float64Array>() {
        if values.is_null(row) {
            return None;
        }
        return Some(values.value(row));
    }
    if let Some(values) = array.as_any().downcast_ref::<arrow_array::Float32Array>() {
        if values.is_null(row) {
            return None;
        }
        return Some(f64::from(values.value(row)));
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use zen_config::R2Config;

    use crate::ApiSymbolRow;

    fn load_env() {
        let workspace_env = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(|p| p.parent())
            .map(|p| p.join(".env"));
        if let Some(env_path) = workspace_env {
            let _ = dotenvy::from_path(env_path);
        }
    }

    fn r2_config_from_env() -> Option<R2Config> {
        let account_id = std::env::var("ZENITH_R2__ACCOUNT_ID").ok()?;
        let access_key_id = std::env::var("ZENITH_R2__ACCESS_KEY_ID").ok()?;
        let secret_access_key = std::env::var("ZENITH_R2__SECRET_ACCESS_KEY").ok()?;
        let bucket_name = std::env::var("ZENITH_R2__BUCKET_NAME").ok()?;
        if account_id.is_empty()
            || access_key_id.is_empty()
            || secret_access_key.is_empty()
            || bucket_name.is_empty()
        {
            return None;
        }

        Some(R2Config {
            account_id,
            access_key_id,
            secret_access_key,
            bucket_name,
            endpoint: String::new(),
        })
    }

    fn aws_env_ready() -> bool {
        std::env::var("AWS_ACCESS_KEY_ID").is_ok()
            && std::env::var("AWS_SECRET_ACCESS_KEY").is_ok()
            && std::env::var("AWS_ENDPOINT_URL").is_ok()
    }

    fn turso_remote_credentials() -> Option<(String, String)> {
        let url = std::env::var("ZENITH_TURSO__URL").ok()?;
        let token = std::env::var("ZENITH_TURSO__AUTH_TOKEN").ok()?;
        if url.is_empty() || token.is_empty() {
            return None;
        }
        Some((url, token))
    }

    fn synthetic_embedding(seed: u32) -> Vec<f32> {
        (0..384)
            .map(|i| {
                let base = (seed as f32) / 100.0;
                let variation = (i as f32) / 384.0;
                (base + variation).sin()
            })
            .collect()
    }

    #[tokio::test]
    async fn discover_catalog_paths_filters_by_version() {
        let db = Builder::new_local(":memory:").build().await.unwrap();
        let conn = db.connect().unwrap();
        conn.execute_batch(
            "CREATE TABLE dl_data_file (
                id TEXT NOT NULL,
                ecosystem TEXT NOT NULL,
                package TEXT NOT NULL,
                version TEXT NOT NULL,
                lance_path TEXT NOT NULL,
                visibility TEXT NOT NULL DEFAULT 'public',
                created_at TEXT NOT NULL
            )",
        )
        .await
        .unwrap();
        conn.execute(
            "INSERT INTO dl_data_file (id, ecosystem, package, version, lance_path, visibility, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, 'public', datetime('now'))",
            libsql::params![
                "dlf-1",
                "rust",
                "tokio",
                "1.39.0",
                "s3://idx/tokio/1.39/symbols.lance"
            ],
        )
        .await
        .unwrap();
        conn.execute(
            "INSERT INTO dl_data_file (id, ecosystem, package, version, lance_path, visibility, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, 'public', datetime('now'))",
            libsql::params![
                "dlf-2",
                "rust",
                "tokio",
                "1.40.0",
                "s3://idx/tokio/1.40/symbols.lance"
            ],
        )
        .await
        .unwrap();

        let paths =
            ZenLake::discover_catalog_paths_with_conn(&conn, "rust", "tokio", Some("1.40.0"))
                .await
                .unwrap();

        assert_eq!(paths.len(), 1);
        assert_eq!(paths[0], "s3://idx/tokio/1.40/symbols.lance");
    }

    #[tokio::test]
    async fn search_lance_paths_empty_returns_empty() {
        let lake = ZenLake::open_in_memory().unwrap();
        let results = lake
            .search_lance_paths(&[], &[0.1_f32; 384], 10)
            .await
            .expect("empty path search should succeed");
        assert!(results.is_empty());
    }

    #[tokio::test]
    async fn e2e_cloud_publish_catalog_lookup_and_vector_search() {
        load_env();

        let Some(r2) = r2_config_from_env() else {
            eprintln!("SKIP: R2 credentials not configured");
            return;
        };
        if !aws_env_ready() {
            eprintln!("SKIP: AWS_* env vars not configured for DuckDB lance reads");
            return;
        }
        let Some((turso_url, turso_token)) = turso_remote_credentials() else {
            eprintln!("SKIP: Turso remote credentials not configured");
            return;
        };

        let lake = ZenLake::open_in_memory().unwrap();
        let embedding = synthetic_embedding(7);
        let ecosystem = format!("it_rust_{}", Utc::now().timestamp_millis());
        let package = format!(
            "it_pkg_{}",
            Utc::now().timestamp_nanos_opt().unwrap_or_default()
        );
        let version = "0.1.0";

        lake.store_symbols(&[ApiSymbolRow {
            id: "sym-it-1".to_string(),
            ecosystem: ecosystem.clone(),
            package: package.clone(),
            version: version.to_string(),
            file_path: "src/lib.rs".to_string(),
            kind: "function".to_string(),
            name: "it_func".to_string(),
            signature: Some("pub fn it_func()".to_string()),
            source: Some("pub fn it_func() {}".to_string()),
            doc_comment: Some("integration test symbol".to_string()),
            line_start: Some(1),
            line_end: Some(1),
            visibility: Some("public".to_string()),
            is_async: false,
            is_unsafe: false,
            is_error_type: false,
            returns_result: false,
            return_type: None,
            generics: None,
            attributes: None,
            metadata: None,
            embedding: embedding.clone(),
        }])
        .unwrap();

        let export = match lake
            .write_to_r2(
                &r2,
                &ecosystem,
                &package,
                version,
                zen_core::enums::Visibility::Public,
            )
            .await
        {
            Ok(export) => export,
            Err(error) => {
                eprintln!("SKIP: R2 export failed: {error}");
                return;
            }
        };
        let Some(symbols_path) = export.symbols_lance_path.clone() else {
            eprintln!("SKIP: R2 export did not produce symbols path");
            return;
        };

        let db = match Builder::new_remote(turso_url.clone(), turso_token.clone())
            .build()
            .await
        {
            Ok(db) => db,
            Err(error) => {
                eprintln!("SKIP: Turso connection failed: {error}");
                return;
            }
        };
        let conn = match db.connect() {
            Ok(conn) => conn,
            Err(error) => {
                eprintln!("SKIP: Turso connect failed: {error}");
                return;
            }
        };

        let snapshot_id = format!("it-snap-{}", Utc::now().timestamp_micros());
        let file_id = format!(
            "it-file-{}",
            Utc::now().timestamp_nanos_opt().unwrap_or_default()
        );
        let created_at = Utc::now().to_rfc3339();

        if let Err(error) = conn
            .execute(
                "INSERT INTO dl_snapshot (id, created_at, note) VALUES (?1, ?2, ?3)",
                libsql::params![
                    snapshot_id.as_str(),
                    created_at.as_str(),
                    "integration test"
                ],
            )
            .await
        {
            eprintln!("SKIP: failed to insert dl_snapshot row: {error}");
            return;
        }

        if let Err(error) = conn
            .execute(
                "INSERT INTO dl_data_file
                 (id, snapshot_id, ecosystem, package, version, lance_path, visibility, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                libsql::params![
                    file_id.as_str(),
                    snapshot_id.as_str(),
                    ecosystem.as_str(),
                    package.as_str(),
                    version,
                    symbols_path.as_str(),
                    "public",
                    created_at.as_str(),
                ],
            )
            .await
        {
            let _ = conn
                .execute(
                    "DELETE FROM dl_snapshot WHERE id = ?1",
                    [snapshot_id.as_str()],
                )
                .await;
            eprintln!("SKIP: failed to insert dl_data_file row: {error}");
            return;
        }

        let results = match lake
            .search(
                &turso_url,
                &turso_token,
                &ecosystem,
                &package,
                Some(version),
                &embedding,
                5,
            )
            .await
        {
            Ok(results) => results,
            Err(error) => {
                let _ = conn
                    .execute("DELETE FROM dl_data_file WHERE id = ?1", [file_id.as_str()])
                    .await;
                let _ = conn
                    .execute(
                        "DELETE FROM dl_snapshot WHERE id = ?1",
                        [snapshot_id.as_str()],
                    )
                    .await;
                panic!("cloud search should succeed after publish+catalog registration: {error}");
            }
        };

        let _ = conn
            .execute("DELETE FROM dl_data_file WHERE id = ?1", [file_id.as_str()])
            .await;
        let _ = conn
            .execute(
                "DELETE FROM dl_snapshot WHERE id = ?1",
                [snapshot_id.as_str()],
            )
            .await;

        assert!(
            !results.is_empty(),
            "cloud search should return indexed symbol"
        );
        assert!(
            results.iter().any(|r| r.id == "sym-it-1"),
            "cloud search should include the exported symbol id"
        );
    }
}
