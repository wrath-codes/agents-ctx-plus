//! # Spike 0.5: DuckDB VSS Extension + MotherDuck + R2 Validation
//!
//! Validates the full DuckDB cloud stack for zenith's documentation lake:
//!
//! ## Part 1: VSS (Vector Similarity Search) — Local
//!
//! - **Extension loading**: `INSTALL vss; LOAD vss;` from the Rust crate
//! - **HNSW index creation**: `CREATE INDEX ... USING HNSW (col) WITH (metric = 'cosine')`
//! - **Vector insert**: `FLOAT[384]` columns populated with synthetic embeddings
//! - **Similarity search**: `array_cosine_similarity()` with `ORDER BY score DESC`
//! - **Filtered vector search**: WHERE clause + vector similarity (hybrid search)
//! - **Nearest neighbor correctness**: Known vectors return expected matches
//!
//! ## Part 2: MotherDuck — Cloud Compute
//!
//! - **Connection**: `md:` protocol with `ZENITH_MOTHERDUCK__ACCESS_TOKEN`
//! - **Remote DB creation**: `CREATE DATABASE` on MotherDuck
//! - **Remote table ops**: CREATE TABLE, INSERT, SELECT through MotherDuck
//! - **Mixed local/remote**: Query local data alongside MotherDuck data
//!
//! ## Part 3: R2 — Cloud Storage (via httpfs)
//!
//! - **Extension loading**: `INSTALL httpfs; LOAD httpfs;`
//! - **S3 secret config**: R2 credentials as DuckDB S3 secret
//! - **Parquet write**: `COPY ... TO 's3://...' (FORMAT PARQUET)`
//! - **Parquet read**: `SELECT * FROM read_parquet('s3://...')`
//!
//! ## Validates
//!
//! Vector search works in DuckDB — blocks Phase 4. MotherDuck and R2 connectivity
//! validated — informs Phase 8 (cloud sync) design.
//!
//! ## Prerequisites
//!
//! - **VSS tests**: No credentials needed (local only)
//! - **MotherDuck tests**: Require `ZENITH_MOTHERDUCK__ACCESS_TOKEN` env var
//! - **R2 tests**: Require `ZENITH_R2__ACCESS_KEY_ID`, `ZENITH_R2__SECRET_ACCESS_KEY`,
//!   `ZENITH_R2__ACCOUNT_ID`, `ZENITH_R2__BUCKET_NAME` env vars
//!
//! Cloud tests are skipped (not failed) when credentials are missing.

use duckdb::{params, Connection};
use tempfile::TempDir;

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

/// Helper: create an in-memory DuckDB connection with VSS loaded.
fn vss_conn() -> Connection {
    let conn = Connection::open_in_memory().expect("failed to open DuckDB in-memory");
    conn.execute_batch("INSTALL vss; LOAD vss;")
        .expect("failed to install/load VSS extension");
    conn
}

/// Generate a deterministic 384-dim embedding from a seed.
/// Similar seeds produce similar vectors (for testing NN correctness).
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

// ===========================================================================
// Part 1: VSS — Local Vector Similarity Search
// ===========================================================================

/// Verify that the VSS extension installs and loads from the Rust crate.
#[test]
fn spike_vss_extension_loads() {
    let conn = vss_conn();

    // Verify VSS is loaded by querying installed extensions
    let mut stmt = conn
        .prepare(
            "SELECT extension_name, loaded FROM duckdb_extensions() WHERE extension_name = 'vss'",
        )
        .unwrap();

    let (name, loaded): (String, bool) = stmt
        .query_row([], |row| Ok((row.get(0)?, row.get(1)?)))
        .unwrap();

    assert_eq!(name, "vss");
    assert!(loaded, "VSS extension should be loaded");
}

/// Verify HNSW index creation on a FLOAT[384] column.
#[test]
fn spike_vss_hnsw_index_creation() {
    let conn = vss_conn();

    conn.execute_batch(
        "CREATE TABLE embeddings (
            id TEXT PRIMARY KEY,
            content TEXT NOT NULL,
            embedding FLOAT[384]
        )",
    )
    .unwrap();

    // Create HNSW index with cosine metric (zenith's default)
    conn.execute_batch(
        "CREATE INDEX idx_embeddings ON embeddings USING HNSW (embedding) WITH (metric = 'cosine')",
    )
    .unwrap();

    // Verify the index exists
    let count: i64 = conn
        .query_row(
            "SELECT count(*) FROM duckdb_indexes() WHERE index_name = 'idx_embeddings'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(count, 1, "HNSW index should be created");
}

/// Verify vector insert + cosine similarity search returns correct nearest neighbors.
#[test]
fn spike_vss_similarity_search() {
    let conn = vss_conn();

    conn.execute_batch(
        "CREATE TABLE api_symbols (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            kind TEXT NOT NULL,
            embedding FLOAT[384]
        )",
    )
    .unwrap();

    // Insert symbols with synthetic embeddings
    // Seeds close together → similar vectors → should rank higher
    let symbols = [
        ("sym-001", "spawn", "function", 10u32),          // cluster A
        ("sym-002", "spawn_blocking", "function", 11u32), // cluster A (similar to spawn)
        ("sym-003", "spawn_local", "function", 12u32),    // cluster A (similar to spawn)
        ("sym-004", "connect", "function", 50u32),        // cluster B
        ("sym-005", "Connection", "struct", 51u32),       // cluster B (similar to connect)
        ("sym-006", "serialize", "function", 90u32),      // cluster C (dissimilar to A & B)
    ];

    for (id, name, kind, seed) in &symbols {
        let emb = synthetic_embedding(*seed);
        let emb_sql = vec_to_sql(&emb);
        conn.execute(
            "INSERT INTO api_symbols (id, name, kind, embedding) VALUES (?, ?, ?, ?::FLOAT[384])",
            params![*id, *name, *kind, emb_sql],
        )
        .unwrap();
    }

    // Create HNSW index
    conn.execute_batch(
        "CREATE INDEX idx_sym_emb ON api_symbols USING HNSW (embedding) WITH (metric = 'cosine')",
    )
    .unwrap();

    // Search for vectors similar to "spawn" (seed=10)
    let query_emb = synthetic_embedding(10);
    let query_sql = vec_to_sql(&query_emb);

    let mut stmt = conn
        .prepare(&format!(
            "SELECT name, array_cosine_similarity(embedding, {query_sql}::FLOAT[384]) as score
             FROM api_symbols
             ORDER BY score DESC
             LIMIT 3"
        ))
        .unwrap();

    let results: Vec<(String, f64)> = stmt
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();

    assert_eq!(results.len(), 3);

    // Top result should be "spawn" itself (perfect match)
    assert_eq!(
        results[0].0, "spawn",
        "top result should be spawn (exact match)"
    );
    assert!(
        results[0].1 > 0.99,
        "exact match should have score > 0.99, got {}",
        results[0].1
    );

    // Next results should be from cluster A (spawn_blocking, spawn_local)
    let top3_names: Vec<&str> = results.iter().map(|r| r.0.as_str()).collect();
    assert!(
        top3_names.contains(&"spawn_blocking") || top3_names.contains(&"spawn_local"),
        "top 3 should contain cluster A members: {top3_names:?}"
    );

    // "serialize" (seed=90) should NOT be in top 3
    assert!(
        !top3_names.contains(&"serialize"),
        "dissimilar 'serialize' should not be in top 3: {top3_names:?}"
    );
}

/// Verify hybrid search: WHERE clause filters + vector similarity ordering.
/// This is zenith's primary search pattern.
#[test]
fn spike_vss_hybrid_search() {
    let conn = vss_conn();

    conn.execute_batch(
        "CREATE TABLE api_symbols (
            id TEXT PRIMARY KEY,
            ecosystem TEXT NOT NULL,
            package TEXT NOT NULL,
            kind TEXT NOT NULL,
            name TEXT NOT NULL,
            is_async BOOLEAN DEFAULT FALSE,
            embedding FLOAT[384]
        )",
    )
    .unwrap();

    // Insert mixed symbols across packages
    let symbols = [
        ("s1", "rust", "tokio", "function", "spawn", true, 10u32),
        (
            "s2",
            "rust",
            "tokio",
            "function",
            "spawn_blocking",
            true,
            11u32,
        ),
        ("s3", "rust", "tokio", "struct", "Runtime", false, 50u32),
        ("s4", "rust", "serde", "function", "serialize", false, 90u32),
        ("s5", "rust", "serde", "trait", "Serialize", false, 91u32),
        (
            "s6",
            "python",
            "asyncio",
            "function",
            "create_task",
            true,
            12u32,
        ),
    ];

    for (id, eco, pkg, kind, name, is_async, seed) in &symbols {
        let emb = synthetic_embedding(*seed);
        let emb_sql = vec_to_sql(&emb);
        conn.execute(
            "INSERT INTO api_symbols VALUES (?, ?, ?, ?, ?, ?, ?::FLOAT[384])",
            params![*id, *eco, *pkg, *kind, *name, *is_async, emb_sql],
        )
        .unwrap();
    }

    conn.execute_batch(
        "CREATE INDEX idx ON api_symbols USING HNSW (embedding) WITH (metric = 'cosine')",
    )
    .unwrap();

    // Hybrid search: async functions in tokio, ordered by similarity to "spawn"
    let query_emb = synthetic_embedding(10);
    let query_sql = vec_to_sql(&query_emb);

    let mut stmt = conn
        .prepare(&format!(
            "SELECT name, array_cosine_similarity(embedding, {query_sql}::FLOAT[384]) as score
             FROM api_symbols
             WHERE ecosystem = 'rust'
               AND package = 'tokio'
               AND is_async = TRUE
             ORDER BY score DESC"
        ))
        .unwrap();

    let results: Vec<(String, f64)> = stmt
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();

    // Should only return async functions in tokio: spawn, spawn_blocking
    assert_eq!(results.len(), 2, "should get 2 async tokio functions");
    assert_eq!(results[0].0, "spawn");
    assert_eq!(results[1].0, "spawn_blocking");

    // Serde and asyncio symbols should be filtered out
    let names: Vec<&str> = results.iter().map(|r| r.0.as_str()).collect();
    assert!(!names.contains(&"serialize"));
    assert!(!names.contains(&"create_task"));
}

/// Verify HNSW index creation on file-backed DuckDB.
///
/// **FINDING**: HNSW persistence is experimental in DuckDB 1.4. Creating the index
/// on a file-backed DB works (with `SET hnsw_enable_experimental_persistence = true`),
/// but reopening the DB and using the persisted index causes an assertion failure:
/// `(unbound_count == 0), function Bind, file table_index_list.cpp, line 121`
///
/// **Implication for zenith**: Use HNSW indexes only in-memory. For persistent storage,
/// store embeddings in Parquet on R2 and build HNSW indexes in-memory at query time,
/// or use `array_cosine_similarity()` without an index (brute-force, acceptable for
/// <100K symbols). Revisit when DuckDB stabilizes HNSW persistence.
#[test]
fn spike_vss_persisted_index_creation() {
    let dir = TempDir::new().unwrap();
    let db_path = dir.path().join("vss_test.duckdb");

    let conn = Connection::open(&db_path).unwrap();
    conn.execute_batch("INSTALL vss; LOAD vss;").unwrap();
    conn.execute_batch("SET hnsw_enable_experimental_persistence = true;")
        .unwrap();

    conn.execute_batch(
        "CREATE TABLE vectors (
            id INTEGER PRIMARY KEY,
            embedding FLOAT[384]
        )",
    )
    .unwrap();

    for i in 0..100 {
        let emb = synthetic_embedding(i);
        let emb_sql = vec_to_sql(&emb);
        conn.execute(
            "INSERT INTO vectors VALUES (?, ?::FLOAT[384])",
            params![i as i32, emb_sql],
        )
        .unwrap();
    }

    // Creating the HNSW index on file-backed DB works
    conn.execute_batch(
        "CREATE INDEX idx_vec ON vectors USING HNSW (embedding) WITH (metric = 'cosine')",
    )
    .unwrap();

    // Search works in the same session
    let query = synthetic_embedding(42);
    let query_sql = vec_to_sql(&query);

    let top_id: i32 = conn
        .query_row(
            &format!(
                "SELECT id FROM vectors
                 ORDER BY array_cosine_similarity(embedding, {query_sql}::FLOAT[384]) DESC
                 LIMIT 1"
            ),
            [],
            |row| row.get(0),
        )
        .unwrap();

    assert_eq!(top_id, 42, "nearest neighbor to seed=42 should be id=42");

    // NOTE: Reopening this DB and loading the persisted HNSW index will crash
    // with SIGABRT (assertion failure in DuckDB 1.4). Do NOT attempt to reopen
    // a DB with persisted HNSW indexes until this is fixed upstream.
}

// ===========================================================================
// Part 2: MotherDuck — Cloud Compute
// ===========================================================================

fn motherduck_token() -> Option<String> {
    load_env();
    std::env::var("ZENITH_MOTHERDUCK__ACCESS_TOKEN")
        .ok()
        .filter(|t| !t.is_empty())
}

/// Verify that we can connect to MotherDuck, create a test DB, and run queries.
#[test]
fn spike_motherduck_connect_and_query() {
    let Some(token) = motherduck_token() else {
        eprintln!("SKIP: ZENITH_MOTHERDUCK__ACCESS_TOKEN not set");
        return;
    };

    // Connect to MotherDuck using md: protocol
    let conn = Connection::open(format!("md:?motherduck_token={token}")).unwrap();

    // Create a test database (or use if exists)
    conn.execute_batch("CREATE DATABASE IF NOT EXISTS zenith_spike_test")
        .unwrap();

    conn.execute_batch("USE zenith_spike_test").unwrap();

    // Create a test table
    let table = format!(
        "spike_md_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis()
    );

    conn.execute(
        &format!(
            "CREATE TABLE IF NOT EXISTS {table} (
                id INTEGER PRIMARY KEY,
                content TEXT NOT NULL,
                embedding FLOAT[384]
            )"
        ),
        [],
    )
    .unwrap();

    // Insert data
    let emb = synthetic_embedding(42);
    let emb_sql = vec_to_sql(&emb);
    conn.execute(
        &format!("INSERT INTO {table} VALUES (1, 'test from rust', {emb_sql}::FLOAT[384])"),
        [],
    )
    .unwrap();

    // Query back
    let (id, content): (i32, String) = conn
        .query_row(
            &format!("SELECT id, content FROM {table} WHERE id = 1"),
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();

    assert_eq!(id, 1);
    assert_eq!(content, "test from rust");

    // Verify FLOAT[384] round-trip via array_length
    let dims: i64 = conn
        .query_row(
            &format!("SELECT array_length(embedding) FROM {table} WHERE id = 1"),
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(dims, 384);

    // Clean up
    conn.execute(&format!("DROP TABLE IF EXISTS {table}"), [])
        .unwrap();
}

/// Verify that VSS works through MotherDuck (HNSW + similarity search on remote data).
#[test]
fn spike_motherduck_vss() {
    let Some(token) = motherduck_token() else {
        eprintln!("SKIP: ZENITH_MOTHERDUCK__ACCESS_TOKEN not set");
        return;
    };

    let conn = Connection::open(format!("md:?motherduck_token={token}")).unwrap();
    conn.execute_batch("INSTALL vss; LOAD vss;").unwrap();
    conn.execute_batch("CREATE DATABASE IF NOT EXISTS zenith_spike_test; USE zenith_spike_test;")
        .unwrap();

    let table = format!(
        "spike_vss_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis()
    );

    conn.execute_batch(&format!(
        "CREATE TABLE {table} (
            id INTEGER PRIMARY KEY,
            name TEXT NOT NULL,
            embedding FLOAT[384]
        )"
    ))
    .unwrap();

    // Insert vectors
    for (id, name, seed) in [
        (1, "spawn", 10u32),
        (2, "connect", 50),
        (3, "serialize", 90),
    ] {
        let emb = synthetic_embedding(seed);
        let emb_sql = vec_to_sql(&emb);
        conn.execute(
            &format!("INSERT INTO {table} VALUES ({id}, '{name}', {emb_sql}::FLOAT[384])"),
            [],
        )
        .unwrap();
    }

    // Similarity search on MotherDuck
    let query = synthetic_embedding(10);
    let query_sql = vec_to_sql(&query);

    let top_name: String = conn
        .query_row(
            &format!(
                "SELECT name FROM {table}
                 ORDER BY array_cosine_similarity(embedding, {query_sql}::FLOAT[384]) DESC
                 LIMIT 1"
            ),
            [],
            |row| row.get(0),
        )
        .unwrap();

    assert_eq!(
        top_name, "spawn",
        "nearest neighbor to spawn embedding should be spawn"
    );

    // Clean up
    conn.execute(&format!("DROP TABLE IF EXISTS {table}"), [])
        .unwrap();
}

// ===========================================================================
// Part 3: R2 — Cloud Storage (Parquet via httpfs/S3)
// ===========================================================================

struct R2Creds {
    account_id: String,
    access_key_id: String,
    secret_access_key: String,
    bucket_name: String,
}

fn r2_credentials() -> Option<R2Creds> {
    load_env();
    Some(R2Creds {
        account_id: std::env::var("ZENITH_R2__ACCOUNT_ID").ok()?,
        access_key_id: std::env::var("ZENITH_R2__ACCESS_KEY_ID").ok()?,
        secret_access_key: std::env::var("ZENITH_R2__SECRET_ACCESS_KEY").ok()?,
        bucket_name: std::env::var("ZENITH_R2__BUCKET_NAME").ok()?,
    })
}

/// Verify that we can write a Parquet file to R2 and read it back via DuckDB httpfs.
#[test]
fn spike_r2_parquet_roundtrip() {
    let Some(creds) = r2_credentials() else {
        eprintln!("SKIP: R2 credentials not set");
        return;
    };

    let conn = Connection::open_in_memory().unwrap();
    conn.execute_batch("INSTALL httpfs; LOAD httpfs;").unwrap();

    // Configure S3 secret for R2
    conn.execute_batch(&format!(
        "CREATE SECRET r2_spike (
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

    // Create a test table with data
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis();
    let r2_path = format!(
        "s3://{bucket}/zenith-spike/test_{ts}.parquet",
        bucket = creds.bucket_name,
    );

    conn.execute_batch(
        "CREATE TABLE spike_data (
            id INTEGER,
            name TEXT,
            kind TEXT,
            score FLOAT
        )",
    )
    .unwrap();

    conn.execute_batch(
        "INSERT INTO spike_data VALUES
         (1, 'spawn', 'function', 0.95),
         (2, 'connect', 'function', 0.88),
         (3, 'Runtime', 'struct', 0.72)",
    )
    .unwrap();

    // Write to R2 as Parquet
    conn.execute_batch(&format!("COPY spike_data TO '{r2_path}' (FORMAT PARQUET)"))
        .unwrap();

    // Read back from R2
    let count: i64 = conn
        .query_row(
            &format!("SELECT count(*) FROM read_parquet('{r2_path}')"),
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(count, 3, "should read back 3 rows from R2 parquet");

    // Verify data integrity
    let (name, score): (String, f64) = conn
        .query_row(
            &format!("SELECT name, score FROM read_parquet('{r2_path}') WHERE id = 1"),
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    assert_eq!(name, "spawn");
    assert!((score - 0.95).abs() < 0.001);

    // Clean up: delete the test file from R2
    // DuckDB doesn't have a built-in S3 delete, so we leave it.
    // The file is tiny and timestamped, won't accumulate.
}

/// Verify that Parquet files with FLOAT[384] embeddings survive the R2 roundtrip.
#[test]
fn spike_r2_embedding_parquet() {
    let Some(creds) = r2_credentials() else {
        eprintln!("SKIP: R2 credentials not set");
        return;
    };

    let conn = Connection::open_in_memory().unwrap();
    conn.execute_batch("INSTALL httpfs; LOAD httpfs;").unwrap();

    conn.execute_batch(&format!(
        "CREATE SECRET r2_emb (
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

    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis();
    let r2_path = format!(
        "s3://{bucket}/zenith-spike/emb_{ts}.parquet",
        bucket = creds.bucket_name,
    );

    // Create table with embeddings
    conn.execute_batch(
        "CREATE TABLE emb_data (
            id INTEGER,
            name TEXT,
            embedding FLOAT[384]
        )",
    )
    .unwrap();

    // Insert 10 rows with 384-dim embeddings
    for i in 0..10 {
        let emb = synthetic_embedding(i);
        let emb_sql = vec_to_sql(&emb);
        conn.execute(
            &format!("INSERT INTO emb_data VALUES ({i}, 'sym_{i}', {emb_sql}::FLOAT[384])"),
            [],
        )
        .unwrap();
    }

    // Write to R2
    conn.execute_batch(&format!("COPY emb_data TO '{r2_path}' (FORMAT PARQUET)"))
        .unwrap();

    // Read back and verify dimensions preserved
    let dims: i64 = conn
        .query_row(
            &format!("SELECT array_length(embedding) FROM read_parquet('{r2_path}') WHERE id = 0"),
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(
        dims, 384,
        "embedding dimensions should survive parquet roundtrip"
    );

    // Load VSS and do similarity search on R2-stored data.
    // NOTE: Parquet roundtrip converts FLOAT[384] → FLOAT[] (variable-length array).
    // Must cast back to FLOAT[384] for array_cosine_similarity() which requires
    // fixed-size arrays: FLOAT[ANY] not FLOAT[].
    conn.execute_batch("INSTALL vss; LOAD vss;").unwrap();

    let query = synthetic_embedding(5);
    let query_sql = vec_to_sql(&query);

    let top_name: String = conn
        .query_row(
            &format!(
                "SELECT name FROM read_parquet('{r2_path}')
                 ORDER BY array_cosine_similarity(embedding::FLOAT[384], {query_sql}::FLOAT[384]) DESC
                 LIMIT 1"
            ),
            [],
            |row| row.get(0),
        )
        .unwrap();

    assert_eq!(
        top_name, "sym_5",
        "nearest neighbor to seed=5 should be sym_5"
    );
}

// ===========================================================================
// Part 4: Lance — Columnar Format with Native Vector + FTS + Hybrid Search
// ===========================================================================
//
// Lance is an open lakehouse format designed for ML/AI workloads with built-in:
// - Vector indexes (IVF-PQ, persistent, no HNSW crash issues)
// - Full-text search (BM25, auto-built)
// - Hybrid search (vector + FTS with alpha blending)
// - S3/R2 support (s3://bucket/path.lance)
// - DuckDB integration via community extension
//
// This could REPLACE our Parquet + VSS HNSW approach entirely:
//   Before: Parquet on R2 + in-memory HNSW (crashes on persist) + manual hybrid
//   After:  Lance on R2 + native vector/FTS/hybrid search (persistent, battle-tested)
//
// Key functions:
// - lance_vector_search(path, vec_col, query_vec, k=N) → _distance
// - lance_fts(path, text_col, query_text, k=N) → _score
// - lance_hybrid_search(path, vec_col, vec, text_col, text, k=N, alpha=0.5) → _hybrid_score

/// Helper: create a DuckDB connection with the lance community extension loaded.
fn lance_conn() -> Connection {
    let conn = Connection::open_in_memory().expect("failed to open DuckDB");
    conn.execute_batch("INSTALL lance FROM community; LOAD lance;")
        .expect("failed to install/load lance extension");
    conn
}

/// Verify that the lance community extension installs and loads.
#[test]
fn spike_lance_extension_loads() {
    let conn = lance_conn();

    let loaded: bool = conn
        .query_row(
            "SELECT loaded FROM duckdb_extensions() WHERE extension_name = 'lance'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert!(loaded, "lance extension should be loaded");
}

/// Verify writing a DuckDB table to a local .lance dataset and reading it back.
#[test]
fn spike_lance_write_and_scan() {
    let conn = lance_conn();
    let dir = TempDir::new().unwrap();
    let lance_path = dir.path().join("symbols.lance");
    let lance_str = lance_path.to_str().unwrap();

    // Create source table with embeddings (small 4-dim for speed)
    conn.execute_batch(
        "CREATE TABLE symbols AS SELECT
            i as id,
            CASE i % 3 WHEN 0 THEN 'spawn' WHEN 1 THEN 'connect' ELSE 'serialize' END as name,
            'function' as kind,
            apply(generate_series(1,4), j -> CAST(hash(i*1000+j) AS FLOAT)/18446744073709551615) as vec
        FROM generate_series(1, 100) s(i)",
    )
    .unwrap();

    // Write to lance format
    conn.execute_batch(&format!("COPY symbols TO '{lance_str}' (FORMAT lance)"))
        .unwrap();

    // Read back via lance scan
    let count: i64 = conn
        .query_row(&format!("SELECT count(*) FROM '{lance_str}'"), [], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(count, 100);

    // Verify schema survived
    let name: String = conn
        .query_row(
            &format!("SELECT name FROM '{lance_str}' WHERE id = 3"),
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(name, "spawn"); // 3 % 3 == 0 → spawn
}

/// Verify lance_vector_search: nearest neighbor search on a .lance dataset.
/// This is the key function that replaces HNSW + array_cosine_similarity.
#[test]
fn spike_lance_vector_search() {
    let conn = lance_conn();
    let dir = TempDir::new().unwrap();
    let lance_str = dir.path().join("symbols.lance");
    let lance_str = lance_str.to_str().unwrap();

    // Create dataset with known clusters:
    // IDs 1-5: "spawn cluster" (similar 4-dim vectors)
    // IDs 6-10: "connect cluster" (different vectors)
    conn.execute_batch(
        "CREATE TABLE symbols AS
         SELECT * FROM (VALUES
            (1, 'spawn',          [0.9, 0.8, 0.1, 0.1]::FLOAT[4]),
            (2, 'spawn_blocking', [0.85, 0.75, 0.15, 0.12]::FLOAT[4]),
            (3, 'spawn_local',    [0.88, 0.82, 0.08, 0.11]::FLOAT[4]),
            (4, 'connect',        [0.1, 0.1, 0.9, 0.8]::FLOAT[4]),
            (5, 'Connection',     [0.12, 0.15, 0.85, 0.78]::FLOAT[4]),
            (6, 'serialize',      [0.5, 0.5, 0.5, 0.5]::FLOAT[4])
         ) AS t(id, name, vec)",
    )
    .unwrap();

    conn.execute_batch(&format!("COPY symbols TO '{lance_str}' (FORMAT lance)"))
        .unwrap();

    // Search for vectors near "spawn" cluster
    let mut stmt = conn
        .prepare(&format!(
            "SELECT name, _distance
             FROM lance_vector_search('{lance_str}', 'vec', [0.9, 0.8, 0.1, 0.1]::FLOAT[4], k=3)
             ORDER BY _distance ASC"
        ))
        .unwrap();

    let results: Vec<(String, f32)> = stmt
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();

    assert_eq!(results.len(), 3);
    assert_eq!(
        results[0].0, "spawn",
        "closest to spawn query should be spawn"
    );
    assert!(
        results[0].1 < 0.01,
        "exact match distance should be ~0, got {}",
        results[0].1
    );

    // Top 3 should all be from the spawn cluster
    let top_names: Vec<&str> = results.iter().map(|r| r.0.as_str()).collect();
    assert!(
        !top_names.contains(&"connect") && !top_names.contains(&"Connection"),
        "connect cluster should not be in top 3 for spawn query: {top_names:?}"
    );
}

/// Verify lance_fts: full-text search (BM25) on a .lance dataset.
/// Lance auto-builds the FTS index on first query.
#[test]
fn spike_lance_fts_search() {
    let conn = lance_conn();
    let dir = TempDir::new().unwrap();
    let lance_str = dir.path().join("docs.lance");
    let lance_str = lance_str.to_str().unwrap();

    conn.execute_batch(
        "CREATE TABLE docs AS
         SELECT * FROM (VALUES
            (1, 'Tokio spawning tasks uses spawn and spawn_blocking', [0.9, 0.8, 0.1, 0.1]::FLOAT[4]),
            (2, 'Connection pooling reduces database overhead',       [0.1, 0.1, 0.9, 0.8]::FLOAT[4]),
            (3, 'Spawned tasks run concurrently on the executor',     [0.85, 0.75, 0.15, 0.12]::FLOAT[4]),
            (4, 'Serialization framework for Rust structs',           [0.5, 0.5, 0.5, 0.5]::FLOAT[4]),
            (5, 'The spawn function creates a new async task',        [0.88, 0.82, 0.08, 0.11]::FLOAT[4])
         ) AS t(id, content, vec)",
    )
    .unwrap();

    conn.execute_batch(&format!("COPY docs TO '{lance_str}' (FORMAT lance)"))
        .unwrap();

    // FTS search for "spawn"
    let mut stmt = conn
        .prepare(&format!(
            "SELECT id, content, _score
             FROM lance_fts('{lance_str}', 'content', 'spawn', k=5)
             ORDER BY _score DESC"
        ))
        .unwrap();

    let results: Vec<(i64, String, f32)> = stmt
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();

    // Should match docs containing the exact term "spawn" (BM25 is term-based).
    // NOTE: Lance FTS uses BM25 which matches exact terms. "spawning" and "spawned"
    // may or may NOT match "spawn" depending on tokenizer stemming config.
    // Doc 5 ("The spawn function...") has exact "spawn" and always matches.
    assert!(
        !results.is_empty(),
        "FTS should find documents mentioning spawn"
    );

    let matched_ids: Vec<i64> = results.iter().map(|r| r.0).collect();
    assert!(
        matched_ids.contains(&5),
        "doc 5 has exact 'spawn': {matched_ids:?}"
    );

    // Doc 2 (connection pooling) should NOT match "spawn"
    assert!(
        !matched_ids.contains(&2),
        "doc 2 should not match 'spawn': {matched_ids:?}"
    );

    // NOTE: Doc 1 ("spawning") and doc 3 ("Spawned") may or may not match "spawn"
    // depending on Lance's BM25 tokenizer stemming. In practice, only doc 5
    // ("The spawn function") matched with exact term "spawn". This is a useful
    // finding — Lance FTS is term-exact by default, not stemmed like SQLite FTS5.
}

/// Verify lance_hybrid_search: combined vector + FTS scoring.
/// This is the exact pattern zenith needs for `zen search`.
#[test]
fn spike_lance_hybrid_search() {
    let conn = lance_conn();
    let dir = TempDir::new().unwrap();
    let lance_str = dir.path().join("docs.lance");
    let lance_str = lance_str.to_str().unwrap();

    conn.execute_batch(
        "CREATE TABLE docs AS
         SELECT * FROM (VALUES
            (1, 'Tokio spawning tasks uses spawn and spawn_blocking', [0.9, 0.8, 0.1, 0.1]::FLOAT[4]),
            (2, 'Connection pooling reduces database overhead',       [0.1, 0.1, 0.9, 0.8]::FLOAT[4]),
            (3, 'Spawned tasks run concurrently on the executor',     [0.85, 0.75, 0.15, 0.12]::FLOAT[4]),
            (4, 'Serialization framework for Rust structs',           [0.5, 0.5, 0.5, 0.5]::FLOAT[4]),
            (5, 'The spawn function creates a new async task',        [0.88, 0.82, 0.08, 0.11]::FLOAT[4])
         ) AS t(id, content, vec)",
    )
    .unwrap();

    conn.execute_batch(&format!("COPY docs TO '{lance_str}' (FORMAT lance)"))
        .unwrap();

    // Hybrid search: vector near "spawn cluster" + text matching "spawn"
    // alpha=0.5 means equal weight to vector and FTS
    let mut stmt = conn
        .prepare(&format!(
            "SELECT id, content, _hybrid_score, _distance, _score
             FROM lance_hybrid_search(
                 '{lance_str}',
                 'vec', [0.9, 0.8, 0.1, 0.1]::FLOAT[4],
                 'content', 'spawn',
                 k=5, alpha=0.5, oversample_factor=4
             )
             ORDER BY _hybrid_score DESC"
        ))
        .unwrap();

    let results: Vec<(i64, String, f32)> = stmt
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();

    assert!(!results.is_empty(), "hybrid search should return results");

    // Top result should be doc 1 or 5 — they match BOTH vector similarity AND text
    let top_id = results[0].0;
    assert!(
        top_id == 1 || top_id == 3 || top_id == 5,
        "top hybrid result should be a spawn-related doc (both vector + text match), got id={top_id}"
    );

    // Doc 2 (connection pooling) has opposite vector AND no text match — should rank low
    if let Some(pos) = results.iter().position(|r| r.0 == 2) {
        assert!(
            pos >= 3,
            "connection pooling doc should rank low in hybrid search, got position {pos}"
        );
    }
}

/// Verify Lance on R2: write .lance dataset to S3-compatible storage, search from there.
/// This validates the full cloud path: DuckDB → Lance → R2 → search.
///
/// NOTE: Lance uses its own AWS credential chain (via the lance-io crate), NOT DuckDB's
/// S3 secrets. Credentials must be set via environment variables BEFORE the process starts:
///   AWS_ACCESS_KEY_ID, AWS_SECRET_ACCESS_KEY, AWS_ENDPOINT_URL, AWS_DEFAULT_REGION
///
/// These are loaded from `.env` via `dotenvy` (see `load_env()`). The `.env` file maps
/// R2 credentials to AWS-style variable names that Lance expects.
#[test]
fn spike_lance_on_r2() {
    // load_env() loads .env which includes AWS_* vars for Lance's credential chain
    load_env();

    let Some(creds) = r2_credentials() else {
        eprintln!("SKIP: R2 credentials not set");
        return;
    };

    // Verify Lance AWS vars are loaded (dotenvy populates them from .env)
    if std::env::var("AWS_ACCESS_KEY_ID").is_err() {
        eprintln!("SKIP: AWS_ACCESS_KEY_ID not set (Lance needs AWS-style env vars for R2)");
        return;
    }

    let conn = Connection::open_in_memory().unwrap();
    conn.execute_batch("INSTALL lance FROM community; LOAD lance;")
        .unwrap();

    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis();
    let r2_path = format!(
        "s3://{bucket}/zenith-spike/lance_{ts}.lance",
        bucket = creds.bucket_name,
    );

    // Create dataset
    conn.execute_batch(
        "CREATE TABLE symbols AS
         SELECT * FROM (VALUES
            (1, 'spawn',     [0.9, 0.8, 0.1, 0.1]::FLOAT[4]),
            (2, 'connect',   [0.1, 0.1, 0.9, 0.8]::FLOAT[4]),
            (3, 'serialize', [0.5, 0.5, 0.5, 0.5]::FLOAT[4])
         ) AS t(id, name, vec)",
    )
    .unwrap();

    // Write lance to R2
    conn.execute_batch(&format!("COPY symbols TO '{r2_path}' (FORMAT lance)"))
        .unwrap();

    // Read back from R2
    let count: i64 = conn
        .query_row(&format!("SELECT count(*) FROM '{r2_path}'"), [], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(count, 3, "should read 3 rows from lance on R2");

    // Vector search on R2-stored lance dataset
    let top_name: String = conn
        .query_row(
            &format!(
                "SELECT name FROM lance_vector_search('{r2_path}', 'vec', [0.9, 0.8, 0.1, 0.1]::FLOAT[4], k=1)
                 ORDER BY _distance ASC LIMIT 1"
            ),
            [],
            |row| row.get(0),
        )
        .unwrap();

    assert_eq!(
        top_name, "spawn",
        "vector search on R2 lance should find spawn"
    );
}

// ===========================================================================
// Part 5: DuckLake on MotherDuck — Managed Lakehouse with Snapshots
// ===========================================================================
//
// DuckLake is a lakehouse format that stores catalog metadata in a SQL database
// (MotherDuck, PostgreSQL, SQLite) and data files in Parquet on object storage.
//
// Two modes:
// 1. Fully managed: `CREATE DATABASE my_lake (TYPE DUCKLAKE)` — MotherDuck stores everything
// 2. Custom storage: `CREATE DATABASE my_lake (TYPE DUCKLAKE, DATA_PATH 's3://...')` —
//    requires S3 bucket in same region as MotherDuck org (us-east-1)
//
// Key features:
// - ACID transactions across table operations
// - Time travel via snapshots (`ducklake_snapshots()`)
// - Schema evolution
// - Works with standard DuckDB SQL (CREATE TABLE, INSERT, SELECT)
//
// FINDING: Our R2 bucket (aether-data) is in eu-west-2 but MotherDuck is in us-east-1.
// Custom DATA_PATH to R2 requires same-region. For now, use fully managed mode or
// create a us-east-1 R2 bucket. This is a Phase 8 deployment concern, not a blocker.

/// Verify that DuckLake works on MotherDuck in fully managed mode:
/// CREATE DATABASE, CREATE TABLE, INSERT, SELECT, snapshots.
#[test]
fn spike_ducklake_managed() {
    let Some(token) = motherduck_token() else {
        eprintln!("SKIP: ZENITH_MOTHERDUCK__ACCESS_TOKEN not set");
        return;
    };

    let conn = Connection::open(format!("md:?motherduck_token={token}")).unwrap();
    conn.execute_batch("INSTALL ducklake;").unwrap();

    // Create fully managed DuckLake (MotherDuck handles storage)
    conn.execute_batch("CREATE DATABASE IF NOT EXISTS zenith_ducklake_spike (TYPE DUCKLAKE)")
        .unwrap();

    conn.execute_batch("USE zenith_ducklake_spike").unwrap();

    let table = format!(
        "spike_dl_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis()
    );

    // Create table and insert data
    conn.execute_batch(&format!(
        "CREATE TABLE IF NOT EXISTS {table} (
            id INTEGER,
            name TEXT,
            kind TEXT
        )"
    ))
    .unwrap();

    conn.execute_batch(&format!(
        "INSERT INTO {table} VALUES
            (1, 'spawn', 'function'),
            (2, 'connect', 'struct'),
            (3, 'serialize', 'function')"
    ))
    .unwrap();

    // Query back
    let count: i64 = conn
        .query_row(&format!("SELECT count(*) FROM {table}"), [], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(count, 3, "DuckLake should have 3 rows");

    let name: String = conn
        .query_row(
            &format!("SELECT name FROM {table} WHERE id = 1"),
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(name, "spawn");

    // Verify snapshots exist (DuckLake tracks all operations)
    let snapshot_count: i64 = conn
        .query_row(
            "SELECT count(*) FROM ducklake_snapshots('zenith_ducklake_spike')",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert!(
        snapshot_count >= 2,
        "DuckLake should have at least 2 snapshots (create + insert), got {snapshot_count}"
    );

    // Clean up
    conn.execute(&format!("DROP TABLE IF EXISTS {table}"), [])
        .unwrap();
}

/// Verify DuckLake with custom DATA_PATH on R2 (zenith bucket, us-east-1).
///
/// Creates a DuckLake with MotherDuck catalog + R2 Parquet storage.
/// Tests: CREATE TABLE, INSERT, SELECT, snapshots, embedding storage.
///
/// FINDING: DuckLake does NOT support `FLOAT[N]` (fixed-size arrays).
/// Must use `FLOAT[]` (variable-length) for embeddings in DuckLake tables.
/// This means `array_cosine_similarity()` (which needs `FLOAT[ANY]`) requires
/// an explicit cast: `embedding::FLOAT[384]` when querying.
#[test]
fn spike_ducklake_r2_with_data() {
    let Some(token) = motherduck_token() else {
        eprintln!("SKIP: ZENITH_MOTHERDUCK__ACCESS_TOKEN not set");
        return;
    };
    let Some(creds) = r2_credentials() else {
        eprintln!("SKIP: R2 credentials not set");
        return;
    };

    let conn = Connection::open(format!("md:?motherduck_token={token}")).unwrap();
    conn.execute_batch("INSTALL ducklake; INSTALL httpfs; LOAD httpfs;")
        .unwrap();

    // Create R2 secret in MotherDuck
    conn.execute_batch(&format!(
        "CREATE OR REPLACE SECRET r2_zenith IN MOTHERDUCK (
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

    // Create DuckLake with R2 storage
    conn.execute_batch(&format!(
        "CREATE DATABASE IF NOT EXISTS zenith_lake (
            TYPE DUCKLAKE,
            DATA_PATH 's3://{bucket}/lake/'
        )",
        bucket = creds.bucket_name,
    ))
    .unwrap();

    conn.execute_batch("USE zenith_lake").unwrap();

    let table = format!(
        "spike_r2_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis()
    );

    // NOTE: DuckLake does NOT support FLOAT[N] (fixed-size arrays).
    // Must use FLOAT[] (variable-length) for embeddings.
    conn.execute_batch(&format!(
        "CREATE TABLE {table} (
            id INTEGER,
            name TEXT,
            kind TEXT,
            embedding FLOAT[]
        )"
    ))
    .unwrap();

    // Insert data with embeddings
    conn.execute_batch(&format!(
        "INSERT INTO {table} VALUES
            (1, 'spawn', 'function', [0.9, 0.8, 0.1, 0.1]),
            (2, 'connect', 'struct', [0.1, 0.1, 0.9, 0.8]),
            (3, 'serialize', 'function', [0.5, 0.5, 0.5, 0.5])"
    ))
    .unwrap();

    // Query back
    let count: i64 = conn
        .query_row(&format!("SELECT count(*) FROM {table}"), [], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(count, 3, "DuckLake on R2 should have 3 rows");

    let name: String = conn
        .query_row(
            &format!("SELECT name FROM {table} WHERE id = 1"),
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(name, "spawn");

    // Verify snapshots (DuckLake tracks every operation)
    let snapshot_count: i64 = conn
        .query_row(
            "SELECT count(*) FROM ducklake_snapshots('zenith_lake')",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert!(
        snapshot_count >= 2,
        "DuckLake should have at least 2 snapshots, got {snapshot_count}"
    );

    // Verify cosine similarity works with FLOAT[] → FLOAT[4] cast
    conn.execute_batch("INSTALL vss; LOAD vss;").unwrap();

    let top_name: String = conn
        .query_row(
            &format!(
                "SELECT name FROM {table}
                 ORDER BY array_cosine_similarity(embedding::FLOAT[4], [0.9, 0.8, 0.1, 0.1]::FLOAT[4]) DESC
                 LIMIT 1"
            ),
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(
        top_name, "spawn",
        "vector search on DuckLake should find spawn"
    );

    // Clean up
    conn.execute(&format!("DROP TABLE IF EXISTS {table}"), [])
        .unwrap();
}
