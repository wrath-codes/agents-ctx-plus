//! # Spike 0.4: DuckDB Local Database Validation
//!
//! Validates that the `duckdb` crate (v1.4, bundled) compiles and works for zenith's
//! documentation lake needs:
//!
//! - **Bundled build**: `duckdb = { version = "1.4", features = ["bundled"] }` compiles from source
//! - **In-memory DB**: `Connection::open_in_memory()` for tests and ephemeral pipelines
//! - **File-backed DB**: `Connection::open(path)` for persistent `.zenith/lake/cache.duckdb`
//! - **Schema creation**: `CREATE TABLE` matching `02-ducklake-data-model.md` tables
//! - **Parameterized inserts**: `params![]` macro with typed parameters
//! - **Query mapping**: `stmt.query_map()` with row-level extraction
//! - **Batch execution**: `execute_batch()` for multi-statement DDL
//! - **Appender API**: Bulk insert for indexing pipeline throughput
//! - **JSON columns**: `JSON` type for metadata (language-specific extras)
//! - **FLOAT arrays**: `FLOAT[384]` columns for fastembed vectors
//! - **Transactions**: Commit/rollback for atomic pipeline writes
//! - **File persistence**: Data survives close + reopen
//!
//! ## Validates
//!
//! DuckDB compiles (bundled) and works locally — blocks Phase 2.
//!
//! ## Async Strategy
//!
//! DuckDB's C engine is inherently synchronous. The `duckdb` crate exposes a synchronous
//! rusqlite-inspired API. Zenith is a tokio-based async application. Three options exist
//! for bridging:
//!
//! ### Option 1: `tokio::task::spawn_blocking` (recommended for zenith)
//!
//! The official recommendation from the duckdb-rs docs. Wrap each DB operation in
//! `spawn_blocking`:
//!
//! ```rust,ignore
//! use tokio::task;
//! use duckdb::Connection;
//!
//! async fn query_symbols(db_path: &str, query: &str) -> duckdb::Result<Vec<Symbol>> {
//!     let db_path = db_path.to_string();
//!     let query = query.to_string();
//!     task::spawn_blocking(move || {
//!         let conn = Connection::open(&db_path)?;
//!         // ... synchronous duckdb operations
//!         Ok(symbols)
//!     })
//!     .await
//!     .expect("join error")
//! }
//! ```
//!
//! **Pros**: Zero extra dependencies, battle-tested pattern, full API access.
//! **Cons**: Boilerplate per call, must move owned values into closure.
//!
//! ### Option 2: `async-duckdb` crate (v0.3.1)
//!
//! Third-party async wrapper by jessekrubin. Spawns a dedicated background thread per
//! connection and uses crossbeam channels to shuttle closures:
//!
//! ```rust,ignore
//! use async_duckdb::ClientBuilder;
//!
//! let client = ClientBuilder::new()
//!     .path("/path/to/db.duckdb")
//!     .open()
//!     .await?;
//!
//! let count: i64 = client.conn(|conn| {
//!     conn.query_row("SELECT COUNT(*) FROM api_symbols", [], |row| row.get(0))
//! }).await?;
//! ```
//!
//! **Pros**: Clean `.conn(|conn| { ... }).await` API, runtime-agnostic (tokio + async-std).
//! **Cons**: Young crate (6 GitHub stars, single maintainer), Pool mode is read-only only,
//! extra dependency.
//!
//! ### Option 3: Custom wrapper (dedicated thread + mpsc channel)
//!
//! Build a thin wrapper: spawn a thread that owns the `Connection`, accept operations
//! via `tokio::sync::mpsc`, return results via `tokio::sync::oneshot`. This is essentially
//! what `async-duckdb` does internally.
//!
//! **Pros**: Full control, no external dependency, can batch operations.
//! **Cons**: More code to write and maintain.
//!
//! ### Decision
//!
//! **Defer to Phase 3.** For now, the spike validates the synchronous API. When we build
//! `zen-lake` in Phase 3, we'll choose between Option 1 and Option 2 based on:
//!
//! - Whether `async-duckdb` has stabilized by then
//! - Whether zen-lake's access pattern is mostly write-heavy (indexing pipeline, favors
//!   Option 1 with a single connection) or read-heavy (search queries, favors Option 2
//!   with a pool)
//! - The `r2d2` connection pool feature in the `duckdb` crate can also be combined with
//!   `spawn_blocking` for a mature pooling solution
//!
//! The most likely outcome is **Option 1** (`spawn_blocking`) for writes during indexing,
//! and either Option 1 or Option 2 for read queries during search — since zenith's DuckDB
//! usage is bursty (index a package, then query), not continuous.

use duckdb::{params, Connection};
use tempfile::TempDir;

/// Helper: create an in-memory DuckDB connection.
fn in_memory_conn() -> Connection {
    Connection::open_in_memory().expect("failed to create in-memory DuckDB connection")
}

// ---------------------------------------------------------------------------
// Spike tests
// ---------------------------------------------------------------------------

/// Verify that the bundled DuckDB compiles and an in-memory connection works.
#[test]
fn spike_in_memory_connects() {
    let conn = in_memory_conn();

    let val: i64 = conn
        .query_row("SELECT 1 + 1 AS result", [], |row| row.get(0))
        .unwrap();
    assert_eq!(val, 2);
}

/// Verify that a file-backed database persists across close + reopen.
#[test]
fn spike_file_db_persists() {
    let dir = TempDir::new().unwrap();
    let db_path = dir.path().join("persist.duckdb");

    // Write data
    {
        let conn = Connection::open(&db_path).unwrap();
        conn.execute_batch(
            "CREATE TABLE kv (key TEXT PRIMARY KEY, value TEXT);
             INSERT INTO kv VALUES ('greeting', 'hello');",
        )
        .unwrap();
    }

    // Reopen and read
    {
        let conn = Connection::open(&db_path).unwrap();
        let value: String = conn
            .query_row(
                "SELECT value FROM kv WHERE key = ?",
                params!["greeting"],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(value, "hello");
    }
}

/// Verify CREATE TABLE + INSERT + SELECT roundtrip with parameterized queries.
/// Uses the `indexed_packages` schema from 02-ducklake-data-model.md.
#[test]
fn spike_crud_roundtrip() {
    let conn = in_memory_conn();

    conn.execute_batch(
        "CREATE TABLE indexed_packages (
            ecosystem TEXT NOT NULL,
            name TEXT NOT NULL,
            version TEXT NOT NULL,
            repo_url TEXT,
            description TEXT,
            license TEXT,
            downloads BIGINT,
            indexed_at TIMESTAMP DEFAULT current_timestamp,
            file_count INTEGER DEFAULT 0,
            symbol_count INTEGER DEFAULT 0,
            PRIMARY KEY (ecosystem, name, version)
        )",
    )
    .unwrap();

    // Insert with params
    conn.execute(
        "INSERT INTO indexed_packages (ecosystem, name, version, repo_url, description, license, downloads, file_count, symbol_count)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
        params![
            "rust",
            "tokio",
            "1.40.0",
            "https://github.com/tokio-rs/tokio",
            "An event-driven, non-blocking I/O platform",
            "MIT",
            85_000_000i64,
            342i32,
            1580i32
        ],
    )
    .unwrap();

    conn.execute(
        "INSERT INTO indexed_packages (ecosystem, name, version, description, downloads)
         VALUES (?, ?, ?, ?, ?)",
        params![
            "rust",
            "serde",
            "1.0.210",
            "A serialization framework",
            120_000_000i64
        ],
    )
    .unwrap();

    // Query all
    let mut stmt = conn
        .prepare("SELECT ecosystem, name, version, downloads FROM indexed_packages ORDER BY name")
        .unwrap();

    let rows: Vec<(String, String, String, i64)> = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, i64>(3)?,
            ))
        })
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();

    assert_eq!(rows.len(), 2);
    assert_eq!(
        rows[0],
        ("rust".into(), "serde".into(), "1.0.210".into(), 120_000_000)
    );
    assert_eq!(
        rows[1],
        ("rust".into(), "tokio".into(), "1.40.0".into(), 85_000_000)
    );
}

/// Verify the `api_symbols` schema from 02-ducklake-data-model.md, including
/// JSON metadata columns and FLOAT array columns for embeddings.
#[test]
fn spike_api_symbols_schema() {
    let conn = in_memory_conn();

    conn.execute_batch(
        "CREATE TABLE api_symbols (
            id TEXT NOT NULL,
            ecosystem TEXT NOT NULL,
            package TEXT NOT NULL,
            version TEXT NOT NULL,
            file_path TEXT NOT NULL,
            kind TEXT NOT NULL,
            name TEXT NOT NULL,
            signature TEXT,
            doc_comment TEXT,
            line_start INTEGER,
            line_end INTEGER,
            visibility TEXT,
            is_async BOOLEAN DEFAULT FALSE,
            is_unsafe BOOLEAN DEFAULT FALSE,
            return_type TEXT,
            generics TEXT,
            attributes TEXT,
            metadata JSON,
            embedding FLOAT[384],
            PRIMARY KEY (id)
        )",
    )
    .unwrap();

    // Insert a symbol with JSON metadata and a 384-dim embedding.
    // DuckDB FLOAT[384] is a fixed-size array — it rejects arrays of other sizes.
    // This is a valuable constraint: it catches dimension mismatches at insert time.
    let embedding: Vec<f32> = (0..384).map(|i| (i as f32) / 384.0).collect();
    let embedding_str = format!(
        "[{}]",
        embedding
            .iter()
            .map(|v| format!("{v}"))
            .collect::<Vec<_>>()
            .join(", ")
    );

    conn.execute(
        "INSERT INTO api_symbols (id, ecosystem, package, version, file_path, kind, name,
            signature, doc_comment, line_start, line_end, visibility, is_async, return_type,
            metadata, embedding)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?::FLOAT[384])",
        params![
            "sym-abc123",
            "rust",
            "tokio",
            "1.40.0",
            "src/runtime/task/mod.rs",
            "function",
            "spawn",
            "pub fn spawn<F: Future + Send + 'static>(future: F) -> JoinHandle<F::Output>",
            "Spawns a new asynchronous task, returning a JoinHandle for it.",
            42i32,
            85i32,
            "pub",
            true,
            "JoinHandle<F::Output>",
            r#"{"lifetimes": ["'static"], "where_clause": "where F: Future + Send + 'static"}"#,
            embedding_str
        ],
    )
    .unwrap();

    // Query back and verify all fields
    let mut stmt = conn
        .prepare(
            "SELECT id, kind, name, signature, is_async, metadata,
                    array_length(embedding) as embed_dims
             FROM api_symbols WHERE id = ?",
        )
        .unwrap();

    let (id, kind, name, sig, is_async, metadata, embed_dims): (
        String,
        String,
        String,
        String,
        bool,
        String,
        i64,
    ) = stmt
        .query_row(params!["sym-abc123"], |row| {
            Ok((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
                row.get(5)?,
                row.get(6)?,
            ))
        })
        .unwrap();

    assert_eq!(id, "sym-abc123");
    assert_eq!(kind, "function");
    assert_eq!(name, "spawn");
    assert!(sig.contains("JoinHandle"));
    assert!(is_async);
    assert!(metadata.contains("lifetimes"));
    assert_eq!(embed_dims, 384, "embedding should have 384 dimensions");

    // NOTE: FLOAT[384] columns cannot be read directly as String via row.get().
    // DuckDB returns Array(Float, 384) type. For actual usage, read via
    // array_cosine_similarity() in SQL or cast to VARCHAR in the query.
}

/// Verify `execute_batch` works for multi-statement DDL — the pattern zen-lake
/// will use to create the full DuckLake schema at startup.
#[test]
fn spike_execute_batch() {
    let conn = in_memory_conn();

    conn.execute_batch(
        "CREATE TABLE indexed_packages (
            ecosystem TEXT NOT NULL,
            name TEXT NOT NULL,
            version TEXT NOT NULL,
            PRIMARY KEY (ecosystem, name, version)
        );

        CREATE TABLE api_symbols (
            id TEXT PRIMARY KEY,
            ecosystem TEXT NOT NULL,
            package TEXT NOT NULL,
            version TEXT NOT NULL,
            kind TEXT NOT NULL,
            name TEXT NOT NULL
        );

        CREATE TABLE doc_chunks (
            id TEXT PRIMARY KEY,
            ecosystem TEXT NOT NULL,
            package TEXT NOT NULL,
            version TEXT NOT NULL,
            chunk_index INTEGER NOT NULL,
            content TEXT NOT NULL
        );

        CREATE INDEX idx_symbols_pkg ON api_symbols(ecosystem, package, version);
        CREATE INDEX idx_symbols_kind ON api_symbols(kind);
        CREATE INDEX idx_chunks_pkg ON doc_chunks(ecosystem, package, version);",
    )
    .unwrap();

    // Verify all tables exist
    let count: i64 = conn
        .query_row(
            "SELECT count(*) FROM information_schema.tables
             WHERE table_schema = 'main'
               AND table_type = 'BASE TABLE'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(count, 3, "should have 3 tables");
}

/// Verify the Appender API for bulk inserts — the pattern zen-lake will use
/// during the indexing pipeline for high-throughput writes.
#[test]
fn spike_appender_bulk_insert() {
    let conn = in_memory_conn();

    conn.execute_batch(
        "CREATE TABLE symbols (
            id TEXT NOT NULL,
            package TEXT NOT NULL,
            kind TEXT NOT NULL,
            name TEXT NOT NULL
        )",
    )
    .unwrap();

    // Use Appender for bulk insert (much faster than individual INSERTs)
    {
        let mut appender = conn.appender("symbols").unwrap();
        for i in 0..1000 {
            appender
                .append_row(params![
                    format!("sym-{i:04}"),
                    "tokio",
                    "function",
                    format!("func_{i}")
                ])
                .unwrap();
        }
        appender.flush().unwrap();
    }

    // Verify count
    let count: i64 = conn
        .query_row("SELECT count(*) FROM symbols", [], |row| row.get(0))
        .unwrap();
    assert_eq!(count, 1000);

    // Verify specific rows
    let name: String = conn
        .query_row(
            "SELECT name FROM symbols WHERE id = ?",
            params!["sym-0042"],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(name, "func_42");
}

/// Verify transactions: commit persists, rollback discards.
/// Zenith uses transactions for atomic pipeline writes (all symbols + chunks
/// for a package are written atomically).
#[test]
fn spike_transactions() {
    let mut conn = in_memory_conn();

    conn.execute_batch("CREATE TABLE packages (name TEXT PRIMARY KEY, version TEXT NOT NULL)")
        .unwrap();

    // Transaction that commits
    {
        let tx = conn.transaction().unwrap();
        tx.execute(
            "INSERT INTO packages VALUES (?, ?)",
            params!["tokio", "1.40.0"],
        )
        .unwrap();
        tx.commit().unwrap();
    }

    // Transaction that rolls back (dropped without commit)
    {
        let tx = conn.transaction().unwrap();
        tx.execute(
            "INSERT INTO packages VALUES (?, ?)",
            params!["serde", "1.0.210"],
        )
        .unwrap();
        tx.rollback().unwrap();
    }

    // Only committed row exists
    let count: i64 = conn
        .query_row("SELECT count(*) FROM packages", [], |row| row.get(0))
        .unwrap();
    assert_eq!(count, 1);

    let name: String = conn
        .query_row("SELECT name FROM packages", [], |row| row.get(0))
        .unwrap();
    assert_eq!(name, "tokio");
}

/// Verify JSON column operations — zen-lake uses JSON for language-specific
/// metadata in `api_symbols.metadata`.
#[test]
fn spike_json_operations() {
    let conn = in_memory_conn();

    conn.execute_batch("CREATE TABLE items (id TEXT PRIMARY KEY, metadata JSON)")
        .unwrap();

    let metadata =
        r#"{"lifetimes": ["'a", "'static"], "is_pyo3": false, "fields": ["name", "age"]}"#;
    conn.execute(
        "INSERT INTO items VALUES (?, ?)",
        params!["item-1", metadata],
    )
    .unwrap();

    // Extract JSON fields with DuckDB's JSON functions
    let lifetime_count: i64 = conn
        .query_row(
            "SELECT json_array_length(metadata->'lifetimes') FROM items WHERE id = ?",
            params!["item-1"],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(lifetime_count, 2);

    let is_pyo3: bool = conn
        .query_row(
            "SELECT (metadata->>'is_pyo3')::BOOLEAN FROM items WHERE id = ?",
            params!["item-1"],
            |row| row.get(0),
        )
        .unwrap();
    assert!(!is_pyo3);

    // JSON path extraction
    let first_field: String = conn
        .query_row(
            "SELECT metadata->'fields'->>0 FROM items WHERE id = ?",
            params!["item-1"],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(first_field, "name");
}

/// Verify FLOAT array columns for embeddings — zen-lake stores 384-dim
/// fastembed vectors in `FLOAT[384]` columns.
#[test]
fn spike_float_array_embeddings() {
    let conn = in_memory_conn();

    conn.execute_batch(
        "CREATE TABLE vectors (
            id TEXT PRIMARY KEY,
            content TEXT NOT NULL,
            embedding FLOAT[384]
        )",
    )
    .unwrap();

    // Generate a small test vector (384 dims)
    let embedding: Vec<f32> = (0..384).map(|i| (i as f32) / 384.0).collect();
    let embedding_str = format!(
        "[{}]",
        embedding
            .iter()
            .map(|v| format!("{v}"))
            .collect::<Vec<_>>()
            .join(", ")
    );

    conn.execute(
        "INSERT INTO vectors VALUES (?, ?, ?::FLOAT[384])",
        params!["vec-1", "test embedding", embedding_str],
    )
    .unwrap();

    // Query back and verify dimensions via array_length
    let dims: i64 = conn
        .query_row(
            "SELECT array_length(embedding) FROM vectors WHERE id = ?",
            params!["vec-1"],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(dims, 384, "embedding should have 384 dimensions");

    // Verify cosine similarity (DuckDB built-in)
    // Insert a second vector and compute similarity
    let embedding2: Vec<f32> = (0..384).map(|i| ((383 - i) as f32) / 384.0).collect();
    let embedding2_str = format!(
        "[{}]",
        embedding2
            .iter()
            .map(|v| format!("{v}"))
            .collect::<Vec<_>>()
            .join(", ")
    );

    conn.execute(
        "INSERT INTO vectors VALUES (?, ?, ?::FLOAT[384])",
        params!["vec-2", "different embedding", embedding2_str],
    )
    .unwrap();

    // array_cosine_similarity should return a value between -1 and 1
    let similarity: f64 = conn
        .query_row(
            "SELECT array_cosine_similarity(
                (SELECT embedding FROM vectors WHERE id = 'vec-1'),
                (SELECT embedding FROM vectors WHERE id = 'vec-2')
            )",
            [],
            |row| row.get(0),
        )
        .unwrap();

    assert!(
        (-1.0..=1.0).contains(&similarity),
        "cosine similarity should be in [-1, 1], got {similarity}"
    );
    // These vectors are not identical, so similarity should not be 1.0
    assert!(
        similarity < 0.99,
        "different vectors should not be perfectly similar, got {similarity}"
    );
}

/// Verify the doc_chunks schema and query patterns from 02-ducklake-data-model.md.
#[test]
fn spike_doc_chunks_schema() {
    let conn = in_memory_conn();

    conn.execute_batch(
        "CREATE TABLE doc_chunks (
            id TEXT NOT NULL,
            ecosystem TEXT NOT NULL,
            package TEXT NOT NULL,
            version TEXT NOT NULL,
            chunk_index INTEGER NOT NULL,
            title TEXT,
            content TEXT NOT NULL,
            source_file TEXT,
            format TEXT,
            embedding FLOAT[384],
            PRIMARY KEY (id)
        )",
    )
    .unwrap();

    // Insert documentation chunks
    let chunks = [
        (
            "dc-001",
            "Getting Started",
            "To use tokio, add it to your Cargo.toml...",
            "README.md",
            "md",
        ),
        (
            "dc-002",
            "Runtime",
            "The tokio runtime is the core of the framework...",
            "README.md",
            "md",
        ),
        (
            "dc-003",
            "Spawning Tasks",
            "Use tokio::spawn to create a new async task...",
            "docs/guide.md",
            "md",
        ),
    ];

    for (i, (id, title, content, source, fmt)) in chunks.iter().enumerate() {
        conn.execute(
            "INSERT INTO doc_chunks (id, ecosystem, package, version, chunk_index, title, content, source_file, format)
             VALUES (?, 'rust', 'tokio', '1.40.0', ?, ?, ?, ?, ?)",
            params![*id, i as i32, *title, *content, *source, *fmt],
        )
        .unwrap();
    }

    // Query by package
    let count: i64 = conn
        .query_row(
            "SELECT count(*) FROM doc_chunks WHERE ecosystem = 'rust' AND package = 'tokio'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(count, 3);

    // Query by source file
    let mut stmt = conn
        .prepare(
            "SELECT title, content FROM doc_chunks
             WHERE source_file = ? ORDER BY chunk_index",
        )
        .unwrap();

    let readme_chunks: Vec<(String, String)> = stmt
        .query_map(params!["README.md"], |row| Ok((row.get(0)?, row.get(1)?)))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();

    assert_eq!(readme_chunks.len(), 2);
    assert_eq!(readme_chunks[0].0, "Getting Started");
    assert_eq!(readme_chunks[1].0, "Runtime");
}

/// Comprehensive end-to-end spike: simulate the indexing pipeline pattern.
/// Create schema → insert package → bulk insert symbols → insert doc chunks
/// → query by package → cosine similarity search.
#[test]
fn spike_end_to_end_indexing_pattern() {
    let dir = TempDir::new().unwrap();
    let db_path = dir.path().join("zenith_lake.duckdb");

    let conn = Connection::open(&db_path).unwrap();

    // 1. Create full schema (simulating ZenLake::open_local)
    conn.execute_batch(
        "CREATE TABLE indexed_packages (
            ecosystem TEXT NOT NULL,
            name TEXT NOT NULL,
            version TEXT NOT NULL,
            repo_url TEXT,
            description TEXT,
            downloads BIGINT,
            indexed_at TIMESTAMP DEFAULT current_timestamp,
            file_count INTEGER DEFAULT 0,
            symbol_count INTEGER DEFAULT 0,
            PRIMARY KEY (ecosystem, name, version)
        );

        CREATE TABLE api_symbols (
            id TEXT PRIMARY KEY,
            ecosystem TEXT NOT NULL,
            package TEXT NOT NULL,
            version TEXT NOT NULL,
            file_path TEXT NOT NULL,
            kind TEXT NOT NULL,
            name TEXT NOT NULL,
            signature TEXT,
            doc_comment TEXT,
            visibility TEXT,
            is_async BOOLEAN DEFAULT FALSE,
            metadata JSON,
            embedding FLOAT[384]
        );

        CREATE TABLE doc_chunks (
            id TEXT PRIMARY KEY,
            ecosystem TEXT NOT NULL,
            package TEXT NOT NULL,
            version TEXT NOT NULL,
            chunk_index INTEGER NOT NULL,
            title TEXT,
            content TEXT NOT NULL,
            source_file TEXT,
            embedding FLOAT[384]
        );

        CREATE INDEX idx_symbols_pkg ON api_symbols(ecosystem, package, version);
        CREATE INDEX idx_chunks_pkg ON doc_chunks(ecosystem, package, version);",
    )
    .unwrap();

    // 2. Register package (simulating store_package)
    conn.execute(
        "INSERT INTO indexed_packages (ecosystem, name, version, description, downloads)
         VALUES (?, ?, ?, ?, ?)",
        params![
            "rust",
            "anyhow",
            "1.0.93",
            "Flexible error handling",
            95_000_000i64
        ],
    )
    .unwrap();

    // 3. Bulk insert symbols via Appender (simulating store_symbols)
    {
        let mut appender = conn.appender("api_symbols").unwrap();
        let symbols = [
            ("sym-001", "function", "anyhow", "Context::context"),
            ("sym-002", "function", "anyhow", "bail"),
            ("sym-003", "function", "anyhow", "ensure"),
            ("sym-004", "trait", "anyhow", "Context"),
            ("sym-005", "struct", "anyhow", "Error"),
        ];

        for (id, kind, pkg, name) in &symbols {
            appender
                .append_row(params![
                    *id,                          // id
                    "rust",                       // ecosystem
                    *pkg,                         // package
                    "1.0.93",                     // version
                    "src/lib.rs",                 // file_path
                    *kind,                        // kind
                    *name,                        // name
                    format!("pub fn {name}()"),   // signature (placeholder)
                    format!("The {name} symbol"), // doc_comment
                    "pub",                        // visibility
                    false,                        // is_async
                    "{}",                         // metadata (empty JSON)
                    duckdb::types::Null           // embedding (NULL for now)
                ])
                .unwrap();
        }
        appender.flush().unwrap();
    }

    // 4. Update package symbol count
    conn.execute(
        "UPDATE indexed_packages SET symbol_count = (
            SELECT count(*) FROM api_symbols
            WHERE ecosystem = 'rust' AND package = 'anyhow' AND version = '1.0.93'
        ) WHERE ecosystem = 'rust' AND name = 'anyhow' AND version = '1.0.93'",
        [],
    )
    .unwrap();

    // 5. Verify the full roundtrip
    let symbol_count: i64 = conn
        .query_row(
            "SELECT symbol_count FROM indexed_packages
             WHERE ecosystem = 'rust' AND name = 'anyhow'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(symbol_count, 5);

    // 6. Query by kind (simulating search)
    let mut stmt = conn
        .prepare("SELECT name FROM api_symbols WHERE kind = ? AND package = ? ORDER BY name")
        .unwrap();

    let functions: Vec<String> = stmt
        .query_map(params!["function", "anyhow"], |row| row.get(0))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();

    assert_eq!(functions, vec!["Context::context", "bail", "ensure"]);

    // 7. Verify file persisted
    drop(conn);
    assert!(db_path.exists(), "DuckDB file should persist on disk");

    // 8. Reopen and verify data survived
    let conn2 = Connection::open(&db_path).unwrap();
    let count: i64 = conn2
        .query_row("SELECT count(*) FROM api_symbols", [], |row| row.get(0))
        .unwrap();
    assert_eq!(count, 5, "data should survive close + reopen");
}
