//! # Spike 0.18: R2 Parquet Export for Team Index
//!
//! Validates the R2 Parquet export pipeline for sharing indexed package data
//! across team members without requiring per-user MotherDuck accounts:
//!
//! - **Part A**: Export DuckDB tables (api_symbols, doc_chunks, indexed_packages) to R2 Parquet
//! - **Part B**: Read R2 Parquet files into local DuckDB, verify types and content
//! - **Part C**: Performance measurement for export and query
//! - **Part D**: Incremental export (delta files) and merge
//! - **Part E**: Manifest JSON lifecycle
//! - **Part F**: Lance format on R2 (vector search, FTS, hybrid search)
//!
//! ## Prerequisites
//!
//! R2 tests require these env vars in `zenith/.env`:
//!
//! ```bash
//! ZENITH_R2__ACCESS_KEY_ID=...
//! ZENITH_R2__SECRET_ACCESS_KEY=...
//! ZENITH_R2__ACCOUNT_ID=...
//! ZENITH_R2__BUCKET_NAME=zenith
//! ```
//!
//! Tests are skipped (not failed) when credentials are missing.

use duckdb::Connection;

/// Load env vars from the workspace .env file.
fn load_env() {
    let workspace_env = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .map(|p| p.join(".env"));

    if let Some(env_path) = workspace_env {
        let _ = dotenvy::from_path(&env_path);
    }
}

struct R2Creds {
    account_id: String,
    access_key_id: String,
    secret_access_key: String,
    bucket_name: String,
}

fn r2_credentials() -> Option<R2Creds> {
    load_env();
    let creds = R2Creds {
        account_id: std::env::var("ZENITH_R2__ACCOUNT_ID").ok()?,
        access_key_id: std::env::var("ZENITH_R2__ACCESS_KEY_ID").ok()?,
        secret_access_key: std::env::var("ZENITH_R2__SECRET_ACCESS_KEY").ok()?,
        bucket_name: std::env::var("ZENITH_R2__BUCKET_NAME").ok()?,
    };
    if creds.account_id.is_empty()
        || creds.access_key_id.is_empty()
        || creds.secret_access_key.is_empty()
        || creds.bucket_name.is_empty()
    {
        return None;
    }
    Some(creds)
}

/// Create an in-memory DuckDB connection with httpfs loaded and R2 secret configured.
fn r2_conn(creds: &R2Creds) -> Connection {
    let conn = Connection::open_in_memory().expect("Failed to open DuckDB in-memory");
    conn.execute_batch("INSTALL httpfs; LOAD httpfs;")
        .expect("Failed to install/load httpfs");
    conn.execute_batch(&format!(
        "CREATE SECRET r2_spike18 (
            TYPE s3,
            KEY_ID '{key_id}',
            SECRET '{secret}',
            ENDPOINT '{account_id}.r2.cloudflarestorage.com',
            URL_STYLE 'path'
        )",
        key_id = creds.access_key_id,
        secret = creds.secret_access_key,
        account_id = creds.account_id,
    ))
    .expect("Failed to create R2 secret");
    conn
}

/// Generate a deterministic 384-dim embedding from a seed.
fn synthetic_embedding(seed: u32) -> Vec<f32> {
    (0..384)
        .map(|i| {
            let base = (seed as f32) / 100.0;
            let variation = (i as f32) / 384.0;
            (base + variation).sin()
        })
        .collect()
}

/// Format a vector as a DuckDB array literal: [0.1, 0.2, ...]
fn vec_to_sql(v: &[f32]) -> String {
    format!(
        "[{}]",
        v.iter()
            .map(|x| format!("{x}"))
            .collect::<Vec<_>>()
            .join(", ")
    )
}

/// Unique R2 path prefix for this spike run to avoid collisions.
fn spike_prefix(creds: &R2Creds) -> String {
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis();
    format!("s3://{}/zenith-spike18/{ts}", creds.bucket_name)
}

/// Create the api_symbols table and populate with test data.
/// Uses DuckDB Appender for bulk insert when count > 100.
fn create_symbols_table(conn: &Connection, count: u32) {
    conn.execute_batch(
        "CREATE TABLE api_symbols (
            id VARCHAR,
            ecosystem VARCHAR NOT NULL,
            package VARCHAR NOT NULL,
            version VARCHAR NOT NULL,
            file_path VARCHAR NOT NULL,
            kind VARCHAR NOT NULL,
            name VARCHAR NOT NULL,
            signature VARCHAR,
            source TEXT,
            doc_comment TEXT,
            line_start INTEGER,
            line_end INTEGER,
            visibility VARCHAR DEFAULT 'public',
            is_async BOOLEAN DEFAULT FALSE,
            is_unsafe BOOLEAN DEFAULT FALSE,
            is_error_type BOOLEAN DEFAULT FALSE,
            returns_result BOOLEAN DEFAULT FALSE,
            metadata JSON,
            attributes TEXT,
            embedding FLOAT[],
            created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
        )",
    )
    .expect("Failed to create api_symbols table");

    if count <= 100 {
        // Small counts: individual inserts (simpler, supports all types)
        for i in 0..count {
            let emb = synthetic_embedding(i);
            let emb_sql = vec_to_sql(&emb);
            let is_async = i % 3 == 0;
            let is_error = i % 10 == 0;
            conn.execute(
                &format!(
                    "INSERT INTO api_symbols VALUES (
                        'sym-{i:05}', 'rust', 'tokio', '1.49.0',
                        'src/runtime/mod.rs', 'function', 'func_{i}',
                        'pub async fn func_{i}(x: i32) -> Result<()>',
                        'fn func_{i}(x: i32) {{ todo!() }}',
                        'Documentation for func_{i}',
                        {line_start}, {line_end}, 'public',
                        {is_async}, false, {is_error}, true,
                        '{{\"is_async\": {is_async}, \"return_type\": \"Result<()>\", \"lifetimes\": [\"a\"]}}',
                        '[\"#[tokio::main]\"]',
                        {emb_sql}::FLOAT[384],
                        CURRENT_TIMESTAMP
                    )",
                    line_start = i * 10 + 1,
                    line_end = i * 10 + 8,
                ),
                [],
            )
            .unwrap_or_else(|e| panic!("Failed to insert symbol {i}: {e}"));
        }
    } else {
        // Large counts: batch INSERT via generate_series for speed.
        // Use integer generate_series and compute embedding via list comprehension.
        conn.execute_batch(&format!(
            "INSERT INTO api_symbols
             SELECT
                 'sym-' || lpad(i::VARCHAR, 5, '0'),
                 'rust', 'tokio', '1.49.0',
                 'src/runtime/mod.rs', 'function',
                 'func_' || i::VARCHAR,
                 'pub async fn func_' || i::VARCHAR || '(x: i32) -> Result<()>',
                 'fn func_' || i::VARCHAR || '(x: i32) {{ todo!() }}',
                 'Documentation for func_' || i::VARCHAR,
                 i * 10 + 1,
                 i * 10 + 8,
                 'public',
                 (i % 3 = 0),
                 false,
                 (i % 10 = 0),
                 true,
                 json_object('is_async', (i % 3 = 0), 'return_type', 'Result<()>', 'lifetimes', json_array('a')),
                 json_array('#[tokio::main]'),
                 list_transform(generate_series(0, 383), j -> sin(i::FLOAT / 100.0 + j::FLOAT / 384.0))::FLOAT[384],
                 CURRENT_TIMESTAMP
             FROM generate_series(0, {count} - 1) t(i)"
        ))
        .unwrap_or_else(|e| panic!("Failed to batch insert {count} symbols: {e}"));
    }
}

/// Create the doc_chunks table and populate with test data.
fn create_doc_chunks_table(conn: &Connection, count: u32) {
    conn.execute_batch(
        "CREATE TABLE doc_chunks (
            id VARCHAR PRIMARY KEY,
            ecosystem VARCHAR NOT NULL,
            package VARCHAR NOT NULL,
            version VARCHAR NOT NULL,
            file_path VARCHAR NOT NULL,
            section_title VARCHAR,
            content TEXT NOT NULL,
            chunk_index INTEGER NOT NULL,
            embedding FLOAT[],
            created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
        )",
    )
    .expect("Failed to create doc_chunks table");

    for i in 0..count {
        let emb = synthetic_embedding(i + 10_000);
        let emb_sql = vec_to_sql(&emb);
        conn.execute(
            &format!(
                "INSERT INTO doc_chunks VALUES (
                    'chk-{i:05}', 'rust', 'tokio', '1.49.0',
                    'README.md', 'Section {i}',
                    'This is documentation chunk {i} about async runtime patterns.',
                    {i},
                    {emb_sql}::FLOAT[384],
                    CURRENT_TIMESTAMP
                )"
            ),
            [],
        )
        .unwrap_or_else(|e| panic!("Failed to insert chunk {i}: {e}"));
    }
}

/// Create the indexed_packages table and populate.
fn create_packages_table(conn: &Connection) {
    conn.execute_batch(
        "CREATE TABLE indexed_packages (
            ecosystem VARCHAR NOT NULL,
            package VARCHAR NOT NULL,
            version VARCHAR NOT NULL,
            indexed_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
            symbol_count INTEGER DEFAULT 0,
            chunk_count INTEGER DEFAULT 0,
            source_cached BOOLEAN DEFAULT FALSE,
            PRIMARY KEY (ecosystem, package, version)
        )",
    )
    .expect("Failed to create indexed_packages table");

    conn.execute_batch(
        "INSERT INTO indexed_packages VALUES
            ('rust', 'tokio', '1.49.0', CURRENT_TIMESTAMP, 100, 20, true),
            ('rust', 'axum', '0.8.0', CURRENT_TIMESTAMP, 50, 10, false),
            ('rust', 'serde', '1.0.219', CURRENT_TIMESTAMP, 200, 40, true)",
    )
    .expect("Failed to insert packages");
}

// ============================================================================
// Part A: Parquet Export to R2
// ============================================================================

#[test]
fn spike_export_symbols_to_r2_parquet() {
    let Some(creds) = r2_credentials() else {
        eprintln!("SKIP: R2 credentials not set");
        return;
    };

    let conn = r2_conn(&creds);
    let prefix = spike_prefix(&creds);
    let path = format!("{prefix}/api_symbols.parquet");

    create_symbols_table(&conn, 100);

    let start = std::time::Instant::now();
    conn.execute(
        &format!("COPY api_symbols TO '{path}' (FORMAT PARQUET, COMPRESSION ZSTD)"),
        [],
    )
    .expect("Failed to export api_symbols to R2");
    let elapsed = start.elapsed();

    eprintln!("  Exported 100 symbols to R2 in {elapsed:?}");
    eprintln!("  Path: {path}");

    // Verify by reading back row count
    let mut stmt = conn
        .prepare(&format!("SELECT count(*) FROM read_parquet('{path}')"))
        .unwrap();
    let count: i64 = stmt.query_row([], |row| row.get(0)).unwrap();
    assert_eq!(count, 100, "Should have 100 rows in exported Parquet");
    eprintln!("  Verified: {count} rows in R2 Parquet");
}

#[test]
fn spike_export_doc_chunks_to_r2_parquet() {
    let Some(creds) = r2_credentials() else {
        eprintln!("SKIP: R2 credentials not set");
        return;
    };

    let conn = r2_conn(&creds);
    let prefix = spike_prefix(&creds);
    let path = format!("{prefix}/doc_chunks.parquet");

    create_doc_chunks_table(&conn, 20);

    conn.execute(
        &format!("COPY doc_chunks TO '{path}' (FORMAT PARQUET, COMPRESSION ZSTD)"),
        [],
    )
    .expect("Failed to export doc_chunks to R2");

    let mut stmt = conn
        .prepare(&format!("SELECT count(*) FROM read_parquet('{path}')"))
        .unwrap();
    let count: i64 = stmt.query_row([], |row| row.get(0)).unwrap();
    assert_eq!(count, 20);
    eprintln!("  Exported and verified {count} doc chunks");
}

#[test]
fn spike_export_indexed_packages_to_r2_parquet() {
    let Some(creds) = r2_credentials() else {
        eprintln!("SKIP: R2 credentials not set");
        return;
    };

    let conn = r2_conn(&creds);
    let prefix = spike_prefix(&creds);
    let path = format!("{prefix}/indexed_packages.parquet");

    create_packages_table(&conn);

    conn.execute(
        &format!("COPY indexed_packages TO '{path}' (FORMAT PARQUET, COMPRESSION ZSTD)"),
        [],
    )
    .expect("Failed to export indexed_packages to R2");

    let mut stmt = conn
        .prepare(&format!("SELECT count(*) FROM read_parquet('{path}')"))
        .unwrap();
    let count: i64 = stmt.query_row([], |row| row.get(0)).unwrap();
    assert_eq!(count, 3);
    eprintln!("  Exported and verified {count} indexed packages");
}

// ============================================================================
// Part B: Parquet Read from R2
// ============================================================================

#[test]
fn spike_read_symbols_from_r2_parquet() {
    let Some(creds) = r2_credentials() else {
        eprintln!("SKIP: R2 credentials not set");
        return;
    };

    // Write first
    let conn = r2_conn(&creds);
    let prefix = spike_prefix(&creds);
    let path = format!("{prefix}/read_test_symbols.parquet");
    create_symbols_table(&conn, 50);
    conn.execute(
        &format!("COPY api_symbols TO '{path}' (FORMAT PARQUET, COMPRESSION ZSTD)"),
        [],
    )
    .unwrap();

    // Read into a fresh connection (simulates a different machine)
    let reader = r2_conn(&creds);
    let mut stmt = reader
        .prepare(&format!(
            "SELECT id, ecosystem, package, kind, name, signature, is_async, line_start
             FROM read_parquet('{path}') LIMIT 5"
        ))
        .unwrap();

    let rows: Vec<(String, String, String, String, String, String, bool, i32)> = stmt
        .query_map([], |row| {
            Ok((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
                row.get(5)?,
                row.get(6)?,
                row.get(7)?,
            ))
        })
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();

    assert_eq!(rows.len(), 5);
    assert_eq!(rows[0].1, "rust"); // ecosystem
    assert_eq!(rows[0].2, "tokio"); // package
    assert_eq!(rows[0].3, "function"); // kind
    eprintln!("  Read 5 symbols from R2 Parquet: all columns correct");
    for (id, _, _, kind, name, sig, is_async, line) in &rows {
        eprintln!("    {id}: {kind} {name} @ line {line} (async={is_async}) — {sig}");
    }
}

#[test]
fn spike_embedding_roundtrip_cosine_similarity() {
    let Some(creds) = r2_credentials() else {
        eprintln!("SKIP: R2 credentials not set");
        return;
    };

    let conn = r2_conn(&creds);
    let prefix = spike_prefix(&creds);
    let path = format!("{prefix}/vector_test.parquet");
    create_symbols_table(&conn, 50);
    conn.execute(
        &format!("COPY api_symbols TO '{path}' (FORMAT PARQUET, COMPRESSION ZSTD)"),
        [],
    )
    .unwrap();

    // Query embedding: use seed 5 which should be most similar to sym-00005
    let query_emb = synthetic_embedding(5);
    let query_sql = vec_to_sql(&query_emb);

    let reader = r2_conn(&creds);
    // Parquet stores FLOAT[] (variable-size). Must cast both sides to FLOAT[384].
    let mut stmt = reader
        .prepare(&format!(
            "SELECT name,
                    array_cosine_similarity(
                        embedding::FLOAT[384],
                        {query_sql}::FLOAT[384]
                    ) AS score
             FROM read_parquet('{path}')
             ORDER BY score DESC
             LIMIT 5"
        ))
        .unwrap();

    let results: Vec<(String, f64)> = stmt
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();

    assert!(!results.is_empty(), "Should have results");
    assert_eq!(results[0].0, "func_5", "Most similar should be func_5");
    assert!(
        results[0].1 > 0.99,
        "Self-similarity should be > 0.99, got {}",
        results[0].1
    );
    // Scores should be descending
    for w in results.windows(2) {
        assert!(
            w[0].1 >= w[1].1,
            "Scores should be descending: {} >= {}",
            w[0].1,
            w[1].1
        );
    }

    eprintln!("  Vector search over R2 Parquet works:");
    for (name, score) in &results {
        eprintln!("    {name}: {score:.6}");
    }
}

#[test]
fn spike_metadata_json_roundtrip() {
    let Some(creds) = r2_credentials() else {
        eprintln!("SKIP: R2 credentials not set");
        return;
    };

    let conn = r2_conn(&creds);
    let prefix = spike_prefix(&creds);
    let path = format!("{prefix}/json_test.parquet");
    create_symbols_table(&conn, 10);
    conn.execute(
        &format!("COPY api_symbols TO '{path}' (FORMAT PARQUET, COMPRESSION ZSTD)"),
        [],
    )
    .unwrap();

    let reader = r2_conn(&creds);

    // Test JSON operator access on Parquet data
    let mut stmt = reader
        .prepare(&format!(
            "SELECT name,
                    metadata->>'is_async' AS is_async_str,
                    metadata->>'return_type' AS ret_type,
                    json_array_length(metadata->'lifetimes') AS lifetime_count
             FROM read_parquet('{path}')
             WHERE metadata->>'is_async' = 'true'
             LIMIT 3"
        ))
        .unwrap();

    let rows: Vec<(String, String, String, i64)> = stmt
        .query_map([], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
        })
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();

    assert!(!rows.is_empty(), "Should find async functions");
    for (name, is_async, ret_type, lifetimes) in &rows {
        assert_eq!(is_async, "true");
        assert_eq!(ret_type, "Result<()>");
        assert_eq!(*lifetimes, 1, "Should have 1 lifetime");
        eprintln!("    {name}: is_async={is_async}, return_type={ret_type}, lifetimes={lifetimes}");
    }
    eprintln!("  JSON metadata roundtrip through Parquet works");
}

// ============================================================================
// Part C: Performance
// ============================================================================

#[test]
fn spike_export_10k_symbols_performance() {
    let Some(creds) = r2_credentials() else {
        eprintln!("SKIP: R2 credentials not set");
        return;
    };

    let conn = r2_conn(&creds);
    let prefix = spike_prefix(&creds);
    let path = format!("{prefix}/perf_10k.parquet");

    eprintln!("  Inserting 10,000 symbols...");
    let insert_start = std::time::Instant::now();
    create_symbols_table(&conn, 10_000);
    let insert_elapsed = insert_start.elapsed();
    eprintln!("  Insert time: {insert_elapsed:?}");

    eprintln!("  Exporting to R2...");
    let export_start = std::time::Instant::now();
    conn.execute(
        &format!("COPY api_symbols TO '{path}' (FORMAT PARQUET, COMPRESSION ZSTD)"),
        [],
    )
    .expect("Export failed");
    let export_elapsed = export_start.elapsed();
    eprintln!("  Export time: {export_elapsed:?}");
    assert!(
        export_elapsed.as_secs() < 30,
        "Export should take < 30s, took {export_elapsed:?}"
    );
}

#[test]
fn spike_query_10k_symbols_performance() {
    let Some(creds) = r2_credentials() else {
        eprintln!("SKIP: R2 credentials not set");
        return;
    };

    let conn = r2_conn(&creds);
    let prefix = spike_prefix(&creds);
    let path = format!("{prefix}/perf_query_10k.parquet");

    create_symbols_table(&conn, 10_000);
    conn.execute(
        &format!("COPY api_symbols TO '{path}' (FORMAT PARQUET, COMPRESSION ZSTD)"),
        [],
    )
    .unwrap();

    let reader = r2_conn(&creds);

    // Vector search
    let query_emb = synthetic_embedding(500);
    let query_sql = vec_to_sql(&query_emb);

    let vec_start = std::time::Instant::now();
    let mut stmt = reader
        .prepare(&format!(
            "SELECT name,
                    array_cosine_similarity(embedding::FLOAT[384], {query_sql}::FLOAT[384]) AS score
             FROM read_parquet('{path}')
             ORDER BY score DESC LIMIT 10"
        ))
        .unwrap();
    let _results: Vec<(String, f64)> = stmt
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    let vec_elapsed = vec_start.elapsed();
    eprintln!("  Vector search (10K symbols): {vec_elapsed:?}");

    // Text filter
    let text_start = std::time::Instant::now();
    let mut stmt = reader
        .prepare(&format!(
            "SELECT name, signature FROM read_parquet('{path}')
             WHERE name LIKE '%500%' OR doc_comment LIKE '%500%'
             LIMIT 10"
        ))
        .unwrap();
    let _results: Vec<(String, String)> = stmt
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    let text_elapsed = text_start.elapsed();
    eprintln!("  Text filter search (10K symbols): {text_elapsed:?}");

    assert!(
        vec_elapsed.as_secs() < 10,
        "Vector search should be < 10s, took {vec_elapsed:?}"
    );
    assert!(
        text_elapsed.as_secs() < 10,
        "Text search should be < 10s, took {text_elapsed:?}"
    );
}

// ============================================================================
// Part D: Incremental Export
// ============================================================================

#[test]
fn spike_incremental_export_by_timestamp() {
    let Some(creds) = r2_credentials() else {
        eprintln!("SKIP: R2 credentials not set");
        return;
    };

    let conn = r2_conn(&creds);
    let prefix = spike_prefix(&creds);
    let base_path = format!("{prefix}/incr_base.parquet");
    let delta_path = format!("{prefix}/incr_delta.parquet");

    // Create table with 100 rows (batch 1)
    create_symbols_table(&conn, 100);

    // Record the timestamp boundary — cast to VARCHAR since DuckDB returns Timestamp type
    let mut stmt = conn
        .prepare("SELECT max(created_at)::VARCHAR FROM api_symbols")
        .unwrap();
    let boundary: String = stmt.query_row([], |row| row.get(0)).unwrap();
    eprintln!("  Boundary timestamp: {boundary}");

    // Export base
    conn.execute(
        &format!("COPY api_symbols TO '{base_path}' (FORMAT PARQUET, COMPRESSION ZSTD)"),
        [],
    )
    .unwrap();

    // Small delay to ensure distinct timestamps
    std::thread::sleep(std::time::Duration::from_millis(100));

    // Insert 50 more rows (batch 2)
    for i in 100..150 {
        let emb = synthetic_embedding(i);
        let emb_sql = vec_to_sql(&emb);
        conn.execute(
            &format!(
                "INSERT INTO api_symbols VALUES (
                    'sym-{i:05}', 'rust', 'axum', '0.8.0',
                    'src/router.rs', 'function', 'func_{i}',
                    'pub fn func_{i}()', NULL, NULL,
                    {ls}, {le}, 'public', false, false, false, false,
                    '{{}}', '[]', {emb_sql}::FLOAT[384], CURRENT_TIMESTAMP
                )",
                ls = i * 10,
                le = i * 10 + 5,
            ),
            [],
        )
        .unwrap();
    }

    // Export delta (only rows after boundary)
    conn.execute(
        &format!(
            "COPY (SELECT * FROM api_symbols WHERE created_at > '{boundary}')
             TO '{delta_path}' (FORMAT PARQUET, COMPRESSION ZSTD)"
        ),
        [],
    )
    .unwrap();

    // Verify delta has exactly 50 rows
    let mut stmt = conn
        .prepare(&format!(
            "SELECT count(*) FROM read_parquet('{delta_path}')"
        ))
        .unwrap();
    let delta_count: i64 = stmt.query_row([], |row| row.get(0)).unwrap();
    assert_eq!(delta_count, 50, "Delta should have 50 rows");
    eprintln!("  Delta export: {delta_count} rows (expected 50)");
}

#[test]
fn spike_merge_base_and_delta_parquet() {
    let Some(creds) = r2_credentials() else {
        eprintln!("SKIP: R2 credentials not set");
        return;
    };

    let conn = r2_conn(&creds);
    let prefix = spike_prefix(&creds);
    let base_path = format!("{prefix}/merge_base.parquet");
    let delta_path = format!("{prefix}/merge_delta.parquet");

    // Base: 100 symbols from tokio
    create_symbols_table(&conn, 100);
    conn.execute(
        &format!("COPY api_symbols TO '{base_path}' (FORMAT PARQUET, COMPRESSION ZSTD)"),
        [],
    )
    .unwrap();

    // Delta: 50 more symbols (different IDs, from axum)
    conn.execute_batch("DELETE FROM api_symbols").unwrap();
    for i in 100..150 {
        let emb = synthetic_embedding(i);
        let emb_sql = vec_to_sql(&emb);
        conn.execute(
            &format!(
                "INSERT INTO api_symbols VALUES (
                    'sym-{i:05}', 'rust', 'axum', '0.8.0',
                    'src/router.rs', 'function', 'func_{i}',
                    'pub fn func_{i}()', NULL, NULL,
                    {ls}, {le}, 'public', false, false, false, false,
                    '{{}}', '[]', {emb_sql}::FLOAT[384], CURRENT_TIMESTAMP
                )",
                ls = i * 10,
                le = i * 10 + 5,
            ),
            [],
        )
        .unwrap();
    }
    conn.execute(
        &format!("COPY api_symbols TO '{delta_path}' (FORMAT PARQUET, COMPRESSION ZSTD)"),
        [],
    )
    .unwrap();

    // Merge: read both files with glob/list syntax
    let reader = r2_conn(&creds);
    let mut stmt = reader
        .prepare(&format!(
            "SELECT count(*) FROM read_parquet(['{base_path}', '{delta_path}'])"
        ))
        .unwrap();
    let total: i64 = stmt.query_row([], |row| row.get(0)).unwrap();
    assert_eq!(total, 150, "Merged should have 150 rows");

    // Verify both ecosystems present
    let mut stmt = reader
        .prepare(&format!(
            "SELECT package, count(*) as cnt
             FROM read_parquet(['{base_path}', '{delta_path}'])
             GROUP BY package ORDER BY package"
        ))
        .unwrap();
    let groups: Vec<(String, i64)> = stmt
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();

    assert_eq!(groups.len(), 2);
    eprintln!("  Merged Parquet files:");
    for (pkg, cnt) in &groups {
        eprintln!("    {pkg}: {cnt} symbols");
    }
    assert_eq!(groups[0], ("axum".to_string(), 50));
    assert_eq!(groups[1], ("tokio".to_string(), 100));
    eprintln!("  Base + delta merge: {total} total rows");
}

// ============================================================================
// Part E: Manifest
// ============================================================================

#[test]
fn spike_write_manifest_to_r2() {
    let Some(creds) = r2_credentials() else {
        eprintln!("SKIP: R2 credentials not set");
        return;
    };

    let conn = r2_conn(&creds);
    let prefix = spike_prefix(&creds);
    let manifest_path = format!("{prefix}/manifest.json");

    // Build manifest as a single-row JSON table and export
    let manifest = serde_json::json!({
        "schema_version": 1,
        "exported_at": chrono::Utc::now().to_rfc3339(),
        "org_id": "org_test_spike",
        "packages": [
            {"ecosystem": "rust", "package": "tokio", "version": "1.49.0", "symbol_count": 100},
            {"ecosystem": "rust", "package": "axum", "version": "0.8.0", "symbol_count": 50}
        ],
        "total_symbols": 150,
        "total_chunks": 30
    });

    // DuckDB can write JSON as a single-column table
    conn.execute(
        &format!(
            "COPY (SELECT '{manifest_str}'::JSON AS manifest)
             TO '{manifest_path}' (FORMAT JSON, ARRAY true)",
            manifest_str = manifest.to_string().replace('\'', "''"),
        ),
        [],
    )
    .expect("Failed to write manifest");

    eprintln!("  Wrote manifest to {manifest_path}");

    // Read it back — cast struct to JSON string via to_json()
    let mut stmt = conn
        .prepare(&format!(
            "SELECT to_json(manifest)::VARCHAR FROM read_json('{manifest_path}')"
        ))
        .unwrap();
    let result: String = stmt.query_row([], |row| row.get(0)).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();

    assert_eq!(parsed["schema_version"], 1);
    assert_eq!(parsed["total_symbols"], 150);
    assert_eq!(parsed["packages"].as_array().unwrap().len(), 2);
    eprintln!("  Manifest read back and verified");
}

#[test]
fn spike_read_manifest_check_freshness() {
    let Some(creds) = r2_credentials() else {
        eprintln!("SKIP: R2 credentials not set");
        return;
    };

    let conn = r2_conn(&creds);
    let prefix = spike_prefix(&creds);
    let manifest_path = format!("{prefix}/freshness_manifest.json");

    // Write a manifest with a known timestamp
    let exported_at = chrono::Utc::now().to_rfc3339();
    let manifest = serde_json::json!({
        "schema_version": 1,
        "exported_at": exported_at,
        "org_id": "org_test",
        "packages": [
            {"ecosystem": "rust", "package": "tokio", "version": "1.49.0", "symbol_count": 100}
        ],
        "total_symbols": 100,
        "total_chunks": 20
    });

    conn.execute(
        &format!(
            "COPY (SELECT '{manifest_str}'::JSON AS manifest)
             TO '{manifest_path}' (FORMAT JSON, ARRAY true)",
            manifest_str = manifest.to_string().replace('\'', "''"),
        ),
        [],
    )
    .unwrap();

    // Read and check freshness — cast struct to JSON string
    let mut stmt = conn
        .prepare(&format!(
            "SELECT to_json(manifest)::VARCHAR FROM read_json('{manifest_path}')"
        ))
        .unwrap();
    let result: String = stmt.query_row([], |row| row.get(0)).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();

    let exported_str = parsed["exported_at"].as_str().unwrap();
    let exported_time = chrono::DateTime::parse_from_rfc3339(exported_str).unwrap();
    let now = chrono::Utc::now();
    let age = now.signed_duration_since(exported_time);

    let is_stale = age.num_seconds() > 3600; // > 1 hour = stale
    eprintln!("  Manifest exported_at: {exported_str}");
    eprintln!("  Age: {}s", age.num_seconds());
    eprintln!("  Is stale (> 1h): {is_stale}");
    assert!(!is_stale, "Just-written manifest should not be stale");

    let packages = parsed["packages"].as_array().unwrap();
    eprintln!("  Packages in manifest:");
    for pkg in packages {
        eprintln!(
            "    {}/{} v{} ({} symbols)",
            pkg["ecosystem"].as_str().unwrap(),
            pkg["package"].as_str().unwrap(),
            pkg["version"].as_str().unwrap(),
            pkg["symbol_count"]
        );
    }
}

// ============================================================================
// Part F: Lance Format on R2
// ============================================================================
//
// Lance is validated in spike 0.5 as superior for vector search:
// - Persistent vector indexes (vs HNSW crash on DuckDB 1.4)
// - Native BM25 FTS via lance_fts()
// - Hybrid search via lance_hybrid_search()
// - S3/R2 support via AWS credential chain (NOT DuckDB secrets)
//
// These tests validate Lance on R2 specifically for the team index use case:
// real 384-dim embeddings, JSON metadata, and the full search stack.

/// Create a DuckDB connection with lance extension loaded.
fn lance_conn() -> Connection {
    let conn = Connection::open_in_memory().expect("Failed to open DuckDB");
    conn.execute_batch("INSTALL lance FROM community; LOAD lance;")
        .expect("Failed to install/load lance extension");
    conn
}

/// Check if Lance AWS credentials are available.
fn lance_r2_available() -> bool {
    load_env();
    std::env::var("AWS_ACCESS_KEY_ID").is_ok()
        && std::env::var("AWS_SECRET_ACCESS_KEY").is_ok()
        && std::env::var("AWS_ENDPOINT_URL").is_ok()
}

/// Get a unique Lance R2 path for this test run.
fn lance_r2_path(creds: &R2Creds, name: &str) -> String {
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis();
    format!(
        "s3://{}/zenith-spike18/lance_{ts}_{name}.lance",
        creds.bucket_name
    )
}

/// F1: Write api_symbols with 384-dim embeddings to Lance on R2, read back.
#[test]
fn spike_lance_r2_write_and_scan() {
    let Some(creds) = r2_credentials() else {
        eprintln!("SKIP: R2 credentials not set");
        return;
    };
    if !lance_r2_available() {
        eprintln!("SKIP: AWS env vars not set (Lance needs AWS_ACCESS_KEY_ID etc.)");
        return;
    }

    let conn = lance_conn();
    let path = lance_r2_path(&creds, "symbols");

    // Create table with real 384-dim embeddings and JSON metadata
    create_symbols_table(&conn, 50);

    let start = std::time::Instant::now();
    conn.execute_batch(&format!("COPY api_symbols TO '{path}' (FORMAT lance)"))
        .expect("Failed to write Lance to R2");
    let elapsed = start.elapsed();
    eprintln!("  Wrote 50 symbols to Lance on R2 in {elapsed:?}");
    eprintln!("  Path: {path}");

    // Read back from R2
    let mut stmt = conn
        .prepare(&format!("SELECT count(*) FROM '{path}'"))
        .unwrap();
    let count: i64 = stmt.query_row([], |row| row.get(0)).unwrap();
    assert_eq!(count, 50, "Should have 50 rows in Lance dataset");

    // Verify column types survive
    let mut stmt = conn
        .prepare(&format!(
            "SELECT id, name, kind, is_async, metadata->>'return_type' AS ret
             FROM '{path}' WHERE is_async = true LIMIT 3"
        ))
        .unwrap();
    let rows: Vec<(String, String, String, bool, String)> = stmt
        .query_map([], |row| {
            Ok((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
            ))
        })
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();

    assert!(!rows.is_empty(), "Should find async functions");
    for (id, name, kind, is_async, ret) in &rows {
        assert!(is_async);
        assert_eq!(kind, "function");
        eprintln!("    {id}: {name} -> {ret}");
    }
    eprintln!("  Lance R2 scan: {count} rows, columns + JSON metadata intact");
}

/// F2: lance_vector_search on R2 with real 384-dim embeddings.
#[test]
fn spike_lance_r2_vector_search() {
    let Some(creds) = r2_credentials() else {
        eprintln!("SKIP: R2 credentials not set");
        return;
    };
    if !lance_r2_available() {
        eprintln!("SKIP: AWS env vars not set");
        return;
    }

    let conn = lance_conn();
    let path = lance_r2_path(&creds, "vec_search");

    create_symbols_table(&conn, 100);
    conn.execute_batch(&format!("COPY api_symbols TO '{path}' (FORMAT lance)"))
        .unwrap();

    // Query vector for seed=5 (should match func_5 closest)
    let query_emb = synthetic_embedding(5);
    let query_sql = vec_to_sql(&query_emb);

    let start = std::time::Instant::now();
    let mut stmt = conn
        .prepare(&format!(
            "SELECT name, _distance
             FROM lance_vector_search('{path}', 'embedding', {query_sql}::FLOAT[384], k=5)
             ORDER BY _distance ASC"
        ))
        .unwrap();

    let results: Vec<(String, f64)> = stmt
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    let elapsed = start.elapsed();

    assert!(!results.is_empty(), "Should have results");
    assert_eq!(results[0].0, "func_5", "Nearest should be func_5");
    assert!(
        results[0].1 < 0.01,
        "Self-distance should be ~0, got {}",
        results[0].1
    );

    // Distances should be ascending
    for w in results.windows(2) {
        assert!(
            w[0].1 <= w[1].1,
            "Distances should be ascending: {} <= {}",
            w[0].1,
            w[1].1
        );
    }

    eprintln!("  lance_vector_search on R2 (100 symbols, 384-dim): {elapsed:?}");
    for (name, dist) in &results {
        eprintln!("    {name}: distance={dist:.6}");
    }
}

/// F3: lance_fts (BM25) on R2 — search doc_comment text.
#[test]
fn spike_lance_r2_fts_search() {
    let Some(creds) = r2_credentials() else {
        eprintln!("SKIP: R2 credentials not set");
        return;
    };
    if !lance_r2_available() {
        eprintln!("SKIP: AWS env vars not set");
        return;
    }

    let conn = lance_conn();
    let path = lance_r2_path(&creds, "fts_search");

    // Create doc_chunks with searchable content
    conn.execute_batch(
        "CREATE TABLE fts_docs (
            id VARCHAR, content TEXT, embedding FLOAT[4]
        )",
    )
    .unwrap();
    conn.execute_batch(
        "INSERT INTO fts_docs VALUES
            ('d1', 'How to spawn a tokio task for async concurrency', [0.9, 0.8, 0.1, 0.1]::FLOAT[4]),
            ('d2', 'Connection pooling with reqwest HTTP client', [0.1, 0.1, 0.9, 0.8]::FLOAT[4]),
            ('d3', 'Serde serialization and deserialization patterns', [0.5, 0.5, 0.5, 0.5]::FLOAT[4]),
            ('d4', 'Understanding the tokio runtime and spawn_blocking', [0.8, 0.7, 0.2, 0.2]::FLOAT[4]),
            ('d5', 'Error handling with thiserror and anyhow crates', [0.3, 0.3, 0.7, 0.7]::FLOAT[4])"
    ).unwrap();
    conn.execute_batch(&format!("COPY fts_docs TO '{path}' (FORMAT lance)"))
        .unwrap();

    let start = std::time::Instant::now();
    let mut stmt = conn
        .prepare(&format!(
            "SELECT id, content, _score
             FROM lance_fts('{path}', 'content', 'tokio spawn', k=5)
             ORDER BY _score DESC"
        ))
        .unwrap();

    let results: Vec<(String, String, f64)> = stmt
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    let elapsed = start.elapsed();

    assert!(
        !results.is_empty(),
        "Should find docs matching 'tokio spawn'"
    );
    // d1 and d4 both mention tokio and spawn
    let top_ids: Vec<&str> = results.iter().map(|(id, _, _)| id.as_str()).collect();
    assert!(
        top_ids.contains(&"d1") || top_ids.contains(&"d4"),
        "Should find d1 or d4 in results: {top_ids:?}"
    );

    eprintln!("  lance_fts on R2 for 'tokio spawn': {elapsed:?}");
    for (id, content, score) in &results {
        eprintln!(
            "    {id} (score={score:.4}): {}",
            &content[..content.len().min(60)]
        );
    }
}

/// F4: lance_hybrid_search on R2 — combined vector + FTS.
#[test]
fn spike_lance_r2_hybrid_search() {
    let Some(creds) = r2_credentials() else {
        eprintln!("SKIP: R2 credentials not set");
        return;
    };
    if !lance_r2_available() {
        eprintln!("SKIP: AWS env vars not set");
        return;
    }

    let conn = lance_conn();
    let path = lance_r2_path(&creds, "hybrid");

    conn.execute_batch(
        "CREATE TABLE hybrid_docs (
            id VARCHAR, content TEXT, embedding FLOAT[4]
        )",
    )
    .unwrap();
    conn.execute_batch(
        "INSERT INTO hybrid_docs VALUES
            ('d1', 'How to spawn a tokio task for async concurrency', [0.9, 0.8, 0.1, 0.1]::FLOAT[4]),
            ('d2', 'Connection pooling with reqwest HTTP client', [0.1, 0.1, 0.9, 0.8]::FLOAT[4]),
            ('d3', 'Serde serialization and deserialization patterns', [0.5, 0.5, 0.5, 0.5]::FLOAT[4]),
            ('d4', 'Understanding the tokio runtime and spawn_blocking', [0.8, 0.7, 0.2, 0.2]::FLOAT[4]),
            ('d5', 'Error handling with thiserror and anyhow crates', [0.3, 0.3, 0.7, 0.7]::FLOAT[4])"
    ).unwrap();
    conn.execute_batch(&format!("COPY hybrid_docs TO '{path}' (FORMAT lance)"))
        .unwrap();

    // Hybrid: vector close to d1 + text "spawn"
    let start = std::time::Instant::now();
    let mut stmt = conn
        .prepare(&format!(
            "SELECT id, content, _hybrid_score
             FROM lance_hybrid_search(
                 '{path}',
                 'embedding', [0.9, 0.8, 0.1, 0.1]::FLOAT[4],
                 'content', 'spawn',
                 k=5, alpha=0.5
             )
             ORDER BY _hybrid_score DESC"
        ))
        .unwrap();

    let results: Vec<(String, String, f64)> = stmt
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    let elapsed = start.elapsed();

    assert!(!results.is_empty(), "Hybrid search should return results");
    // d1 or d4 should rank highest: both are vector-close AND contain "spawn"/"tokio"
    // d4 may rank higher because "spawn_blocking" contains "spawn" and BM25 scores it well
    let top_id = &results[0].0;
    assert!(
        top_id == "d1" || top_id == "d4",
        "Top hybrid result should be d1 or d4 (both match vector + text), got {top_id}"
    );

    eprintln!("  lance_hybrid_search on R2 (alpha=0.5): {elapsed:?}");
    for (id, content, score) in &results {
        eprintln!(
            "    {id} (hybrid={score:.4}): {}",
            &content[..content.len().min(60)]
        );
    }
}

/// F5: Performance comparison — Parquet brute-force vs Lance vector search on R2.
#[test]
fn spike_lance_vs_parquet_vector_perf() {
    let Some(creds) = r2_credentials() else {
        eprintln!("SKIP: R2 credentials not set");
        return;
    };
    if !lance_r2_available() {
        eprintln!("SKIP: AWS env vars not set");
        return;
    }

    let conn = lance_conn();
    // Also load httpfs for Parquet comparison
    conn.execute_batch("INSTALL httpfs; LOAD httpfs;").unwrap();
    conn.execute_batch(&format!(
        "CREATE SECRET r2_lance_perf (
            TYPE s3,
            KEY_ID '{key_id}',
            SECRET '{secret}',
            ENDPOINT '{account_id}.r2.cloudflarestorage.com',
            URL_STYLE 'path'
        )",
        key_id = creds.access_key_id,
        secret = creds.secret_access_key,
        account_id = creds.account_id,
    ))
    .unwrap();

    let prefix = spike_prefix(&creds);
    let parquet_path = format!("{prefix}/perf_compare.parquet");
    let lance_path = lance_r2_path(&creds, "perf_compare");

    // Create 1000 symbols (enough to see a difference, not so many it takes forever)
    create_symbols_table(&conn, 100);

    // Export both formats
    conn.execute(
        &format!("COPY api_symbols TO '{parquet_path}' (FORMAT PARQUET, COMPRESSION ZSTD)"),
        [],
    )
    .unwrap();
    conn.execute_batch(&format!(
        "COPY api_symbols TO '{lance_path}' (FORMAT lance)"
    ))
    .unwrap();

    let query_emb = synthetic_embedding(42);
    let query_sql = vec_to_sql(&query_emb);

    // Parquet: brute-force cosine similarity
    let pq_start = std::time::Instant::now();
    let mut stmt = conn
        .prepare(&format!(
            "SELECT name, array_cosine_similarity(embedding::FLOAT[384], {query_sql}::FLOAT[384]) AS score
             FROM read_parquet('{parquet_path}')
             ORDER BY score DESC LIMIT 5"
        ))
        .unwrap();
    let _: Vec<(String, f64)> = stmt
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    let pq_elapsed = pq_start.elapsed();

    // Lance: native vector search
    let lance_start = std::time::Instant::now();
    let mut stmt = conn
        .prepare(&format!(
            "SELECT name, _distance
             FROM lance_vector_search('{lance_path}', 'embedding', {query_sql}::FLOAT[384], k=5)
             ORDER BY _distance ASC"
        ))
        .unwrap();
    let _: Vec<(String, f64)> = stmt
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    let lance_elapsed = lance_start.elapsed();

    eprintln!("  Performance comparison (100 symbols, 384-dim, R2):");
    eprintln!("    Parquet brute-force: {pq_elapsed:?}");
    eprintln!("    Lance vector_search: {lance_elapsed:?}");
    eprintln!(
        "    Ratio: {:.1}x",
        pq_elapsed.as_secs_f64() / lance_elapsed.as_secs_f64().max(0.001)
    );
}
