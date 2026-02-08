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
