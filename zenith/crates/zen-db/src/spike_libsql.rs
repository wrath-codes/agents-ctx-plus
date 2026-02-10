//! # Spike 0.2: libSQL Local Database Validation
//!
//! Validates that the `libsql` crate (v0.9.29) works for zenith's needs:
//!
//! - **Local DB creation**: `Builder::new_local()` (including `:memory:`)
//! - **SQL execution**: CREATE TABLE, INSERT with parameterized queries
//! - **Row querying**: SELECT with `rows.next().await?` iteration
//! - **ID generation**: `lower(hex(randomblob(4)))` prefixed-ID pattern
//! - **Transactions**: batch operations with commit/rollback
//! - **FTS5**: full-text search with porter tokenizer (critical for Phase 1)
//! - **execute_batch**: multi-statement execution for schema migrations
//!
//! ## Validates
//!
//! libSQL crate works locally — blocks Phase 1.
//!
//! ## Background
//!
//! Originally planned with the `turso` crate (Limbo-based, Rust-native SQLite).
//! Switched to `libsql` (C SQLite fork) because:
//! - turso 0.5.0-pre.8 is pre-release with API gaps (`&String` not `IntoValue`, no `new_in_memory`)
//! - FTS is behind an experimental flag (`index_method`) not exposed by `turso::Builder`
//! - turso FTS uses different SQL syntax (`CREATE INDEX ... USING fts()`, not FTS5 virtual tables)
//! - libsql is the stable, battle-tested crate with native FTS5 support
//!
//! Plan: switch back to `turso` crate once it stabilizes and exposes FTS.

use libsql::Builder;
use tempfile::TempDir;

/// Helper: create an in-memory database for tests.
async fn in_memory_db() -> libsql::Database {
    Builder::new_local(":memory:")
        .build()
        .await
        .expect("failed to create in-memory database")
}

/// Helper: create a file-backed database in a temp directory.
async fn file_db(dir: &TempDir) -> libsql::Database {
    let path = dir.path().join("test.db");
    Builder::new_local(path)
        .build()
        .await
        .expect("failed to create file-backed database")
}

// ---------------------------------------------------------------------------
// Spike tests
// ---------------------------------------------------------------------------

/// Verify that we can create an in-memory database and get a connection.
#[tokio::test]
async fn spike_in_memory_db_connects() {
    let db = in_memory_db().await;
    let conn = db.connect().expect("failed to connect");

    // Smoke test: execute a trivial query
    let mut rows = conn.query("SELECT 1 + 1 AS result", ()).await.unwrap();
    let row = rows.next().await.unwrap().expect("expected a row");
    let val = row.get::<i64>(0).unwrap();
    assert_eq!(val, 2);
}

/// Verify that a file-backed database persists to disk.
#[tokio::test]
async fn spike_file_db_persists() {
    let dir = TempDir::new().unwrap();

    // Write data
    {
        let db = file_db(&dir).await;
        let conn = db.connect().unwrap();
        conn.execute(
            "CREATE TABLE kv (key TEXT PRIMARY KEY, value TEXT)",
            (),
        )
        .await
        .unwrap();
        conn.execute(
            "INSERT INTO kv (key, value) VALUES (?, ?)",
            libsql::params!["greeting", "hello"],
        )
        .await
        .unwrap();
    }

    // Reopen and read
    {
        let db = file_db(&dir).await;
        let conn = db.connect().unwrap();
        let mut rows = conn
            .query("SELECT value FROM kv WHERE key = ?", ["greeting"])
            .await
            .unwrap();
        let row = rows.next().await.unwrap().expect("expected a row");
        let value = row.get::<String>(0).unwrap();
        assert_eq!(value, "hello");
    }
}

/// Verify CREATE TABLE + INSERT + SELECT roundtrip with multiple columns
/// and typed parameters (the core CRUD pattern zen-db needs).
#[tokio::test]
async fn spike_crud_roundtrip() {
    let db = in_memory_db().await;
    let conn = db.connect().unwrap();

    conn.execute(
        "CREATE TABLE findings (
            id TEXT PRIMARY KEY,
            content TEXT NOT NULL,
            confidence TEXT NOT NULL DEFAULT 'medium',
            created_at TEXT NOT NULL DEFAULT (datetime('now'))
        )",
        (),
    )
    .await
    .unwrap();

    // Insert
    conn.execute(
        "INSERT INTO findings (id, content, confidence) VALUES (?, ?, ?)",
        libsql::params!["fnd-test001", "libsql works locally", "high"],
    )
    .await
    .unwrap();

    conn.execute(
        "INSERT INTO findings (id, content, confidence) VALUES (?, ?, ?)",
        libsql::params!["fnd-test002", "FTS5 needs validation", "medium"],
    )
    .await
    .unwrap();

    // Query all
    let mut rows = conn
        .query("SELECT id, content, confidence FROM findings ORDER BY id", ())
        .await
        .unwrap();

    let row1 = rows.next().await.unwrap().expect("expected row 1");
    assert_eq!(row1.get::<String>(0).unwrap(), "fnd-test001");
    assert_eq!(row1.get::<String>(1).unwrap(), "libsql works locally");
    assert_eq!(row1.get::<String>(2).unwrap(), "high");

    let row2 = rows.next().await.unwrap().expect("expected row 2");
    assert_eq!(row2.get::<String>(0).unwrap(), "fnd-test002");
    assert_eq!(row2.get::<String>(1).unwrap(), "FTS5 needs validation");
    assert_eq!(row2.get::<String>(2).unwrap(), "medium");

    // No more rows
    assert!(rows.next().await.unwrap().is_none());
}

/// Verify the prefixed ID generation pattern using `lower(hex(randomblob(4)))`.
/// This is the exact SQL expression zenith uses for all entity IDs.
#[tokio::test]
async fn spike_id_generation() {
    let db = in_memory_db().await;
    let conn = db.connect().unwrap();

    // Generate IDs with different prefixes
    let prefixes = ["fnd", "ses", "hyp", "tsk", "iss", "aud"];
    for prefix in prefixes {
        let sql = format!("SELECT '{prefix}-' || lower(hex(randomblob(4)))");
        let mut rows = conn.query(&sql, ()).await.unwrap();
        let row = rows.next().await.unwrap().expect("expected an ID row");
        let id = row.get::<String>(0).unwrap();

        // Verify format: prefix + "-" + 8 hex chars
        assert!(
            id.starts_with(&format!("{prefix}-")),
            "ID '{id}' should start with '{prefix}-'"
        );
        let hex_part = &id[prefix.len() + 1..];
        assert_eq!(hex_part.len(), 8, "hex part of '{id}' should be 8 chars");
        assert!(
            hex_part.chars().all(|c| c.is_ascii_hexdigit()),
            "hex part '{hex_part}' should be all hex digits"
        );
        assert_eq!(hex_part, &hex_part.to_lowercase(), "hex part should be lowercase");
    }

    // Verify uniqueness: generate 100 IDs and check for collisions
    let mut ids = std::collections::HashSet::new();
    for _ in 0..100 {
        let mut rows = conn
            .query("SELECT 'fnd-' || lower(hex(randomblob(4)))", ())
            .await
            .unwrap();
        let row = rows.next().await.unwrap().unwrap();
        let id = row.get::<String>(0).unwrap();
        assert!(ids.insert(id.clone()), "duplicate ID generated: {id}");
    }
}

/// Verify that IDs generated inside INSERT via DEFAULT work correctly.
/// This is how zen-db will generate IDs at insert time.
#[tokio::test]
async fn spike_id_generation_in_insert() {
    let db = in_memory_db().await;
    let conn = db.connect().unwrap();

    conn.execute(
        "CREATE TABLE items (
            id TEXT PRIMARY KEY DEFAULT ('itm-' || lower(hex(randomblob(4)))),
            name TEXT NOT NULL
        )",
        (),
    )
    .await
    .unwrap();

    // Insert without specifying ID — let the DEFAULT generate it
    conn.execute("INSERT INTO items (name) VALUES (?)", ["test item"])
        .await
        .unwrap();

    // Retrieve and verify the generated ID
    let mut rows = conn.query("SELECT id, name FROM items", ()).await.unwrap();
    let row = rows.next().await.unwrap().expect("expected a row");
    let id = row.get::<String>(0).unwrap();
    let name = row.get::<String>(1).unwrap();

    assert!(id.starts_with("itm-"), "generated ID should start with 'itm-': {id}");
    assert_eq!(id.len(), 12, "ID should be 12 chars (3 prefix + 1 dash + 8 hex): {id}");
    assert_eq!(name, "test item");
}

/// Verify transactions: commit persists, rollback discards.
#[tokio::test]
async fn spike_transactions() {
    let db = in_memory_db().await;
    let conn = db.connect().unwrap();

    conn.execute("CREATE TABLE counters (name TEXT PRIMARY KEY, val INTEGER)", ())
        .await
        .unwrap();

    // Transaction that commits
    {
        let tx = conn.transaction().await.unwrap();
        tx.execute(
            "INSERT INTO counters (name, val) VALUES (?, ?)",
            libsql::params!["committed", 42],
        )
        .await
        .unwrap();
        tx.commit().await.unwrap();
    }

    // Transaction that rolls back explicitly
    {
        let tx = conn.transaction().await.unwrap();
        tx.execute(
            "INSERT INTO counters (name, val) VALUES (?, ?)",
            libsql::params!["rolled_back", 99],
        )
        .await
        .unwrap();
        tx.rollback().await.unwrap();
    }

    // Verify: only committed row exists
    let mut rows = conn
        .query("SELECT name, val FROM counters ORDER BY name", ())
        .await
        .unwrap();

    let row = rows.next().await.unwrap().expect("expected committed row");
    assert_eq!(row.get::<String>(0).unwrap(), "committed");
    assert_eq!(row.get::<i64>(1).unwrap(), 42);

    // No more rows (rolled_back should not be there)
    assert!(
        rows.next().await.unwrap().is_none(),
        "rolled-back row should not exist"
    );
}

/// Verify `execute_batch` works for multi-statement schema migrations.
/// This is how zen-db will apply `migrations/001_initial.sql`.
#[tokio::test]
async fn spike_execute_batch() {
    let db = in_memory_db().await;
    let conn = db.connect().unwrap();

    // Simulate a migration batch with multiple statements
    conn.execute_batch(
        "
        CREATE TABLE sessions (
            id TEXT PRIMARY KEY,
            status TEXT NOT NULL DEFAULT 'active',
            started_at TEXT NOT NULL DEFAULT (datetime('now')),
            ended_at TEXT
        );

        CREATE TABLE audit_trail (
            id TEXT PRIMARY KEY DEFAULT ('aud-' || lower(hex(randomblob(4)))),
            session_id TEXT REFERENCES sessions(id),
            entity_type TEXT NOT NULL,
            entity_id TEXT NOT NULL,
            action TEXT NOT NULL,
            detail TEXT,
            created_at TEXT NOT NULL DEFAULT (datetime('now'))
        );

        CREATE INDEX idx_audit_entity ON audit_trail(entity_type, entity_id);
        CREATE INDEX idx_audit_session ON audit_trail(session_id);
        ",
    )
    .await
    .unwrap();

    // Verify both tables were created by inserting into them
    conn.execute(
        "INSERT INTO sessions (id, status) VALUES (?, ?)",
        libsql::params!["ses-00000001", "active"],
    )
    .await
    .unwrap();

    conn.execute(
        "INSERT INTO audit_trail (session_id, entity_type, entity_id, action)
         VALUES (?, ?, ?, ?)",
        libsql::params!["ses-00000001", "session", "ses-00000001", "created"],
    )
    .await
    .unwrap();

    // Verify the auto-generated audit ID
    let mut rows = conn
        .query("SELECT id, action FROM audit_trail", ())
        .await
        .unwrap();
    let row = rows.next().await.unwrap().expect("expected audit row");
    let audit_id = row.get::<String>(0).unwrap();
    assert!(
        audit_id.starts_with("aud-"),
        "audit ID should be auto-generated: {audit_id}"
    );
    assert_eq!(row.get::<String>(1).unwrap(), "created");
}

/// Verify FTS5 with porter tokenizer works for full-text search.
/// This is critical for Phase 1 — zen-db uses FTS5 across 7 virtual tables.
/// libsql (C SQLite fork) has native FTS5 support — no experimental flags needed.
#[tokio::test]
async fn spike_fts5_search() {
    let db = in_memory_db().await;
    let conn = db.connect().unwrap();

    // Create content table and FTS5 virtual table (mirrors zenith schema pattern)
    conn.execute_batch(
        "
        CREATE TABLE findings (
            id TEXT PRIMARY KEY,
            content TEXT NOT NULL,
            source TEXT,
            created_at TEXT NOT NULL DEFAULT (datetime('now'))
        );

        CREATE VIRTUAL TABLE findings_fts USING fts5(
            content,
            source,
            content='findings',
            content_rowid='rowid',
            tokenize='porter unicode61'
        );

        -- Triggers to keep FTS in sync (from 01-turso-data-model.md pattern)
        CREATE TRIGGER findings_ai AFTER INSERT ON findings BEGIN
            INSERT INTO findings_fts(rowid, content, source)
            VALUES (new.rowid, new.content, new.source);
        END;

        CREATE TRIGGER findings_ad AFTER DELETE ON findings BEGIN
            INSERT INTO findings_fts(findings_fts, rowid, content, source)
            VALUES ('delete', old.rowid, old.content, old.source);
        END;

        CREATE TRIGGER findings_au AFTER UPDATE ON findings BEGIN
            INSERT INTO findings_fts(findings_fts, rowid, content, source)
            VALUES ('delete', old.rowid, old.content, old.source);
            INSERT INTO findings_fts(rowid, content, source)
            VALUES (new.rowid, new.content, new.source);
        END;
        ",
    )
    .await
    .unwrap();

    // Insert test data
    let test_data: &[(&str, &str, &str)] = &[
        ("fnd-001", "Tokio spawning tasks uses spawn() and spawn_blocking()", "docs"),
        ("fnd-002", "The async runtime manages task scheduling internally", "research"),
        ("fnd-003", "Connection pooling reduces database overhead", "benchmark"),
        ("fnd-004", "Spawned tasks run concurrently on the tokio executor", "code review"),
        ("fnd-005", "HTTP server listens on port 8080 by default", "config"),
    ];

    for (id, content, source) in test_data {
        conn.execute(
            "INSERT INTO findings (id, content, source) VALUES (?, ?, ?)",
            libsql::params![*id, *content, *source],
        )
        .await
        .unwrap();
    }

    // Porter stemming test: "spawning" should match "spawn" and "spawned"
    let mut rows = conn
        .query(
            "SELECT f.id, f.content
             FROM findings_fts fts
             JOIN findings f ON f.rowid = fts.rowid
             WHERE findings_fts MATCH ?
             ORDER BY rank",
            ["spawning"],
        )
        .await
        .unwrap();

    let mut matched_ids = Vec::new();
    while let Some(row) = rows.next().await.unwrap() {
        matched_ids.push(row.get::<String>(0).unwrap());
    }

    assert!(
        matched_ids.contains(&"fnd-001".to_string()),
        "FTS should match 'spawn()' via porter stemming: {matched_ids:?}"
    );
    assert!(
        matched_ids.contains(&"fnd-004".to_string()),
        "FTS should match 'Spawned' via porter stemming: {matched_ids:?}"
    );
    assert!(
        !matched_ids.contains(&"fnd-003".to_string()),
        "FTS should NOT match unrelated content: {matched_ids:?}"
    );

    // Phrase search: "connection pooling"
    let mut rows = conn
        .query(
            "SELECT f.id FROM findings_fts fts
             JOIN findings f ON f.rowid = fts.rowid
             WHERE findings_fts MATCH ?",
            ["\"connection pooling\""],
        )
        .await
        .unwrap();
    let row = rows.next().await.unwrap().expect("expected phrase match");
    assert_eq!(row.get::<String>(0).unwrap(), "fnd-003");

    // Column-filtered search: search only in source column
    let mut rows = conn
        .query(
            "SELECT f.id FROM findings_fts fts
             JOIN findings f ON f.rowid = fts.rowid
             WHERE findings_fts MATCH ?",
            ["source:research"],
        )
        .await
        .unwrap();
    let row = rows.next().await.unwrap().expect("expected source match");
    assert_eq!(row.get::<String>(0).unwrap(), "fnd-002");
}

/// Verify datetime handling — zenith stores timestamps as ISO 8601 TEXT.
#[tokio::test]
async fn spike_datetime_handling() {
    let db = in_memory_db().await;
    let conn = db.connect().unwrap();

    conn.execute(
        "CREATE TABLE events (
            id INTEGER PRIMARY KEY,
            name TEXT,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            custom_at TEXT
        )",
        (),
    )
    .await
    .unwrap();

    // Insert with default timestamp
    conn.execute("INSERT INTO events (name) VALUES (?)", ["auto"])
        .await
        .unwrap();

    // Insert with explicit ISO 8601 timestamp (how chrono will format it)
    conn.execute(
        "INSERT INTO events (name, custom_at) VALUES (?, ?)",
        libsql::params!["manual", "2026-02-08T12:00:00+00:00"],
    )
    .await
    .unwrap();

    // Verify default timestamp is populated and looks like a datetime
    let mut rows = conn
        .query("SELECT created_at FROM events WHERE name = ?", ["auto"])
        .await
        .unwrap();
    let row = rows.next().await.unwrap().unwrap();
    let ts = row.get::<String>(0).unwrap();
    assert!(
        ts.contains('-') && ts.contains(':'),
        "default timestamp should look like a datetime: {ts}"
    );

    // Verify explicit timestamp roundtrips
    let mut rows = conn
        .query("SELECT custom_at FROM events WHERE name = ?", ["manual"])
        .await
        .unwrap();
    let row = rows.next().await.unwrap().unwrap();
    let ts = row.get::<String>(0).unwrap();
    assert_eq!(ts, "2026-02-08T12:00:00+00:00");

    // Verify datetime comparison works (for queries like "audit entries after X")
    let mut rows = conn
        .query(
            "SELECT count(*) FROM events WHERE created_at >= datetime('now', '-1 hour')",
            (),
        )
        .await
        .unwrap();
    let row = rows.next().await.unwrap().unwrap();
    let count = row.get::<i64>(0).unwrap();
    assert!(count >= 1, "should find at least one recent event");
}

/// Verify NULL handling — many zenith entity fields are optional.
#[tokio::test]
async fn spike_null_handling() {
    let db = in_memory_db().await;
    let conn = db.connect().unwrap();

    conn.execute(
        "CREATE TABLE tasks (
            id TEXT PRIMARY KEY,
            title TEXT NOT NULL,
            issue_id TEXT,
            session_id TEXT
        )",
        (),
    )
    .await
    .unwrap();

    // Insert with NULLs
    conn.execute(
        "INSERT INTO tasks (id, title, issue_id, session_id) VALUES (?, ?, NULL, NULL)",
        libsql::params!["tsk-001", "standalone task"],
    )
    .await
    .unwrap();

    // Insert with values
    conn.execute(
        "INSERT INTO tasks (id, title, issue_id, session_id) VALUES (?, ?, ?, ?)",
        libsql::params!["tsk-002", "linked task", "iss-001", "ses-001"],
    )
    .await
    .unwrap();

    // Query and check NULL vs non-NULL
    let mut rows = conn
        .query("SELECT id, issue_id FROM tasks ORDER BY id", ())
        .await
        .unwrap();

    let row1 = rows.next().await.unwrap().unwrap();
    assert_eq!(row1.get::<String>(0).unwrap(), "tsk-001");
    // libsql: get on a NULL column returns an error, use get_value for nullable
    let issue_val = row1.get_value(1).unwrap();
    assert!(
        matches!(issue_val, libsql::Value::Null),
        "issue_id should be NULL for tsk-001, got: {issue_val:?}"
    );

    let row2 = rows.next().await.unwrap().unwrap();
    assert_eq!(row2.get::<String>(1).unwrap(), "iss-001");
}

/// Verify foreign key enforcement — zenith relies on FK constraints.
#[tokio::test]
async fn spike_foreign_keys() {
    let db = in_memory_db().await;
    let conn = db.connect().unwrap();

    // Enable foreign keys (SQLite has them off by default)
    conn.execute("PRAGMA foreign_keys = ON", ()).await.unwrap();

    conn.execute_batch(
        "
        CREATE TABLE sessions (
            id TEXT PRIMARY KEY,
            status TEXT NOT NULL DEFAULT 'active'
        );

        CREATE TABLE findings (
            id TEXT PRIMARY KEY,
            session_id TEXT REFERENCES sessions(id),
            content TEXT NOT NULL
        );
        ",
    )
    .await
    .unwrap();

    // Insert a session
    conn.execute("INSERT INTO sessions (id) VALUES (?)", ["ses-001"])
        .await
        .unwrap();

    // Insert a finding linked to existing session — should succeed
    conn.execute(
        "INSERT INTO findings (id, session_id, content) VALUES (?, ?, ?)",
        libsql::params!["fnd-001", "ses-001", "valid reference"],
    )
    .await
    .unwrap();

    // Insert a finding linked to non-existent session — should fail
    let result = conn
        .execute(
            "INSERT INTO findings (id, session_id, content) VALUES (?, ?, ?)",
            libsql::params!["fnd-002", "ses-nonexistent", "invalid reference"],
        )
        .await;

    assert!(result.is_err(), "FK violation should return an error");
}

/// Comprehensive spike summary: build the exact pattern zen-db will use
/// for its migration + repo pattern, end-to-end.
#[tokio::test]
async fn spike_end_to_end_zen_pattern() {
    let dir = TempDir::new().unwrap();
    let db_path = dir.path().join("zenith.db");

    // 1. Open database (simulating ZenDb::open_local)
    let db = Builder::new_local(db_path)
        .build()
        .await
        .unwrap();
    let conn = db.connect().unwrap();

    // 2. Run migrations (simulating ZenDb::run_migrations via execute_batch)
    conn.execute("PRAGMA foreign_keys = ON", ()).await.unwrap();
    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS sessions (
            id TEXT PRIMARY KEY DEFAULT ('ses-' || lower(hex(randomblob(4)))),
            status TEXT NOT NULL DEFAULT 'active',
            goal TEXT,
            started_at TEXT NOT NULL DEFAULT (datetime('now')),
            ended_at TEXT
        );

        CREATE TABLE IF NOT EXISTS findings (
            id TEXT PRIMARY KEY DEFAULT ('fnd-' || lower(hex(randomblob(4)))),
            session_id TEXT REFERENCES sessions(id),
            content TEXT NOT NULL,
            confidence TEXT NOT NULL DEFAULT 'medium',
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            updated_at TEXT NOT NULL DEFAULT (datetime('now'))
        );

        CREATE TABLE IF NOT EXISTS finding_tags (
            finding_id TEXT NOT NULL REFERENCES findings(id),
            tag TEXT NOT NULL,
            PRIMARY KEY (finding_id, tag)
        );

        CREATE TABLE IF NOT EXISTS audit_trail (
            id TEXT PRIMARY KEY DEFAULT ('aud-' || lower(hex(randomblob(4)))),
            session_id TEXT REFERENCES sessions(id),
            entity_type TEXT NOT NULL,
            entity_id TEXT NOT NULL,
            action TEXT NOT NULL,
            detail TEXT,
            created_at TEXT NOT NULL DEFAULT (datetime('now'))
        );

        CREATE VIRTUAL TABLE IF NOT EXISTS findings_fts USING fts5(
            content,
            content='findings',
            content_rowid='rowid',
            tokenize='porter unicode61'
        );

        CREATE TRIGGER IF NOT EXISTS findings_ai AFTER INSERT ON findings BEGIN
            INSERT INTO findings_fts(rowid, content) VALUES (new.rowid, new.content);
        END;
        ",
    )
    .await
    .unwrap();

    // 3. Start a session (simulating SessionRepo::start)
    conn.execute("INSERT INTO sessions (goal) VALUES (?)", ["Spike 0.2"])
        .await
        .unwrap();

    let mut rows = conn
        .query("SELECT id FROM sessions WHERE status = 'active' LIMIT 1", ())
        .await
        .unwrap();
    let session_id: String = rows.next().await.unwrap().unwrap().get(0).unwrap();
    assert!(session_id.starts_with("ses-"));

    // 4. Create findings in a transaction (simulating FindingRepo::create + AuditRepo::append)
    let tx = conn.transaction().await.unwrap();

    tx.execute(
        "INSERT INTO findings (session_id, content, confidence) VALUES (?, ?, ?)",
        libsql::params![session_id.as_str(), "libsql crate compiles and works locally", "high"],
    )
    .await
    .unwrap();

    // Get the generated finding ID
    let mut rows = tx
        .query("SELECT id FROM findings ORDER BY rowid DESC LIMIT 1", ())
        .await
        .unwrap();
    let finding_id: String = rows.next().await.unwrap().unwrap().get(0).unwrap();
    assert!(finding_id.starts_with("fnd-"));

    // Write audit entry
    let detail_json = r#"{"confidence":"high"}"#;
    tx.execute(
        "INSERT INTO audit_trail (session_id, entity_type, entity_id, action, detail)
         VALUES (?, ?, ?, ?, ?)",
        libsql::params![session_id.as_str(), "finding", finding_id.as_str(), "created", detail_json],
    )
    .await
    .unwrap();

    // Tag the finding
    tx.execute(
        "INSERT INTO finding_tags (finding_id, tag) VALUES (?, ?)",
        libsql::params![finding_id.as_str(), "verified"],
    )
    .await
    .unwrap();

    tx.commit().await.unwrap();

    // 5. Search via FTS5 (simulating FindingRepo::search)
    let mut rows = conn
        .query(
            "SELECT f.id, f.content
             FROM findings_fts fts
             JOIN findings f ON f.rowid = fts.rowid
             WHERE findings_fts MATCH ?",
            ["libsql"],
        )
        .await
        .unwrap();
    let row = rows.next().await.unwrap().expect("FTS should find our finding");
    assert_eq!(row.get::<String>(0).unwrap(), finding_id);

    // 6. Query audit trail (simulating AuditRepo::query)
    let mut rows = conn
        .query(
            "SELECT entity_type, entity_id, action FROM audit_trail WHERE session_id = ?",
            [session_id.as_str()],
        )
        .await
        .unwrap();
    let row = rows.next().await.unwrap().expect("should have audit entry");
    assert_eq!(row.get::<String>(0).unwrap(), "finding");
    assert_eq!(row.get::<String>(1).unwrap(), finding_id);
    assert_eq!(row.get::<String>(2).unwrap(), "created");

    // 7. Query tags (simulating FindingRepo::tags)
    let mut rows = conn
        .query(
            "SELECT tag FROM finding_tags WHERE finding_id = ?",
            [finding_id.as_str()],
        )
        .await
        .unwrap();
    let row = rows.next().await.unwrap().expect("should have tag");
    assert_eq!(row.get::<String>(0).unwrap(), "verified");
}

// ---------------------------------------------------------------------------
// Spike 0.2b: NULL binding for FK columns + dynamic params validation
// ---------------------------------------------------------------------------
// These tests validate two BLOCKING open questions from the Phase 2 review:
//   Q1: unwrap_or("") breaks FK constraints when PRAGMA foreign_keys = ON
//   Q2: params_from_iter(Vec<libsql::Value>) works for dynamic UPDATE SET clauses

/// Q1: Prove that empty string ("") violates FK constraints but Value::Null does not.
///
/// The Phase 2 plan uses `unwrap_or("")` for nullable FK columns like `research_id`.
/// Spike 0.12's replayer inherited this pattern. This test proves it fails when
/// PRAGMA foreign_keys = ON, confirming we MUST use Value::Null instead.
#[tokio::test]
async fn spike_empty_string_violates_fk_constraint() {
    let db = in_memory_db().await;
    let conn = db.connect().unwrap();
    conn.execute("PRAGMA foreign_keys = ON", ()).await.unwrap();

    conn.execute_batch(
        "
        CREATE TABLE sessions (
            id TEXT PRIMARY KEY,
            status TEXT NOT NULL DEFAULT 'active'
        );
        CREATE TABLE research_items (
            id TEXT PRIMARY KEY,
            title TEXT NOT NULL
        );
        CREATE TABLE findings (
            id TEXT PRIMARY KEY,
            research_id TEXT REFERENCES research_items(id),
            session_id TEXT NOT NULL REFERENCES sessions(id),
            content TEXT NOT NULL,
            confidence TEXT NOT NULL DEFAULT 'medium'
        );
        INSERT INTO sessions (id) VALUES ('ses-001');
        INSERT INTO research_items (id, title) VALUES ('res-001', 'Test Research');
        ",
    )
    .await
    .unwrap();

    // Case 1: Valid FK reference — should succeed
    let result = conn
        .execute(
            "INSERT INTO findings (id, research_id, session_id, content) VALUES (?, ?, ?, ?)",
            libsql::params!["fnd-001", "res-001", "ses-001", "linked finding"],
        )
        .await;
    assert!(result.is_ok(), "Valid FK reference should succeed");

    // Case 2: Empty string as FK — should FAIL (no research_item with id="")
    let result = conn
        .execute(
            "INSERT INTO findings (id, research_id, session_id, content) VALUES (?, ?, ?, ?)",
            libsql::params!["fnd-002", "", "ses-001", "empty string FK"],
        )
        .await;
    assert!(
        result.is_err(),
        "Empty string FK should fail: no parent row with id=''"
    );
    let err_msg = format!("{}", result.unwrap_err());
    assert!(
        err_msg.contains("FOREIGN KEY"),
        "Error should be FK violation, got: {err_msg}"
    );

    // Case 3: NULL as FK — should succeed (NULL bypasses FK checks)
    let result = conn
        .execute(
            "INSERT INTO findings (id, research_id, session_id, content) VALUES (?1, ?2, ?3, ?4)",
            vec![
                libsql::Value::Text("fnd-003".to_string()),
                libsql::Value::Null,
                libsql::Value::Text("ses-001".to_string()),
                libsql::Value::Text("null FK finding".to_string()),
            ],
        )
        .await;
    assert!(
        result.is_ok(),
        "NULL FK should succeed (NULL bypasses FK constraint): {:?}",
        result.err()
    );

    // Verify the NULL was stored correctly
    let mut rows = conn
        .query(
            "SELECT research_id FROM findings WHERE id = 'fnd-003'",
            (),
        )
        .await
        .unwrap();
    let row = rows.next().await.unwrap().unwrap();
    let val = row.get_value(0).unwrap();
    assert!(
        matches!(val, libsql::Value::Null),
        "research_id should be NULL, got: {val:?}"
    );
}

/// Q1 extended: Prove the unwrap_or("") pattern in spike 0.12 replayer would
/// break on entities with absent FK fields during rebuild.
///
/// Simulates what happens when a trail operation has no research_id field
/// and the replayer does `op.data["research_id"].as_str().unwrap_or("")`.
#[tokio::test]
async fn spike_replay_unwrap_or_empty_breaks_fk() {
    let db = in_memory_db().await;
    let conn = db.connect().unwrap();
    conn.execute("PRAGMA foreign_keys = ON", ()).await.unwrap();

    conn.execute_batch(
        "
        CREATE TABLE sessions (
            id TEXT PRIMARY KEY,
            status TEXT NOT NULL DEFAULT 'active'
        );
        CREATE TABLE research_items (
            id TEXT PRIMARY KEY,
            title TEXT NOT NULL
        );
        CREATE TABLE findings (
            id TEXT PRIMARY KEY,
            research_id TEXT REFERENCES research_items(id),
            session_id TEXT NOT NULL REFERENCES sessions(id),
            content TEXT NOT NULL,
            confidence TEXT NOT NULL DEFAULT 'medium'
        );
        INSERT INTO sessions (id) VALUES ('ses-001');
        ",
    )
    .await
    .unwrap();

    // Simulate trail JSON where research_id is absent (finding not linked to research)
    let trail_data: serde_json::Value = serde_json::json!({
        "id": "fnd-replay-001",
        "session_id": "ses-001",
        "content": "standalone finding",
        "confidence": "high"
    });

    // The spike 0.12 pattern: unwrap_or("") — THIS SHOULD FAIL
    let research_id = trail_data
        .get("research_id")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let broken_result = conn
        .execute(
            "INSERT INTO findings (id, research_id, session_id, content, confidence) VALUES (?, ?, ?, ?, ?)",
            libsql::params![
                trail_data["id"].as_str().unwrap(),
                research_id,
                trail_data["session_id"].as_str().unwrap(),
                trail_data["content"].as_str().unwrap(),
                trail_data["confidence"].as_str().unwrap()
            ],
        )
        .await;
    assert!(
        broken_result.is_err(),
        "unwrap_or(\"\") replay should fail with FK constraint violation"
    );

    // The correct pattern: use Value::Null for absent FK fields
    let research_value = match trail_data.get("research_id").and_then(|v| v.as_str()) {
        Some(s) if !s.is_empty() => libsql::Value::Text(s.to_string()),
        _ => libsql::Value::Null,
    };
    let correct_result = conn
        .execute(
            "INSERT INTO findings (id, research_id, session_id, content, confidence) VALUES (?1, ?2, ?3, ?4, ?5)",
            vec![
                libsql::Value::Text("fnd-replay-001".to_string()),
                research_value,
                libsql::Value::Text("ses-001".to_string()),
                libsql::Value::Text("standalone finding".to_string()),
                libsql::Value::Text("high".to_string()),
            ],
        )
        .await;
    assert!(
        correct_result.is_ok(),
        "Value::Null replay should succeed: {:?}",
        correct_result.err()
    );
}

/// Q2: Validate that params_from_iter(Vec<libsql::Value>) works for dynamic
/// UPDATE SET clauses.
///
/// The Phase 2 plan proposes Vec<Box<dyn IntoValue>> which won't compile.
/// This test validates the alternative: build SQL dynamically, pass
/// Vec<libsql::Value> via params_from_iter().
#[tokio::test]
async fn spike_dynamic_update_with_params_from_iter() {
    let db = in_memory_db().await;
    let conn = db.connect().unwrap();

    conn.execute_batch(
        "
        CREATE TABLE findings (
            id TEXT PRIMARY KEY,
            content TEXT NOT NULL,
            source TEXT,
            confidence TEXT NOT NULL DEFAULT 'medium',
            updated_at TEXT NOT NULL DEFAULT (datetime('now'))
        );
        INSERT INTO findings (id, content, source, confidence)
        VALUES ('fnd-001', 'original content', 'original source', 'low');
        ",
    )
    .await
    .unwrap();

    // Simulate a partial update: only content and confidence changed, source unchanged
    let update_content: Option<String> = Some("updated content".to_string());
    let update_source: Option<Option<String>> = None; // not changed
    let update_confidence: Option<String> = Some("high".to_string());

    // Build dynamic SET clause + params vector
    let mut sets = Vec::new();
    let mut vals: Vec<libsql::Value> = Vec::new();

    if let Some(content) = &update_content {
        vals.push(libsql::Value::Text(content.clone()));
        sets.push(format!("content = ?{}", vals.len()));
    }
    if let Some(source_opt) = &update_source {
        match source_opt {
            Some(s) => vals.push(libsql::Value::Text(s.clone())),
            None => vals.push(libsql::Value::Null),
        }
        sets.push(format!("source = ?{}", vals.len()));
    }
    if let Some(confidence) = &update_confidence {
        vals.push(libsql::Value::Text(confidence.clone()));
        sets.push(format!("confidence = ?{}", vals.len()));
    }

    // Always update updated_at
    vals.push(libsql::Value::Text(
        chrono::Utc::now().to_rfc3339().to_string(),
    ));
    sets.push(format!("updated_at = ?{}", vals.len()));

    // WHERE id = ?N
    vals.push(libsql::Value::Text("fnd-001".to_string()));
    let id_pos = vals.len();

    let sql = format!(
        "UPDATE findings SET {} WHERE id = ?{}",
        sets.join(", "),
        id_pos
    );

    // Execute with params_from_iter
    let result = conn
        .execute(&sql, libsql::params_from_iter(vals))
        .await;
    assert!(
        result.is_ok(),
        "Dynamic UPDATE with params_from_iter should succeed: {:?}",
        result.err()
    );
    assert_eq!(result.unwrap(), 1, "Should update exactly 1 row");

    // Verify: content and confidence changed, source unchanged
    let mut rows = conn
        .query(
            "SELECT content, source, confidence FROM findings WHERE id = 'fnd-001'",
            (),
        )
        .await
        .unwrap();
    let row = rows.next().await.unwrap().unwrap();
    assert_eq!(row.get::<String>(0).unwrap(), "updated content");
    assert_eq!(row.get::<String>(1).unwrap(), "original source");
    assert_eq!(row.get::<String>(2).unwrap(), "high");
}

/// Q2 extended: Validate dynamic UPDATE can set a nullable column to NULL
/// using Vec<libsql::Value> with params_from_iter.
///
/// This also validates the Option<Option<T>> pattern from the update builders:
/// Some(None) = "set this field to NULL".
#[tokio::test]
async fn spike_dynamic_update_set_null_with_params_from_iter() {
    let db = in_memory_db().await;
    let conn = db.connect().unwrap();

    conn.execute_batch(
        "
        CREATE TABLE findings (
            id TEXT PRIMARY KEY,
            content TEXT NOT NULL,
            source TEXT,
            confidence TEXT NOT NULL DEFAULT 'medium',
            updated_at TEXT NOT NULL DEFAULT (datetime('now'))
        );
        INSERT INTO findings (id, content, source, confidence)
        VALUES ('fnd-001', 'some content', 'has a source', 'high');
        ",
    )
    .await
    .unwrap();

    // Update: set source to NULL (Option<Option<String>> = Some(None))
    let mut sets = Vec::new();
    let mut vals: Vec<libsql::Value> = Vec::new();

    // source = Some(None) means "set to NULL"
    vals.push(libsql::Value::Null);
    sets.push(format!("source = ?{}", vals.len()));

    // updated_at
    vals.push(libsql::Value::Text(
        chrono::Utc::now().to_rfc3339().to_string(),
    ));
    sets.push(format!("updated_at = ?{}", vals.len()));

    // WHERE
    vals.push(libsql::Value::Text("fnd-001".to_string()));
    let id_pos = vals.len();

    let sql = format!(
        "UPDATE findings SET {} WHERE id = ?{}",
        sets.join(", "),
        id_pos
    );

    let result = conn
        .execute(&sql, libsql::params_from_iter(vals))
        .await;
    assert!(result.is_ok(), "Setting NULL via params_from_iter should work: {:?}", result.err());

    // Verify source is now NULL
    let mut rows = conn
        .query("SELECT source FROM findings WHERE id = 'fnd-001'", ())
        .await
        .unwrap();
    let row = rows.next().await.unwrap().unwrap();
    let val = row.get_value(0).unwrap();
    assert!(
        matches!(val, libsql::Value::Null),
        "source should be NULL after update, got: {val:?}"
    );

    // Verify other fields unchanged
    let mut rows = conn
        .query(
            "SELECT content, confidence FROM findings WHERE id = 'fnd-001'",
            (),
        )
        .await
        .unwrap();
    let row = rows.next().await.unwrap().unwrap();
    assert_eq!(row.get::<String>(0).unwrap(), "some content");
    assert_eq!(row.get::<String>(1).unwrap(), "high");
}

/// Q2 bonus: Validate Vec<libsql::Value> works directly with conn.execute()
/// (without params_from_iter), since Params implements From<Vec<Value>>.
#[tokio::test]
async fn spike_vec_value_directly_as_params() {
    let db = in_memory_db().await;
    let conn = db.connect().unwrap();

    conn.execute(
        "CREATE TABLE test_direct (id TEXT PRIMARY KEY, name TEXT, score INTEGER)",
        (),
    )
    .await
    .unwrap();

    // Pass Vec<Value> directly — Params: From<Vec<Value>>
    let params = vec![
        libsql::Value::Text("t-001".to_string()),
        libsql::Value::Text("direct test".to_string()),
        libsql::Value::Integer(42),
    ];

    let result = conn
        .execute(
            "INSERT INTO test_direct (id, name, score) VALUES (?1, ?2, ?3)",
            params,
        )
        .await;
    assert!(
        result.is_ok(),
        "Vec<Value> directly as params should work: {:?}",
        result.err()
    );

    // Verify
    let mut rows = conn
        .query("SELECT name, score FROM test_direct WHERE id = 't-001'", ())
        .await
        .unwrap();
    let row = rows.next().await.unwrap().unwrap();
    assert_eq!(row.get::<String>(0).unwrap(), "direct test");
    assert_eq!(row.get::<i64>(1).unwrap(), 42);
}

// ---------------------------------------------------------------------------
// Spike 0.2c: Transaction + trail file I/O atomicity
// ---------------------------------------------------------------------------
// Validates Q3 (Transaction boundary): wrapping SQL + audit inside a
// transaction, writing the trail file before COMMIT, and proving that
// a trail failure causes the DB to roll back (no orphaned DB state).

/// Q3: Prove that transaction rollback works when trail write fails.
///
/// Simulates the mutation protocol:
///   BEGIN → SQL insert → audit insert → trail append → COMMIT
/// If the trail append fails, the transaction should roll back,
/// leaving no orphaned rows in the database.
#[tokio::test]
async fn spike_transaction_rollback_on_trail_failure() {
    let db = in_memory_db().await;
    let conn = db.connect().unwrap();
    conn.execute("PRAGMA foreign_keys = ON", ()).await.unwrap();

    conn.execute_batch(
        "
        CREATE TABLE sessions (
            id TEXT PRIMARY KEY,
            status TEXT NOT NULL DEFAULT 'active'
        );
        CREATE TABLE findings (
            id TEXT PRIMARY KEY,
            session_id TEXT NOT NULL REFERENCES sessions(id),
            content TEXT NOT NULL,
            confidence TEXT NOT NULL DEFAULT 'medium'
        );
        CREATE TABLE audit_trail (
            id TEXT PRIMARY KEY,
            session_id TEXT,
            entity_type TEXT NOT NULL,
            entity_id TEXT NOT NULL,
            action TEXT NOT NULL
        );
        INSERT INTO sessions (id) VALUES ('ses-001');
        ",
    )
    .await
    .unwrap();

    // --- Scenario A: Happy path — trail succeeds, commit succeeds ---
    {
        let tx = conn.transaction().await.unwrap();

        tx.execute(
            "INSERT INTO findings (id, session_id, content, confidence) VALUES (?1, ?2, ?3, ?4)",
            libsql::params!["fnd-ok", "ses-001", "good finding", "high"],
        )
        .await
        .unwrap();

        tx.execute(
            "INSERT INTO audit_trail (id, session_id, entity_type, entity_id, action) VALUES (?1, ?2, ?3, ?4, ?5)",
            libsql::params!["aud-ok", "ses-001", "finding", "fnd-ok", "created"],
        )
        .await
        .unwrap();

        // Simulate successful trail write (just a no-op here)
        let trail_ok = true;
        assert!(trail_ok);

        tx.commit().await.unwrap();
    }

    // Verify finding and audit exist
    let mut rows = conn
        .query("SELECT COUNT(*) FROM findings WHERE id = 'fnd-ok'", ())
        .await
        .unwrap();
    assert_eq!(
        rows.next().await.unwrap().unwrap().get::<i64>(0).unwrap(),
        1
    );
    let mut rows = conn
        .query(
            "SELECT COUNT(*) FROM audit_trail WHERE entity_id = 'fnd-ok'",
            (),
        )
        .await
        .unwrap();
    assert_eq!(
        rows.next().await.unwrap().unwrap().get::<i64>(0).unwrap(),
        1
    );

    // --- Scenario B: Trail fails — transaction must roll back ---
    {
        let tx = conn.transaction().await.unwrap();

        tx.execute(
            "INSERT INTO findings (id, session_id, content, confidence) VALUES (?1, ?2, ?3, ?4)",
            libsql::params!["fnd-fail", "ses-001", "doomed finding", "low"],
        )
        .await
        .unwrap();

        tx.execute(
            "INSERT INTO audit_trail (id, session_id, entity_type, entity_id, action) VALUES (?1, ?2, ?3, ?4, ?5)",
            libsql::params!["aud-fail", "ses-001", "finding", "fnd-fail", "created"],
        )
        .await
        .unwrap();

        // Simulate trail write failure (e.g., disk full, bad path)
        let trail_result: Result<(), std::io::Error> = Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            "simulated trail write failure",
        ));

        if trail_result.is_err() {
            // Do NOT commit — transaction drops, triggering implicit rollback
            drop(tx);
        }
    }

    // Verify: fnd-fail should NOT exist (rolled back)
    let mut rows = conn
        .query(
            "SELECT COUNT(*) FROM findings WHERE id = 'fnd-fail'",
            (),
        )
        .await
        .unwrap();
    let count = rows.next().await.unwrap().unwrap().get::<i64>(0).unwrap();
    assert_eq!(
        count, 0,
        "Finding should NOT exist after trail failure rollback"
    );

    // Verify: aud-fail should NOT exist either
    let mut rows = conn
        .query(
            "SELECT COUNT(*) FROM audit_trail WHERE entity_id = 'fnd-fail'",
            (),
        )
        .await
        .unwrap();
    let count = rows.next().await.unwrap().unwrap().get::<i64>(0).unwrap();
    assert_eq!(
        count, 0,
        "Audit entry should NOT exist after trail failure rollback"
    );

    // Verify: the happy-path data is still intact
    let mut rows = conn
        .query("SELECT COUNT(*) FROM findings WHERE id = 'fnd-ok'", ())
        .await
        .unwrap();
    assert_eq!(
        rows.next().await.unwrap().unwrap().get::<i64>(0).unwrap(),
        1,
        "Happy-path finding should still exist"
    );
}

/// Q3 extended: Prove that Transaction drops trigger implicit rollback.
///
/// libsql::Transaction should roll back on drop if commit() was not called.
/// This is critical for the error-handling pattern where we use `?` after
/// trail write and let the transaction drop naturally on error propagation.
#[tokio::test]
async fn spike_transaction_implicit_rollback_on_drop() {
    let db = in_memory_db().await;
    let conn = db.connect().unwrap();

    conn.execute(
        "CREATE TABLE test_rollback (id TEXT PRIMARY KEY, value TEXT)",
        (),
    )
    .await
    .unwrap();

    // Insert inside a transaction, then drop without committing
    {
        let tx = conn.transaction().await.unwrap();
        tx.execute(
            "INSERT INTO test_rollback (id, value) VALUES ('r-001', 'should vanish')",
            (),
        )
        .await
        .unwrap();

        // Verify: visible inside the transaction
        let mut rows = tx
            .query(
                "SELECT COUNT(*) FROM test_rollback WHERE id = 'r-001'",
                (),
            )
            .await
            .unwrap();
        assert_eq!(
            rows.next().await.unwrap().unwrap().get::<i64>(0).unwrap(),
            1,
            "Row should be visible inside transaction"
        );

        // Drop tx without commit — implicit rollback
    }

    // Verify: row should not exist after implicit rollback
    let mut rows = conn
        .query(
            "SELECT COUNT(*) FROM test_rollback WHERE id = 'r-001'",
            (),
        )
        .await
        .unwrap();
    let count = rows.next().await.unwrap().unwrap().get::<i64>(0).unwrap();
    assert_eq!(
        count, 0,
        "Row should NOT exist after transaction drop (implicit rollback)"
    );
}

/// Q3 bonus: Prove the full mutation protocol pattern compiles and works
/// end-to-end with real file I/O for the trail.
///
/// This is the exact pattern ZenService will use:
///   let tx = conn.transaction().await?;
///   tx.execute(SQL, params).await?;
///   tx.execute(audit SQL, params).await?;
///   trail_writer.append(...)?;  // file I/O
///   tx.commit().await?;
#[tokio::test]
async fn spike_full_mutation_protocol_with_file_trail() {
    use std::io::Write;

    let db = in_memory_db().await;
    let conn = db.connect().unwrap();
    conn.execute("PRAGMA foreign_keys = ON", ()).await.unwrap();

    let trail_dir = tempfile::TempDir::new().unwrap();
    let trail_path = trail_dir.path().join("ses-001.jsonl");

    conn.execute_batch(
        "
        CREATE TABLE sessions (
            id TEXT PRIMARY KEY,
            status TEXT NOT NULL DEFAULT 'active'
        );
        CREATE TABLE findings (
            id TEXT PRIMARY KEY,
            research_id TEXT,
            session_id TEXT NOT NULL REFERENCES sessions(id),
            content TEXT NOT NULL,
            confidence TEXT NOT NULL DEFAULT 'medium'
        );
        CREATE TABLE audit_trail (
            id TEXT PRIMARY KEY,
            session_id TEXT,
            entity_type TEXT NOT NULL,
            entity_id TEXT NOT NULL,
            action TEXT NOT NULL,
            created_at TEXT NOT NULL DEFAULT (datetime('now'))
        );
        INSERT INTO sessions (id) VALUES ('ses-001');
        ",
    )
    .await
    .unwrap();

    // Full mutation protocol
    let tx = conn.transaction().await.unwrap();

    // 1. SQL insert (with Value::Null for nullable FK)
    tx.execute(
        "INSERT INTO findings (id, research_id, session_id, content, confidence) VALUES (?1, ?2, ?3, ?4, ?5)",
        vec![
            libsql::Value::Text("fnd-proto-001".to_string()),
            libsql::Value::Null,
            libsql::Value::Text("ses-001".to_string()),
            libsql::Value::Text("protocol test finding".to_string()),
            libsql::Value::Text("high".to_string()),
        ],
    )
    .await
    .unwrap();

    // 2. Audit insert (inside same transaction)
    tx.execute(
        "INSERT INTO audit_trail (id, session_id, entity_type, entity_id, action) VALUES (?1, ?2, ?3, ?4, ?5)",
        libsql::params!["aud-proto-001", "ses-001", "finding", "fnd-proto-001", "created"],
    )
    .await
    .unwrap();

    // 3. Trail append — real file I/O before commit
    let trail_entry = serde_json::json!({
        "v": 1,
        "ts": chrono::Utc::now().to_rfc3339(),
        "ses": "ses-001",
        "op": "create",
        "entity": "finding",
        "id": "fnd-proto-001",
        "data": {
            "content": "protocol test finding",
            "confidence": "high"
        }
    });
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&trail_path)
        .unwrap();
    writeln!(file, "{}", serde_json::to_string(&trail_entry).unwrap()).unwrap();
    file.flush().unwrap();

    // 4. Commit
    tx.commit().await.unwrap();

    // Verify DB state
    let mut rows = conn
        .query(
            "SELECT content, confidence FROM findings WHERE id = 'fnd-proto-001'",
            (),
        )
        .await
        .unwrap();
    let row = rows.next().await.unwrap().unwrap();
    assert_eq!(row.get::<String>(0).unwrap(), "protocol test finding");
    assert_eq!(row.get::<String>(1).unwrap(), "high");

    // Verify research_id is NULL
    let mut rows = conn
        .query(
            "SELECT research_id FROM findings WHERE id = 'fnd-proto-001'",
            (),
        )
        .await
        .unwrap();
    let row = rows.next().await.unwrap().unwrap();
    assert!(matches!(row.get_value(0).unwrap(), libsql::Value::Null));

    // Verify audit exists
    let mut rows = conn
        .query(
            "SELECT action FROM audit_trail WHERE entity_id = 'fnd-proto-001'",
            (),
        )
        .await
        .unwrap();
    let row = rows.next().await.unwrap().unwrap();
    assert_eq!(row.get::<String>(0).unwrap(), "created");

    // Verify trail file exists and contains the entry
    let trail_content = std::fs::read_to_string(&trail_path).unwrap();
    assert!(trail_content.contains("fnd-proto-001"));
    assert!(trail_content.contains("protocol test finding"));

    // Parse it back to verify it's valid JSONL
    let parsed: serde_json::Value = serde_json::from_str(trail_content.trim()).unwrap();
    assert_eq!(parsed["op"], "create");
    assert_eq!(parsed["entity"], "finding");
    assert_eq!(parsed["id"], "fnd-proto-001");
}

// ---------------------------------------------------------------------------
// Spike 0.2d: Replay ambiguity — JSON null vs absent key
// ---------------------------------------------------------------------------
// Validates Issue 6: The replayer must distinguish three states for each field:
//   - Key absent in JSON → field not changed (skip in UPDATE)
//   - Key present, value is JSON null → set column to SQL NULL
//   - Key present, value is string → set column to that value
//
// The write side uses Option<Option<T>> with skip_serializing_if to produce
// the correct JSON shape. These tests validate the READ side (replay).

/// Issue 6: Prove that serde_json distinguishes null from absent correctly,
/// and that we can map each case to the right libsql::Value for replay.
#[tokio::test]
async fn spike_replay_null_vs_absent_vs_value() {
    // Simulate three trail update payloads:

    // Case A: field present with a value
    let data_a: serde_json::Value = serde_json::json!({
        "content": "updated content",
        "source": "new source"
    });

    // Case B: field present but null (means "set to NULL")
    let data_b: serde_json::Value = serde_json::json!({
        "content": "updated content",
        "source": null
    });

    // Case C: field absent (means "not changed")
    let data_c: serde_json::Value = serde_json::json!({
        "content": "updated content"
    });

    // The correct replay helper: extract a field to libsql::Value
    fn json_field_to_update(data: &serde_json::Value, field: &str) -> Option<libsql::Value> {
        match data.get(field) {
            None => None, // absent → don't touch this column
            Some(serde_json::Value::Null) => Some(libsql::Value::Null), // explicit null → SET to NULL
            Some(serde_json::Value::String(s)) => Some(libsql::Value::Text(s.clone())),
            Some(v) => Some(libsql::Value::Text(v.to_string())), // numbers, bools as text
        }
    }

    // Case A: source = "new source"
    let source_a = json_field_to_update(&data_a, "source");
    assert!(
        matches!(&source_a, Some(libsql::Value::Text(s)) if s == "new source"),
        "Case A: source should be Text(\"new source\"), got: {source_a:?}"
    );

    // Case B: source = null → set to NULL
    let source_b = json_field_to_update(&data_b, "source");
    assert!(
        matches!(source_b, Some(libsql::Value::Null)),
        "Case B: source should be Some(Null) for explicit JSON null, got: {source_b:?}"
    );

    // Case C: source absent → don't change
    let source_c = json_field_to_update(&data_c, "source");
    assert!(
        source_c.is_none(),
        "Case C: source should be None (absent), got: {source_c:?}"
    );

    // Now prove the dynamic UPDATE builder works correctly with these cases
    let db = in_memory_db().await;
    let conn = db.connect().unwrap();

    conn.execute_batch(
        "
        CREATE TABLE findings (
            id TEXT PRIMARY KEY,
            content TEXT NOT NULL,
            source TEXT,
            confidence TEXT NOT NULL DEFAULT 'medium',
            updated_at TEXT NOT NULL DEFAULT (datetime('now'))
        );
        INSERT INTO findings (id, content, source, confidence)
        VALUES ('fnd-001', 'original', 'original source', 'low');
        ",
    )
    .await
    .unwrap();

    // Replay Case B: update content, set source to NULL, confidence absent (unchanged)
    let update_data = &data_b;
    let mut sets = Vec::new();
    let mut vals: Vec<libsql::Value> = Vec::new();

    if let Some(v) = json_field_to_update(update_data, "content") {
        vals.push(v);
        sets.push(format!("content = ?{}", vals.len()));
    }
    if let Some(v) = json_field_to_update(update_data, "source") {
        vals.push(v);
        sets.push(format!("source = ?{}", vals.len()));
    }
    if let Some(v) = json_field_to_update(update_data, "confidence") {
        vals.push(v);
        sets.push(format!("confidence = ?{}", vals.len()));
    }

    assert_eq!(sets.len(), 2, "Should only update content and source, not confidence");

    vals.push(libsql::Value::Text(
        chrono::Utc::now().to_rfc3339(),
    ));
    sets.push(format!("updated_at = ?{}", vals.len()));

    vals.push(libsql::Value::Text("fnd-001".to_string()));
    let id_pos = vals.len();

    let sql = format!(
        "UPDATE findings SET {} WHERE id = ?{}",
        sets.join(", "),
        id_pos
    );
    conn.execute(&sql, libsql::params_from_iter(vals))
        .await
        .unwrap();

    // Verify: content updated, source is NULL, confidence unchanged
    let mut rows = conn
        .query(
            "SELECT content, source, confidence FROM findings WHERE id = 'fnd-001'",
            (),
        )
        .await
        .unwrap();
    let row = rows.next().await.unwrap().unwrap();
    assert_eq!(row.get::<String>(0).unwrap(), "updated content");
    assert!(
        matches!(row.get_value(1).unwrap(), libsql::Value::Null),
        "source should be NULL after replay"
    );
    assert_eq!(
        row.get::<String>(2).unwrap(),
        "low",
        "confidence should be unchanged"
    );
}

/// Issue 6 extended: Validate the Option<Option<T>> serialization produces
/// the correct JSON for each case, which the replayer then reads correctly.
///
/// This proves the write side and read side are compatible end-to-end.
#[tokio::test]
async fn spike_option_option_serde_roundtrip_for_replay() {
    #[derive(Debug, serde::Serialize, serde::Deserialize)]
    struct FindingUpdate {
        #[serde(skip_serializing_if = "Option::is_none")]
        content: Option<String>,
        #[serde(
            skip_serializing_if = "Option::is_none",
            serialize_with = "serialize_option_option",
            deserialize_with = "deserialize_option_option",
            default
        )]
        source: Option<Option<String>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        confidence: Option<String>,
    }

    fn serialize_option_option<S: serde::Serializer>(
        val: &Option<Option<String>>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        match val {
            None => unreachable!("skip_serializing_if prevents this"),
            Some(None) => serializer.serialize_none(),
            Some(Some(s)) => serializer.serialize_str(s),
        }
    }

    fn deserialize_option_option<'de, D: serde::Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Option<Option<String>>, D::Error> {
        use serde::Deserialize;
        let val: Option<String> = Option::deserialize(deserializer)?;
        Ok(Some(val)) // Some(None) = explicit null, Some(Some(s)) = value
    }

    // Case 1: Update content only (source and confidence unchanged)
    let update1 = FindingUpdate {
        content: Some("new content".to_string()),
        source: None,      // not changed → absent from JSON
        confidence: None,  // not changed → absent from JSON
    };
    let json1 = serde_json::to_string(&update1).unwrap();
    assert_eq!(json1, r#"{"content":"new content"}"#);

    // Case 2: Set source to NULL explicitly
    let update2 = FindingUpdate {
        content: Some("new content".to_string()),
        source: Some(None),  // set to NULL → JSON null
        confidence: None,
    };
    let json2 = serde_json::to_string(&update2).unwrap();
    assert_eq!(json2, r#"{"content":"new content","source":null}"#);

    // Case 3: Set source to a new value
    let update3 = FindingUpdate {
        content: None,
        source: Some(Some("new source".to_string())),
        confidence: None,
    };
    let json3 = serde_json::to_string(&update3).unwrap();
    assert_eq!(json3, r#"{"source":"new source"}"#);

    // Now parse them back and verify the replay helper works
    fn json_field_to_update(data: &serde_json::Value, field: &str) -> Option<libsql::Value> {
        match data.get(field) {
            None => None,
            Some(serde_json::Value::Null) => Some(libsql::Value::Null),
            Some(serde_json::Value::String(s)) => Some(libsql::Value::Text(s.clone())),
            Some(v) => Some(libsql::Value::Text(v.to_string())),
        }
    }

    // Parse case 2 JSON and check replay interpretation
    let data2: serde_json::Value = serde_json::from_str(&json2).unwrap();
    assert!(
        matches!(json_field_to_update(&data2, "content"), Some(libsql::Value::Text(s)) if s == "new content")
    );
    assert!(matches!(
        json_field_to_update(&data2, "source"),
        Some(libsql::Value::Null)
    ));
    assert!(json_field_to_update(&data2, "confidence").is_none());
}

// ---------------------------------------------------------------------------
// Spike 0.2e: Concurrent writes to the SAME session file
// ---------------------------------------------------------------------------
// Validates Issue 11: spike 0.12 only tested concurrent writes to SEPARATE
// session files. This tests multiple tasks appending to the SAME file.

/// Issue 11: Prove that concurrent appends to the same JSONL file can
/// produce corrupted/interleaved lines.
///
/// serde_jsonlines::append_json_lines opens, writes, and closes the file
/// on each call. On POSIX with O_APPEND, individual writes ≤ PIPE_BUF
/// (4096 bytes on most systems) are atomic. Our JSONL lines are typically
/// 200-500 bytes, so they should be safe. This test validates that.
#[tokio::test]
async fn spike_concurrent_same_session_file_append() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("ses-shared.jsonl");

    let mut handles = Vec::new();

    // Spawn 8 concurrent tasks all writing to the SAME file
    for task_idx in 0..8u32 {
        let file_path = path.clone();
        handles.push(tokio::spawn(async move {
            for op_idx in 0..50u32 {
                let op = serde_json::json!({
                    "v": 1,
                    "ts": format!("2026-02-09T10:{:02}:{:02}Z", task_idx, op_idx),
                    "ses": "ses-shared",
                    "op": "create",
                    "entity": "finding",
                    "id": format!("fnd-t{task_idx:02}-{op_idx:03}"),
                    "data": {
                        "content": format!("Task {task_idx} finding {op_idx}"),
                        "confidence": "medium"
                    }
                });
                serde_jsonlines::append_json_lines(&file_path, [&op]).unwrap();
            }
        }));
    }

    for handle in handles {
        handle.await.unwrap();
    }

    // Verify: file should have exactly 400 lines (8 tasks * 50 ops)
    let content = std::fs::read_to_string(&path).unwrap();
    let lines: Vec<&str> = content.lines().collect();
    assert_eq!(
        lines.len(),
        400,
        "Should have 400 lines (8 tasks * 50 ops), got {}",
        lines.len()
    );

    // Verify: every line is valid JSON (no interleaving/corruption)
    let mut valid_count = 0;
    let mut corrupt_count = 0;
    for (i, line) in lines.iter().enumerate() {
        match serde_json::from_str::<serde_json::Value>(line) {
            Ok(val) => {
                assert_eq!(val["ses"], "ses-shared");
                assert!(val["id"].as_str().unwrap().starts_with("fnd-t"));
                valid_count += 1;
            }
            Err(e) => {
                eprintln!("Line {i} corrupt: {e}\n  Content: {line}");
                corrupt_count += 1;
            }
        }
    }

    assert_eq!(
        corrupt_count, 0,
        "All lines should be valid JSON, {corrupt_count} were corrupt"
    );
    assert_eq!(valid_count, 400);

    // Verify: all 400 unique IDs are present (no lost writes)
    let ids: std::collections::HashSet<String> = lines
        .iter()
        .filter_map(|line| {
            serde_json::from_str::<serde_json::Value>(line)
                .ok()
                .and_then(|v| v["id"].as_str().map(|s| s.to_string()))
        })
        .collect();
    assert_eq!(ids.len(), 400, "All 400 unique IDs should be present");
}

// ---------------------------------------------------------------------------
// Spike 0.2f: SQL injection surface — table name validation
// ---------------------------------------------------------------------------
// Validates Issue 12: count_by_status and entity_type_to_table use
// format!("... {table} ...") for table names. This proves that using
// EntityType enum → &'static str mapping is injection-safe.

/// Issue 12: Prove that entity_type_to_table returns only valid table names,
/// making format! interpolation safe when the input is EntityType.
#[tokio::test]
async fn spike_entity_type_table_mapping_is_exhaustive() {
    // Every EntityType variant must map to a known table name
    let valid_tables = [
        "sessions",
        "research_items",
        "findings",
        "hypotheses",
        "insights",
        "issues",
        "tasks",
        "implementation_log",
        "compatibility_checks",
        "studies",
        "entity_links",
        "audit_trail",
    ];

    // Simulate the entity_type_to_table function from the plan
    fn entity_type_to_table(entity: &zen_core::enums::EntityType) -> &'static str {
        match entity {
            zen_core::enums::EntityType::Session => "sessions",
            zen_core::enums::EntityType::Research => "research_items",
            zen_core::enums::EntityType::Finding => "findings",
            zen_core::enums::EntityType::Hypothesis => "hypotheses",
            zen_core::enums::EntityType::Insight => "insights",
            zen_core::enums::EntityType::Issue => "issues",
            zen_core::enums::EntityType::Task => "tasks",
            zen_core::enums::EntityType::ImplLog => "implementation_log",
            zen_core::enums::EntityType::Compat => "compatibility_checks",
            zen_core::enums::EntityType::Study => "studies",
            zen_core::enums::EntityType::EntityLink => "entity_links",
            zen_core::enums::EntityType::Audit => "audit_trail",
        }
    }

    // Verify all variants map to known tables
    use zen_core::enums::EntityType;
    let all_variants = [
        EntityType::Session,
        EntityType::Research,
        EntityType::Finding,
        EntityType::Hypothesis,
        EntityType::Insight,
        EntityType::Issue,
        EntityType::Task,
        EntityType::ImplLog,
        EntityType::Compat,
        EntityType::Study,
        EntityType::EntityLink,
        EntityType::Audit,
    ];

    for variant in &all_variants {
        let table = entity_type_to_table(variant);
        assert!(
            valid_tables.contains(&table),
            "EntityType::{variant:?} maps to unknown table: {table}"
        );
        // Verify no special characters that could be SQL injection
        assert!(
            table.chars().all(|c| c.is_ascii_alphanumeric() || c == '_'),
            "Table name contains unsafe characters: {table}"
        );
    }
}

/// Issue 12 extended: Prove that count_by_status using EntityType instead of
/// &str eliminates the injection surface entirely.
#[tokio::test]
async fn spike_count_by_status_with_enum_is_safe() {
    let db = in_memory_db().await;
    let conn = db.connect().unwrap();

    conn.execute_batch(
        "
        CREATE TABLE tasks (
            id TEXT PRIMARY KEY,
            status TEXT NOT NULL DEFAULT 'open'
        );
        INSERT INTO tasks (id, status) VALUES ('tsk-1', 'open');
        INSERT INTO tasks (id, status) VALUES ('tsk-2', 'open');
        INSERT INTO tasks (id, status) VALUES ('tsk-3', 'in_progress');
        ",
    )
    .await
    .unwrap();

    // Safe pattern: EntityType enum → &'static str → format!
    fn entity_type_to_table(entity: &zen_core::enums::EntityType) -> &'static str {
        match entity {
            zen_core::enums::EntityType::Task => "tasks",
            _ => panic!("unhandled entity type for this test"),
        }
    }

    let table = entity_type_to_table(&zen_core::enums::EntityType::Task);
    let sql = format!("SELECT COUNT(*) FROM {table} WHERE status = ?1");
    let mut rows = conn.query(&sql, ["open"]).await.unwrap();
    let count = rows.next().await.unwrap().unwrap().get::<i64>(0).unwrap();
    assert_eq!(count, 2);

    // The status value is parameterized (?1), so it's safe even with user input.
    // The table name comes from a &'static str match arm, so no injection possible.
}

// ---------------------------------------------------------------------------
// Spike 0.2g: Option<T> native support in libsql params
// ---------------------------------------------------------------------------
// libsql 0.9.29 has `impl<T: Into<Value>> From<Option<T>> for Value`
// which means Option<String>, Option<&str>, etc. work as IntoValue.
// This means we can use the params! macro directly with Option types
// instead of manually constructing Vec<Value>.

/// Prove that Option<&str> works directly in the params! macro.
/// None becomes Value::Null, Some("x") becomes Value::Text("x").
#[tokio::test]
async fn spike_option_works_natively_in_params_macro() {
    let db = in_memory_db().await;
    let conn = db.connect().unwrap();
    conn.execute("PRAGMA foreign_keys = ON", ()).await.unwrap();

    conn.execute_batch(
        "
        CREATE TABLE sessions (id TEXT PRIMARY KEY, status TEXT NOT NULL DEFAULT 'active');
        CREATE TABLE research_items (id TEXT PRIMARY KEY, title TEXT NOT NULL);
        CREATE TABLE findings (
            id TEXT PRIMARY KEY,
            research_id TEXT REFERENCES research_items(id),
            session_id TEXT NOT NULL REFERENCES sessions(id),
            content TEXT NOT NULL,
            source TEXT,
            confidence TEXT NOT NULL DEFAULT 'medium'
        );
        INSERT INTO sessions (id) VALUES ('ses-001');
        INSERT INTO research_items (id, title) VALUES ('res-001', 'Test Research');
        ",
    )
    .await
    .unwrap();

    // Case 1: Some(&str) for FK → should resolve to Value::Text
    let research_id: Option<&str> = Some("res-001");
    let source: Option<&str> = Some("spike test");
    conn.execute(
        "INSERT INTO findings (id, research_id, session_id, content, source, confidence)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        libsql::params!["fnd-001", research_id, "ses-001", "linked finding", source, "high"],
    )
    .await
    .unwrap();

    // Case 2: None for FK → should resolve to Value::Null (bypasses FK check)
    let research_id: Option<&str> = None;
    let source: Option<&str> = None;
    conn.execute(
        "INSERT INTO findings (id, research_id, session_id, content, source, confidence)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        libsql::params!["fnd-002", research_id, "ses-001", "standalone finding", source, "medium"],
    )
    .await
    .unwrap();

    // Verify Case 1: research_id is "res-001", source is "spike test"
    let mut rows = conn
        .query("SELECT research_id, source FROM findings WHERE id = 'fnd-001'", ())
        .await
        .unwrap();
    let row = rows.next().await.unwrap().unwrap();
    assert_eq!(row.get::<String>(0).unwrap(), "res-001");
    assert_eq!(row.get::<String>(1).unwrap(), "spike test");

    // Verify Case 2: research_id is NULL, source is NULL
    let mut rows = conn
        .query("SELECT research_id, source FROM findings WHERE id = 'fnd-002'", ())
        .await
        .unwrap();
    let row = rows.next().await.unwrap().unwrap();
    let research_val = row.get_value(0).unwrap();
    assert!(matches!(research_val, libsql::Value::Null), "research_id should be NULL");
    let source_val = row.get_value(1).unwrap();
    assert!(matches!(source_val, libsql::Value::Null), "source should be NULL");
}

/// Prove Option<String> (owned) works too — matches zen-core entity field types.
#[tokio::test]
async fn spike_option_string_owned_works_in_params_macro() {
    let db = in_memory_db().await;
    let conn = db.connect().unwrap();

    conn.execute_batch(
        "CREATE TABLE items (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            description TEXT
        )",
    )
    .await
    .unwrap();

    // Simulate entity struct fields: Option<String>
    let desc_some: Option<String> = Some("has description".to_string());
    let desc_none: Option<String> = None;

    conn.execute(
        "INSERT INTO items (id, name, description) VALUES (?1, ?2, ?3)",
        libsql::params!["itm-001", "with desc", desc_some],
    )
    .await
    .unwrap();

    conn.execute(
        "INSERT INTO items (id, name, description) VALUES (?1, ?2, ?3)",
        libsql::params!["itm-002", "no desc", desc_none],
    )
    .await
    .unwrap();

    let mut rows = conn.query("SELECT description FROM items WHERE id = 'itm-001'", ()).await.unwrap();
    assert_eq!(rows.next().await.unwrap().unwrap().get::<String>(0).unwrap(), "has description");

    let mut rows = conn.query("SELECT description FROM items WHERE id = 'itm-002'", ()).await.unwrap();
    let val = rows.next().await.unwrap().unwrap().get_value(0).unwrap();
    assert!(matches!(val, libsql::Value::Null));
}

/// Prove Option works with .as_deref() for converting Option<String> to Option<&str>.
/// This is the ergonomic pattern for repo methods that take &Finding (with Option<String> fields).
#[tokio::test]
async fn spike_option_as_deref_pattern_for_repos() {
    let db = in_memory_db().await;
    let conn = db.connect().unwrap();
    conn.execute("PRAGMA foreign_keys = ON", ()).await.unwrap();

    conn.execute_batch(
        "
        CREATE TABLE sessions (id TEXT PRIMARY KEY, status TEXT NOT NULL DEFAULT 'active');
        CREATE TABLE research_items (id TEXT PRIMARY KEY, title TEXT NOT NULL);
        CREATE TABLE findings (
            id TEXT PRIMARY KEY,
            research_id TEXT REFERENCES research_items(id),
            session_id TEXT NOT NULL REFERENCES sessions(id),
            content TEXT NOT NULL,
            source TEXT,
            confidence TEXT NOT NULL DEFAULT 'medium'
        );
        INSERT INTO sessions (id) VALUES ('ses-001');
        INSERT INTO research_items (id, title) VALUES ('res-001', 'Test Research');
        ",
    )
    .await
    .unwrap();

    // Simulate a zen-core Finding struct (Option<String> fields)
    struct MockFinding {
        id: String,
        research_id: Option<String>,
        content: String,
        source: Option<String>,
        confidence: String,
    }

    let finding_with_fk = MockFinding {
        id: "fnd-001".into(),
        research_id: Some("res-001".into()),
        content: "linked finding".into(),
        source: Some("web".into()),
        confidence: "high".into(),
    };

    let finding_without_fk = MockFinding {
        id: "fnd-002".into(),
        research_id: None,
        content: "standalone".into(),
        source: None,
        confidence: "medium".into(),
    };

    // The ergonomic pattern: .as_deref() converts Option<String> → Option<&str>
    for f in [&finding_with_fk, &finding_without_fk] {
        conn.execute(
            "INSERT INTO findings (id, research_id, session_id, content, source, confidence)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            libsql::params![
                f.id.as_str(),
                f.research_id.as_deref(),
                "ses-001",
                f.content.as_str(),
                f.source.as_deref(),
                f.confidence.as_str()
            ],
        )
        .await
        .unwrap();
    }

    // Verify linked finding
    let mut rows = conn.query("SELECT research_id, source FROM findings WHERE id = 'fnd-001'", ()).await.unwrap();
    let row = rows.next().await.unwrap().unwrap();
    assert_eq!(row.get::<String>(0).unwrap(), "res-001");
    assert_eq!(row.get::<String>(1).unwrap(), "web");

    // Verify standalone finding (NULLs)
    let mut rows = conn.query("SELECT research_id, source FROM findings WHERE id = 'fnd-002'", ()).await.unwrap();
    let row = rows.next().await.unwrap().unwrap();
    assert!(matches!(row.get_value(0).unwrap(), libsql::Value::Null));
    assert!(matches!(row.get_value(1).unwrap(), libsql::Value::Null));
}

/// Prove named_params! macro also works with Option types.
#[tokio::test]
async fn spike_named_params_with_option() {
    let db = in_memory_db().await;
    let conn = db.connect().unwrap();

    conn.execute_batch(
        "CREATE TABLE items (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            description TEXT
        )",
    )
    .await
    .unwrap();

    let desc: Option<&str> = None;
    conn.execute(
        "INSERT INTO items (id, name, description) VALUES (:id, :name, :desc)",
        libsql::named_params! {":id": "itm-001", ":name": "test", ":desc": desc},
    )
    .await
    .unwrap();

    let mut rows = conn.query("SELECT description FROM items WHERE id = 'itm-001'", ()).await.unwrap();
    let val = rows.next().await.unwrap().unwrap().get_value(0).unwrap();
    assert!(matches!(val, libsql::Value::Null));
}

/// Prove Vec<Value> is needed for dynamic UPDATE queries where the number of
/// SET clauses varies at runtime (update builders with Option fields).
///
/// The params! macro expands to a fixed-size array at compile time, so it
/// can't handle "SET content = ?1" one call and "SET content = ?1, source = ?2,
/// confidence = ?3" the next. Vec<Value> is the only way.
///
/// Also proves that Option<T>.into() works when building Vec<Value>, so we
/// don't need manual match arms — just `.push(field.into())`.
#[tokio::test]
async fn spike_vec_value_needed_for_dynamic_update_builders() {
    let db = in_memory_db().await;
    let conn = db.connect().unwrap();
    conn.execute("PRAGMA foreign_keys = ON", ()).await.unwrap();

    conn.execute_batch(
        "
        CREATE TABLE sessions (id TEXT PRIMARY KEY, status TEXT NOT NULL DEFAULT 'active');
        CREATE TABLE findings (
            id TEXT PRIMARY KEY,
            session_id TEXT NOT NULL REFERENCES sessions(id),
            content TEXT NOT NULL,
            source TEXT,
            confidence TEXT NOT NULL DEFAULT 'medium',
            updated_at TEXT NOT NULL DEFAULT (datetime('now'))
        );
        INSERT INTO sessions (id) VALUES ('ses-001');
        INSERT INTO findings (id, session_id, content, source, confidence)
            VALUES ('fnd-001', 'ses-001', 'original content', 'web', 'low');
        INSERT INTO findings (id, session_id, content, source, confidence)
            VALUES ('fnd-002', 'ses-001', 'another finding', 'manual', 'medium');
        ",
    )
    .await
    .unwrap();

    // Simulate FindingUpdate builder output — only changed fields are Some
    #[derive(Default)]
    struct FindingUpdate {
        content: Option<String>,
        source: Option<Option<String>>,   // outer=specified?, inner=value-or-NULL
        confidence: Option<String>,
    }

    // Update 1: change only content (1 SET clause, 2 params total)
    let update1 = FindingUpdate {
        content: Some("updated content".into()),
        ..Default::default()
    };

    // Update 2: change content + set source to NULL + change confidence (3 SET clauses, 4 params)
    let update2 = FindingUpdate {
        content: Some("revised finding".into()),
        source: Some(None), // explicitly set to NULL
        confidence: Some("high".into()),
    };

    // Helper: builds dynamic SQL + params from update struct
    fn build_update(id: &str, update: &FindingUpdate) -> (String, Vec<libsql::Value>) {
        let mut clauses = Vec::new();
        let mut params: Vec<libsql::Value> = Vec::new();
        let mut idx = 1u32;

        if let Some(ref content) = update.content {
            clauses.push(format!("content = ?{idx}"));
            params.push(content.as_str().into());
            idx += 1;
        }
        if let Some(ref source) = update.source {
            clauses.push(format!("source = ?{idx}"));
            // Option<Option<String>>: Some(None) → NULL, Some(Some(v)) → text
            params.push(source.as_deref().into());
            idx += 1;
        }
        if let Some(ref confidence) = update.confidence {
            clauses.push(format!("confidence = ?{idx}"));
            params.push(confidence.as_str().into());
            idx += 1;
        }

        // Always add updated_at
        clauses.push(format!("updated_at = datetime('now')"));

        // WHERE clause uses the next param index
        let sql = format!(
            "UPDATE findings SET {} WHERE id = ?{idx}",
            clauses.join(", ")
        );
        params.push(id.into());

        (sql, params)
    }

    // Execute update 1: only content changes (2 params: content + id)
    let (sql1, params1) = build_update("fnd-001", &update1);
    assert_eq!(params1.len(), 2, "update1 should have 2 params (content + id)");
    conn.execute(&sql1, params1).await.unwrap();

    // Execute update 2: 3 fields change (4 params: content + source + confidence + id)
    let (sql2, params2) = build_update("fnd-002", &update2);
    assert_eq!(params2.len(), 4, "update2 should have 4 params");
    conn.execute(&sql2, params2).await.unwrap();

    // Verify update 1: only content changed, source and confidence unchanged
    let mut rows = conn.query(
        "SELECT content, source, confidence FROM findings WHERE id = 'fnd-001'", ()
    ).await.unwrap();
    let row = rows.next().await.unwrap().unwrap();
    assert_eq!(row.get::<String>(0).unwrap(), "updated content");
    assert_eq!(row.get::<String>(1).unwrap(), "web");       // unchanged
    assert_eq!(row.get::<String>(2).unwrap(), "low");        // unchanged

    // Verify update 2: all three changed, source is now NULL
    let mut rows = conn.query(
        "SELECT content, source, confidence FROM findings WHERE id = 'fnd-002'", ()
    ).await.unwrap();
    let row = rows.next().await.unwrap().unwrap();
    assert_eq!(row.get::<String>(0).unwrap(), "revised finding");
    assert!(matches!(row.get_value(1).unwrap(), libsql::Value::Null), "source should be NULL");
    assert_eq!(row.get::<String>(2).unwrap(), "high");

    // Key insight: params! can't do this because the array size differs per call.
    // Vec<Value> + .into() on Option<&str> gives us both dynamism and NULL support.
}
