//! # Spike 0.3: libSQL Embedded Replica + Turso Cloud Sync
//!
//! Validates that `libsql::Builder::new_sync()` works for zenith's cloud sync needs:
//!
//! - **Embedded replica creation**: `Builder::new_sync()` with local file + remote URL + auth token
//! - **Initial sync**: `db.sync().await` pulls remote state to local replica
//! - **Write forwarding**: writes through embedded replica are forwarded to Turso Cloud
//! - **Read-after-write**: data written through replica is readable locally after sync
//! - **Schema sync**: CREATE TABLE + FTS5 propagate through sync
//! - **Manual sync control**: no automatic sync interval — matches zenith's "sync on wrap-up only" design
//! - **Multiple replicas**: two replicas of the same cloud DB see each other's writes after sync
//!
//! ## Validates
//!
//! Cloud sync works — blocks Phase 8.
//!
//! ## Prerequisites
//!
//! These tests require a live Turso Cloud database. Set environment variables in `zenith/.env`:
//!
//! ```bash
//! ZENITH_TURSO__URL=libsql://zenith-dev-<org>.<region>.turso.io
//! ZENITH_TURSO__PLATFORM_API_KEY=<platform-api-key>  # from `turso auth api-tokens mint <name>`
//! ZENITH_TURSO__ORG_SLUG=<org-slug>                  # from `turso org list`
//! ```
//!
//! The database name is extracted from the URL (the part before `-{org-slug}`).
//!
//! The tests programmatically generate a fresh database auth token via the Turso Platform API
//! on each run, so the token never goes stale. The platform API key (`ZENITH_TURSO__PLATFORM_API_KEY`)
//! is long-lived and does not expire.
//!
//! Tests are skipped (not failed) when credentials are missing or the API call fails.
//!
//! ## Design Note
//!
//! Zenith syncs **only** during `zen wrap-up` — no continuous sync, no background thread.
//! This spike validates that manual `db.sync().await` is sufficient for that pattern.

use libsql::Builder;
use std::time::Duration;
use tempfile::TempDir;

// All sync tests require `flavor = "multi_thread"` because libsql's replication
// internals spawn blocking tasks that require a multi-threaded tokio runtime.

/// Load .env from the workspace root (zenith/.env).
fn load_env() {
    let workspace_env = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent() // crates/
        .and_then(|p| p.parent()) // zenith/
        .map(|p| p.join(".env"));

    if let Some(env_path) = workspace_env {
        let _ = dotenvy::from_path(&env_path);
    }
}

/// Generate a fresh database auth token via the Turso Platform API.
///
/// Requires these env vars:
/// - `ZENITH_TURSO__PLATFORM_API_KEY` — long-lived platform API key (from `turso auth api-tokens mint`)
/// - `ZENITH_TURSO__ORG_SLUG` — organization slug
/// - `ZENITH_TURSO__URL` — libsql:// URL for the database
///
/// The database name is extracted from the URL by parsing `libsql://{db_name}-{org_slug}.{rest}`.
///
/// Returns `(url, fresh_db_token)` or None if any vars are missing or API call fails.
async fn turso_credentials() -> Option<(String, String)> {
    load_env();

    let url = std::env::var("ZENITH_TURSO__URL").ok()?;
    let api_key = std::env::var("ZENITH_TURSO__PLATFORM_API_KEY").ok()?;
    let org = std::env::var("ZENITH_TURSO__ORG_SLUG").ok()?;

    if url.is_empty() || api_key.is_empty() || org.is_empty() {
        return None;
    }

    // Extract database name from URL: libsql://{db_name}-{org_slug}.{region}.turso.io
    // e.g. libsql://zenith-dev-wrath-codes.aws-us-east-1.turso.io -> zenith-dev
    let hostname = url
        .strip_prefix("libsql://")?;
    let org_suffix = format!("-{org}.");
    let db_name = hostname
        .split_once(&org_suffix)
        .map(|(name, _)| name)?;

    if db_name.is_empty() {
        eprintln!("SKIP: Could not extract database name from URL: {url}");
        return None;
    }

    // Generate a fresh database auth token via Platform API
    let client = reqwest::Client::new();
    let token_url = format!(
        "https://api.turso.tech/v1/organizations/{org}/databases/{db_name}/auth/tokens?expiration=1h&authorization=full-access"
    );

    let resp = match client
        .post(&token_url)
        .header("Authorization", format!("Bearer {api_key}"))
        .send()
        .await
    {
        Ok(r) => r,
        Err(e) => {
            eprintln!("SKIP: Turso Platform API request failed: {e}");
            return None;
        }
    };

    if !resp.status().is_success() {
        eprintln!(
            "SKIP: Turso Platform API returned {}: {}",
            resp.status(),
            resp.text().await.unwrap_or_default()
        );
        return None;
    }

    let body: serde_json::Value = resp.json().await.ok()?;
    let db_token = body["jwt"].as_str()?.to_string();

    if db_token.is_empty() {
        eprintln!("SKIP: Turso Platform API returned empty JWT");
        return None;
    }

    Some((url, db_token))
}

/// Helper: create an embedded replica with manual sync only (no auto-sync interval).
/// This matches zenith's "sync on wrap-up only" design.
async fn sync_replica(
    local_path: impl AsRef<std::path::Path>,
    url: &str,
    token: &str,
) -> libsql::Database {
    Builder::new_remote_replica(
        local_path.as_ref().to_str().unwrap(),
        url.to_string(),
        token.to_string(),
    )
    .read_your_writes(true)
    .build()
    .await
    .expect("failed to create embedded replica")
}

/// Helper: create a unique table name to avoid collisions between test runs.
/// Uses a timestamp + random suffix.
fn unique_table_name(prefix: &str) -> String {
    use std::time::SystemTime;
    let ts = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_millis();
    format!("{prefix}_{ts}")
}

// ---------------------------------------------------------------------------
// Spike tests — all require ZENITH_TURSO__PLATFORM_API_KEY + ZENITH_TURSO__ORG_SLUG + ZENITH_TURSO__URL
// ---------------------------------------------------------------------------

/// Verify that we can create an embedded replica and connect to Turso Cloud.
/// This is the most basic smoke test: Builder::new_remote_replica() + sync().
#[tokio::test(flavor = "multi_thread")]
async fn spike_sync_replica_connects() {
    let Some((url, token)) = turso_credentials().await else {
        eprintln!("SKIP: Turso credentials not available (set ZENITH_TURSO__PLATFORM_API_KEY, ZENITH_TURSO__ORG_SLUG, ZENITH_TURSO__URL)");
        return;
    };

    let dir = TempDir::new().unwrap();
    let db_path = dir.path().join("replica.db");

    let db = sync_replica(&db_path, &url, &token).await;

    // Initial sync — pulls current cloud state
    db.sync().await.expect("initial sync failed");

    let conn = db.connect().expect("failed to connect to replica");

    // Smoke test: trivial query on the local replica
    let mut rows = conn.query("SELECT 1 + 1 AS result", ()).await.unwrap();
    let row = rows.next().await.unwrap().expect("expected a row");
    let val = row.get::<i64>(0).unwrap();
    assert_eq!(val, 2);

    // Verify local file was created
    assert!(db_path.exists(), "local replica file should exist on disk");
}

/// Verify that writes through the embedded replica are forwarded to Turso Cloud.
/// Pattern: write via replica → sync → data is in cloud.
#[tokio::test(flavor = "multi_thread")]
async fn spike_sync_write_and_read() {
    let Some((url, token)) = turso_credentials().await else {
        eprintln!("SKIP: Turso credentials not available");
        return;
    };

    let dir = TempDir::new().unwrap();
    let db_path = dir.path().join("replica.db");
    let table = unique_table_name("spike_wr");

    let db = sync_replica(&db_path, &url, &token).await;
    db.sync().await.expect("initial sync failed");

    let conn = db.connect().unwrap();

    // Create a table through the replica (forwarded to cloud)
    conn.execute(
        &format!(
            "CREATE TABLE IF NOT EXISTS {table} (
                id TEXT PRIMARY KEY,
                content TEXT NOT NULL,
                created_at TEXT NOT NULL DEFAULT (datetime('now'))
            )"
        ),
        (),
    )
    .await
    .unwrap();

    // Insert data
    conn.execute(
        &format!("INSERT INTO {table} (id, content) VALUES (?, ?)"),
        libsql::params!["spike-001", "embedded replica write test"],
    )
    .await
    .unwrap();

    // Sync to ensure data reaches cloud
    db.sync().await.expect("sync after write failed");

    // Read back from local replica
    let mut rows = conn
        .query(
            &format!("SELECT id, content FROM {table} WHERE id = ?"),
            ["spike-001"],
        )
        .await
        .unwrap();

    let row = rows.next().await.unwrap().expect("expected the inserted row");
    assert_eq!(row.get::<String>(0).unwrap(), "spike-001");
    assert_eq!(
        row.get::<String>(1).unwrap(),
        "embedded replica write test"
    );

    // Clean up: drop the test table
    conn.execute(&format!("DROP TABLE IF EXISTS {table}"), ())
        .await
        .unwrap();
    db.sync().await.expect("cleanup sync failed");
}

/// Verify that two separate replicas of the same cloud DB can see each other's writes
/// after syncing. This validates the full sync round-trip:
///   replica_a writes → sync → cloud → replica_b syncs → reads data
///
/// This is the pattern zenith needs for cross-machine sync at wrap-up.
#[tokio::test(flavor = "multi_thread")]
async fn spike_sync_two_replicas() {
    let Some((url, token)) = turso_credentials().await else {
        eprintln!("SKIP: Turso credentials not available");
        return;
    };

    let dir_a = TempDir::new().unwrap();
    let dir_b = TempDir::new().unwrap();
    let table = unique_table_name("spike_2r");

    // Create replica A
    let db_a = sync_replica(dir_a.path().join("replica_a.db"), &url, &token).await;
    db_a.sync().await.expect("replica_a initial sync failed");
    let conn_a = db_a.connect().unwrap();

    // Create schema through replica A
    conn_a
        .execute(
            &format!(
                "CREATE TABLE IF NOT EXISTS {table} (
                    id TEXT PRIMARY KEY,
                    source TEXT NOT NULL,
                    value TEXT NOT NULL
                )"
            ),
            (),
        )
        .await
        .unwrap();

    // Write from replica A
    conn_a
        .execute(
            &format!("INSERT INTO {table} (id, source, value) VALUES (?, ?, ?)"),
            libsql::params!["from-a", "replica_a", "hello from A"],
        )
        .await
        .unwrap();

    // Sync replica A → cloud
    db_a.sync().await.expect("replica_a sync after write failed");

    // Create replica B (separate local file, same cloud DB)
    let db_b = sync_replica(dir_b.path().join("replica_b.db"), &url, &token).await;
    db_b.sync().await.expect("replica_b initial sync failed");
    let conn_b = db_b.connect().unwrap();

    // Replica B should see replica A's data after sync
    let mut rows = conn_b
        .query(
            &format!("SELECT id, source, value FROM {table} WHERE id = ?"),
            ["from-a"],
        )
        .await
        .unwrap();

    let row = rows
        .next()
        .await
        .unwrap()
        .expect("replica_b should see replica_a's write after sync");
    assert_eq!(row.get::<String>(0).unwrap(), "from-a");
    assert_eq!(row.get::<String>(1).unwrap(), "replica_a");
    assert_eq!(row.get::<String>(2).unwrap(), "hello from A");

    // Write from replica B
    conn_b
        .execute(
            &format!("INSERT INTO {table} (id, source, value) VALUES (?, ?, ?)"),
            libsql::params!["from-b", "replica_b", "hello from B"],
        )
        .await
        .unwrap();
    db_b.sync().await.expect("replica_b sync after write failed");

    // Sync replica A again — should now see B's write
    db_a.sync().await.expect("replica_a re-sync failed");
    let mut rows = conn_a
        .query(
            &format!("SELECT id, source, value FROM {table} WHERE id = ?"),
            ["from-b"],
        )
        .await
        .unwrap();

    let row = rows
        .next()
        .await
        .unwrap()
        .expect("replica_a should see replica_b's write after re-sync");
    assert_eq!(row.get::<String>(0).unwrap(), "from-b");
    assert_eq!(row.get::<String>(1).unwrap(), "replica_b");
    assert_eq!(row.get::<String>(2).unwrap(), "hello from B");

    // Clean up
    conn_a
        .execute(&format!("DROP TABLE IF EXISTS {table}"), ())
        .await
        .unwrap();
    db_a.sync().await.expect("cleanup sync failed");
}

/// Verify that the zenith migration pattern works through an embedded replica:
/// execute_batch for schema + FTS5 + triggers, then CRUD + FTS search.
///
/// This is the critical test — if this works, Phase 8 (ZenDb::open_synced) is unblocked.
#[tokio::test(flavor = "multi_thread")]
async fn spike_sync_schema_and_fts() {
    let Some((url, token)) = turso_credentials().await else {
        eprintln!("SKIP: Turso credentials not available");
        return;
    };

    let dir = TempDir::new().unwrap();
    let table = unique_table_name("spike_fts");
    let fts_table = format!("{table}_fts");

    let db = sync_replica(dir.path().join("replica.db"), &url, &token).await;
    db.sync().await.expect("initial sync failed");
    let conn = db.connect().unwrap();

    // Apply zenith-style migration through the replica
    conn.execute_batch(&format!(
        "
        CREATE TABLE IF NOT EXISTS {table} (
            id TEXT PRIMARY KEY,
            session_id TEXT,
            content TEXT NOT NULL,
            confidence TEXT NOT NULL DEFAULT 'medium',
            created_at TEXT NOT NULL DEFAULT (datetime('now'))
        );

        CREATE VIRTUAL TABLE IF NOT EXISTS {fts_table} USING fts5(
            content,
            content='{table}',
            content_rowid='rowid',
            tokenize='porter unicode61'
        );

        CREATE TRIGGER IF NOT EXISTS {table}_ai AFTER INSERT ON {table} BEGIN
            INSERT INTO {fts_table}(rowid, content) VALUES (new.rowid, new.content);
        END;

        CREATE TRIGGER IF NOT EXISTS {table}_ad AFTER DELETE ON {table} BEGIN
            INSERT INTO {fts_table}({fts_table}, rowid, content)
            VALUES ('delete', old.rowid, old.content);
        END;

        CREATE TRIGGER IF NOT EXISTS {table}_au AFTER UPDATE ON {table} BEGIN
            INSERT INTO {fts_table}({fts_table}, rowid, content)
            VALUES ('delete', old.rowid, old.content);
            INSERT INTO {fts_table}(rowid, content)
            VALUES (new.rowid, new.content);
        END;
        "
    ))
    .await
    .unwrap();

    // Sync schema to cloud
    db.sync().await.expect("schema sync failed");

    // Insert test findings through the replica
    let test_data: &[(&str, &str, &str)] = &[
        ("fnd-s01", "Tokio spawning tasks uses spawn() and spawn_blocking()", "high"),
        ("fnd-s02", "Connection pooling reduces database overhead significantly", "medium"),
        ("fnd-s03", "Spawned tasks run concurrently on the tokio executor", "high"),
    ];

    for (id, content, confidence) in test_data {
        conn.execute(
            &format!("INSERT INTO {table} (id, content, confidence) VALUES (?, ?, ?)"),
            libsql::params![*id, *content, *confidence],
        )
        .await
        .unwrap();
    }

    // Sync data to cloud
    db.sync().await.expect("data sync failed");

    // FTS5 search: porter stemming via the replica
    let mut rows = conn
        .query(
            &format!(
                "SELECT f.id, f.content
                 FROM {fts_table} fts
                 JOIN {table} f ON f.rowid = fts.rowid
                 WHERE {fts_table} MATCH ?
                 ORDER BY rank"
            ),
            ["spawning"],
        )
        .await
        .unwrap();

    let mut matched_ids = Vec::new();
    while let Some(row) = rows.next().await.unwrap() {
        matched_ids.push(row.get::<String>(0).unwrap());
    }

    assert!(
        matched_ids.contains(&"fnd-s01".to_string()),
        "FTS should match 'spawn()' via porter stemming: {matched_ids:?}"
    );
    assert!(
        matched_ids.contains(&"fnd-s03".to_string()),
        "FTS should match 'Spawned' via porter stemming: {matched_ids:?}"
    );
    assert!(
        !matched_ids.contains(&"fnd-s02".to_string()),
        "FTS should NOT match unrelated content: {matched_ids:?}"
    );

    // Clean up: drop tables (FTS virtual table first, then content table)
    conn.execute(&format!("DROP TABLE IF EXISTS {fts_table}"), ())
        .await
        .unwrap();
    conn.execute(&format!("DROP TRIGGER IF EXISTS {table}_ai"), ())
        .await
        .unwrap();
    conn.execute(&format!("DROP TRIGGER IF EXISTS {table}_ad"), ())
        .await
        .unwrap();
    conn.execute(&format!("DROP TRIGGER IF EXISTS {table}_au"), ())
        .await
        .unwrap();
    conn.execute(&format!("DROP TABLE IF EXISTS {table}"), ())
        .await
        .unwrap();
    db.sync().await.expect("cleanup sync failed");
}

/// Verify that sync works after a delay — simulating a work session where
/// writes happen over time and sync is deferred to wrap-up.
#[tokio::test(flavor = "multi_thread")]
async fn spike_sync_deferred_batch() {
    let Some((url, token)) = turso_credentials().await else {
        eprintln!("SKIP: Turso credentials not available");
        return;
    };

    let dir = TempDir::new().unwrap();
    let table = unique_table_name("spike_df");

    let db = sync_replica(dir.path().join("replica.db"), &url, &token).await;
    db.sync().await.expect("initial sync failed");
    let conn = db.connect().unwrap();

    conn.execute(
        &format!(
            "CREATE TABLE IF NOT EXISTS {table} (
                id TEXT PRIMARY KEY,
                content TEXT NOT NULL,
                seq INTEGER NOT NULL
            )"
        ),
        (),
    )
    .await
    .unwrap();

    // Simulate multiple writes over a "work session" without syncing
    for i in 0..10 {
        conn.execute(
            &format!("INSERT INTO {table} (id, content, seq) VALUES (?, ?, ?)"),
            libsql::params![format!("batch-{i:03}"), format!("entry number {i}"), i],
        )
        .await
        .unwrap();

        // Small delay to simulate real work
        if i % 3 == 0 {
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    }

    // All 10 writes happened without sync. Now do the "wrap-up" sync.
    db.sync().await.expect("deferred batch sync failed");

    // Verify all 10 rows are present locally
    let mut rows = conn
        .query(
            &format!("SELECT count(*) FROM {table}"),
            (),
        )
        .await
        .unwrap();
    let row = rows.next().await.unwrap().unwrap();
    let count = row.get::<i64>(0).unwrap();
    assert_eq!(count, 10, "all 10 deferred writes should be present after sync");

    // Verify ordering is preserved
    let mut rows = conn
        .query(
            &format!("SELECT id, seq FROM {table} ORDER BY seq"),
            (),
        )
        .await
        .unwrap();
    for i in 0..10 {
        let row = rows.next().await.unwrap().expect("expected row");
        assert_eq!(row.get::<String>(0).unwrap(), format!("batch-{i:03}"));
        assert_eq!(row.get::<i64>(1).unwrap(), i);
    }
    assert!(rows.next().await.unwrap().is_none());

    // Clean up
    conn.execute(&format!("DROP TABLE IF EXISTS {table}"), ())
        .await
        .unwrap();
    db.sync().await.expect("cleanup sync failed");
}

/// Verify that transactions work through the embedded replica.
/// Zenith uses transactions for atomic CRUD + audit writes.
#[tokio::test(flavor = "multi_thread")]
async fn spike_sync_transactions() {
    let Some((url, token)) = turso_credentials().await else {
        eprintln!("SKIP: Turso credentials not available");
        return;
    };

    let dir = TempDir::new().unwrap();
    let table = unique_table_name("spike_tx");

    let db = sync_replica(dir.path().join("replica.db"), &url, &token).await;
    db.sync().await.expect("initial sync failed");
    let conn = db.connect().unwrap();

    conn.execute(
        &format!(
            "CREATE TABLE IF NOT EXISTS {table} (
                id TEXT PRIMARY KEY,
                value TEXT NOT NULL
            )"
        ),
        (),
    )
    .await
    .unwrap();

    // Transaction that commits
    {
        let tx = conn.transaction().await.unwrap();
        tx.execute(
            &format!("INSERT INTO {table} (id, value) VALUES (?, ?)"),
            libsql::params!["tx-committed", "this should persist"],
        )
        .await
        .unwrap();
        tx.commit().await.unwrap();
    }

    // Transaction that rolls back
    {
        let tx = conn.transaction().await.unwrap();
        tx.execute(
            &format!("INSERT INTO {table} (id, value) VALUES (?, ?)"),
            libsql::params!["tx-rolled-back", "this should vanish"],
        )
        .await
        .unwrap();
        tx.rollback().await.unwrap();
    }

    // Sync after transactions
    db.sync().await.expect("transaction sync failed");

    // Verify: only committed transaction's data exists
    let mut rows = conn
        .query(
            &format!("SELECT id, value FROM {table} ORDER BY id"),
            (),
        )
        .await
        .unwrap();

    let row = rows.next().await.unwrap().expect("expected committed row");
    assert_eq!(row.get::<String>(0).unwrap(), "tx-committed");
    assert_eq!(row.get::<String>(1).unwrap(), "this should persist");

    assert!(
        rows.next().await.unwrap().is_none(),
        "rolled-back row should not exist"
    );

    // Clean up
    conn.execute(&format!("DROP TABLE IF EXISTS {table}"), ())
        .await
        .unwrap();
    db.sync().await.expect("cleanup sync failed");
}
