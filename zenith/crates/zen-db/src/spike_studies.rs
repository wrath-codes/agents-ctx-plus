//! # Spike 0.11: Studies Feature — Data Model Validation
//!
//! **DONE** — Approach B (hybrid) selected. One new `studies` table + reuse existing entities.
//!
//! Spike validates the best data model approach for a structured learning ("studies") feature
//! by testing two approaches against the same real-world scenario: **"Learn how tokio::spawn works."**
//!
//! ## Decision: Approach B (Hybrid)
//!
//! Approach B wins on ergonomics and type safety despite slightly more INSERTs:
//! - **Type-safe filtering**: `FROM studies WHERE ...` vs `WHERE title LIKE 'Study: %'`
//! - **Purpose-built fields**: `topic`, `library`, `methodology`, `summary` as first-class columns
//! - **Dedicated lifecycle**: `active → concluding → completed | abandoned`
//! - **CLI naturalness**: `zen study create --topic "..." --library tokio`
//!
//! Key finding: hypotheses can still use `research_id` FK (since study links to a research_item),
//! giving both the direct FK query path AND the entity_links path.
//!
//! See `docs/schema/09-studies-workflow.md` for the full design.
//!
//! ## Approach A: Compose From Existing Entities (8 tests)
//!
//! Uses `research_items` with conventions (title prefix, tags, description format) — same
//! pattern as the PRD workflow which uses `issues` with `type = 'epic'`. No new tables.
//!
//! - Studies are `research_items` with `Study:` title prefix
//! - Assumptions are `hypotheses` linked via `research_id`
//! - Test results are `findings` tagged `test-result`
//! - Evidence chains use `entity_links` (`validates`, `debunks`)
//! - Conclusions are `insights` linked via `research_id`
//!
//! ## Approach B: Hybrid — One New Table (6 tests)
//!
//! Adds a single `studies` table as a container with its own FTS5 index. All content
//! entities (hypotheses, findings, insights) are reused from the existing schema and
//! linked to the study via `entity_links`.
//!
//! - `studies` table: topic, library, methodology, status, summary
//! - Hypotheses, findings, insights linked via `entity_links` (`source_type = 'study'`)
//! - Dedicated status lifecycle: `active → concluding → completed | abandoned`
//! - ID prefix: `stu-`
//!
//! ## Part C: Comparison (1 test)
//!
//! Side-by-side comparison of both approaches on concrete metrics:
//! SQL complexity, INSERT count, progress query complexity, filterability.
//!
//! ## Validates
//!
//! Which data model approach to use for the studies feature — blocks `08-studies-workflow.md`.
//!
//! ## Scenario
//!
//! Both approaches run the same scenario:
//! 1. Create a study: "How does tokio::spawn work?"
//! 2. Form 3 assumptions (hypotheses)
//! 3. Validate 2 assumptions (findings + entity_links)
//! 4. Invalidate 1 assumption
//! 5. Conclude the study (insight + status update)
//! 6. Query full study state
//! 7. Track progress (N validated, M invalidated, K untested)

use libsql::Builder;

/// Helper: create an in-memory database for tests.
async fn in_memory_db() -> libsql::Database {
    Builder::new_local(":memory:")
        .build()
        .await
        .expect("failed to create in-memory database")
}

/// The shared schema needed by both approaches: sessions, research_items, findings,
/// finding_tags, hypotheses, insights, entity_links, audit_trail + FTS5 + triggers.
/// This is a subset of the full Zenith schema from `01-turso-data-model.md`.
const SHARED_SCHEMA: &str = "
    PRAGMA foreign_keys = ON;

    CREATE TABLE sessions (
        id TEXT PRIMARY KEY,
        status TEXT NOT NULL DEFAULT 'active',
        started_at TEXT NOT NULL DEFAULT (datetime('now')),
        ended_at TEXT,
        summary TEXT
    );

    CREATE TABLE research_items (
        id TEXT PRIMARY KEY,
        session_id TEXT REFERENCES sessions(id),
        title TEXT NOT NULL,
        description TEXT,
        status TEXT NOT NULL DEFAULT 'open',
        created_at TEXT NOT NULL DEFAULT (datetime('now')),
        updated_at TEXT NOT NULL DEFAULT (datetime('now'))
    );

    CREATE TABLE findings (
        id TEXT PRIMARY KEY,
        research_id TEXT REFERENCES research_items(id),
        session_id TEXT REFERENCES sessions(id),
        content TEXT NOT NULL,
        source TEXT,
        confidence TEXT NOT NULL DEFAULT 'medium',
        created_at TEXT NOT NULL DEFAULT (datetime('now')),
        updated_at TEXT NOT NULL DEFAULT (datetime('now'))
    );

    CREATE TABLE finding_tags (
        finding_id TEXT NOT NULL REFERENCES findings(id),
        tag TEXT NOT NULL,
        PRIMARY KEY (finding_id, tag)
    );

    CREATE TABLE hypotheses (
        id TEXT PRIMARY KEY,
        research_id TEXT REFERENCES research_items(id),
        finding_id TEXT REFERENCES findings(id),
        session_id TEXT REFERENCES sessions(id),
        content TEXT NOT NULL,
        status TEXT NOT NULL DEFAULT 'unverified',
        reason TEXT,
        created_at TEXT NOT NULL DEFAULT (datetime('now')),
        updated_at TEXT NOT NULL DEFAULT (datetime('now'))
    );

    CREATE TABLE insights (
        id TEXT PRIMARY KEY,
        research_id TEXT REFERENCES research_items(id),
        session_id TEXT REFERENCES sessions(id),
        content TEXT NOT NULL,
        confidence TEXT NOT NULL DEFAULT 'medium',
        created_at TEXT NOT NULL DEFAULT (datetime('now')),
        updated_at TEXT NOT NULL DEFAULT (datetime('now'))
    );

    CREATE TABLE entity_links (
        id TEXT PRIMARY KEY,
        source_type TEXT NOT NULL,
        source_id TEXT NOT NULL,
        target_type TEXT NOT NULL,
        target_id TEXT NOT NULL,
        relation TEXT NOT NULL,
        created_at TEXT NOT NULL DEFAULT (datetime('now')),
        UNIQUE(source_type, source_id, target_type, target_id, relation)
    );

    CREATE TABLE audit_trail (
        id TEXT PRIMARY KEY,
        session_id TEXT REFERENCES sessions(id),
        entity_type TEXT NOT NULL,
        entity_id TEXT NOT NULL,
        action TEXT NOT NULL,
        detail TEXT,
        created_at TEXT NOT NULL DEFAULT (datetime('now'))
    );

    -- FTS5 virtual tables
    CREATE VIRTUAL TABLE research_fts USING fts5(
        title, description,
        content='research_items',
        content_rowid='rowid',
        tokenize='porter unicode61'
    );

    CREATE VIRTUAL TABLE findings_fts USING fts5(
        content, source,
        content='findings',
        content_rowid='rowid',
        tokenize='porter unicode61'
    );

    CREATE VIRTUAL TABLE hypotheses_fts USING fts5(
        content, reason,
        content='hypotheses',
        content_rowid='rowid',
        tokenize='porter unicode61'
    );

    CREATE VIRTUAL TABLE insights_fts USING fts5(
        content,
        content='insights',
        content_rowid='rowid',
        tokenize='porter unicode61'
    );

    -- FTS sync triggers: research_items
    CREATE TRIGGER research_ai AFTER INSERT ON research_items BEGIN
        INSERT INTO research_fts(rowid, title, description)
        VALUES (new.rowid, new.title, new.description);
    END;
    CREATE TRIGGER research_au AFTER UPDATE ON research_items BEGIN
        INSERT INTO research_fts(research_fts, rowid, title, description)
        VALUES ('delete', old.rowid, old.title, old.description);
        INSERT INTO research_fts(rowid, title, description)
        VALUES (new.rowid, new.title, new.description);
    END;

    -- FTS sync triggers: findings
    CREATE TRIGGER findings_ai AFTER INSERT ON findings BEGIN
        INSERT INTO findings_fts(rowid, content, source)
        VALUES (new.rowid, new.content, new.source);
    END;
    CREATE TRIGGER findings_au AFTER UPDATE ON findings BEGIN
        INSERT INTO findings_fts(findings_fts, rowid, content, source)
        VALUES ('delete', old.rowid, old.content, old.source);
        INSERT INTO findings_fts(rowid, content, source)
        VALUES (new.rowid, new.content, new.source);
    END;

    -- FTS sync triggers: hypotheses
    CREATE TRIGGER hypotheses_ai AFTER INSERT ON hypotheses BEGIN
        INSERT INTO hypotheses_fts(rowid, content, reason)
        VALUES (new.rowid, new.content, new.reason);
    END;
    CREATE TRIGGER hypotheses_au AFTER UPDATE ON hypotheses BEGIN
        INSERT INTO hypotheses_fts(hypotheses_fts, rowid, content, reason)
        VALUES ('delete', old.rowid, old.content, old.reason);
        INSERT INTO hypotheses_fts(rowid, content, reason)
        VALUES (new.rowid, new.content, new.reason);
    END;

    -- FTS sync triggers: insights
    CREATE TRIGGER insights_ai AFTER INSERT ON insights BEGIN
        INSERT INTO insights_fts(rowid, content)
        VALUES (new.rowid, new.content);
    END;
    CREATE TRIGGER insights_au AFTER UPDATE ON insights BEGIN
        INSERT INTO insights_fts(insights_fts, rowid, content)
        VALUES ('delete', old.rowid, old.content);
        INSERT INTO insights_fts(rowid, content)
        VALUES (new.rowid, new.content);
    END;

    -- Indexes
    CREATE INDEX idx_findings_research ON findings(research_id);
    CREATE INDEX idx_hypotheses_research ON hypotheses(research_id);
    CREATE INDEX idx_hypotheses_status ON hypotheses(status);
    CREATE INDEX idx_insights_research ON insights(research_id);
    CREATE INDEX idx_entity_links_source ON entity_links(source_type, source_id);
    CREATE INDEX idx_entity_links_target ON entity_links(target_type, target_id);
    CREATE INDEX idx_entity_links_relation ON entity_links(relation);
    CREATE INDEX idx_finding_tags_tag ON finding_tags(tag);
";

/// Additional schema for Approach B: the studies table + FTS5 + triggers.
const STUDIES_TABLE_SCHEMA: &str = "
    CREATE TABLE studies (
        id TEXT PRIMARY KEY,
        session_id TEXT REFERENCES sessions(id),
        research_id TEXT REFERENCES research_items(id),
        topic TEXT NOT NULL,
        library TEXT,
        methodology TEXT NOT NULL DEFAULT 'explore',
        status TEXT NOT NULL DEFAULT 'active',
        summary TEXT,
        created_at TEXT NOT NULL DEFAULT (datetime('now')),
        updated_at TEXT NOT NULL DEFAULT (datetime('now'))
    );

    CREATE VIRTUAL TABLE studies_fts USING fts5(
        topic, summary,
        content='studies',
        content_rowid='rowid',
        tokenize='porter unicode61'
    );

    CREATE TRIGGER studies_ai AFTER INSERT ON studies BEGIN
        INSERT INTO studies_fts(rowid, topic, summary)
        VALUES (new.rowid, new.topic, new.summary);
    END;
    CREATE TRIGGER studies_au AFTER UPDATE ON studies BEGIN
        INSERT INTO studies_fts(studies_fts, rowid, topic, summary)
        VALUES ('delete', old.rowid, old.topic, old.summary);
        INSERT INTO studies_fts(rowid, topic, summary)
        VALUES (new.rowid, new.topic, new.summary);
    END;

    CREATE INDEX idx_studies_status ON studies(status);
    CREATE INDEX idx_studies_library ON studies(library);
    CREATE INDEX idx_studies_session ON studies(session_id);
    CREATE INDEX idx_studies_research ON studies(research_id);
";

/// Helper: set up in-memory DB with the shared schema and a test session.
/// Returns (Database, Connection, session_id). Database must be kept alive for the connection to work.
async fn setup_shared() -> (libsql::Database, libsql::Connection, String) {
    let db = in_memory_db().await;
    let conn = db.connect().expect("failed to connect");
    conn.execute_batch(SHARED_SCHEMA).await.unwrap();

    // Create a test session
    conn.execute(
        "INSERT INTO sessions (id, status) VALUES (?, ?)",
        libsql::params!["ses-test0001", "active"],
    )
    .await
    .unwrap();

    (db, conn, "ses-test0001".to_string())
}

/// Helper: set up in-memory DB with shared schema + studies table.
async fn setup_with_studies() -> (libsql::Database, libsql::Connection, String) {
    let db = in_memory_db().await;
    let conn = db.connect().expect("failed to connect");
    conn.execute_batch(SHARED_SCHEMA).await.unwrap();
    conn.execute_batch(STUDIES_TABLE_SCHEMA).await.unwrap();

    conn.execute(
        "INSERT INTO sessions (id, status) VALUES (?, ?)",
        libsql::params!["ses-test0001", "active"],
    )
    .await
    .unwrap();

    (db, conn, "ses-test0001".to_string())
}

// ===========================================================================
// Part A: Compose-Only Approach (8 tests)
// ===========================================================================

/// A.1: Create a study as a research_item with "Study:" title prefix.
#[tokio::test]
async fn spike_a_create_study_as_research() {
    let (_db, conn, session_id) = setup_shared().await;

    let study_plan = "## Study Plan\n\n\
        ### Questions\n\
        1. What are the Send + 'static requirements for spawn?\n\
        2. What happens when a spawned task panics?\n\
        3. Is spawn zero-cost (no allocation)?\n\n\
        ### Methodology\n\
        explore: read docs, write test code, validate assumptions";

    conn.execute(
        "INSERT INTO research_items (id, session_id, title, description, status)
         VALUES (?, ?, ?, ?, ?)",
        libsql::params!["res-study001", session_id.as_str(), "Study: How tokio::spawn works", study_plan, "in_progress"],
    )
    .await
    .unwrap();

    // Verify it can be queried
    let mut rows = conn
        .query("SELECT id, title, description, status FROM research_items WHERE id = ?", ["res-study001"])
        .await
        .unwrap();
    let row = rows.next().await.unwrap().expect("expected study row");
    assert_eq!(row.get::<String>(0).unwrap(), "res-study001");
    assert!(row.get::<String>(1).unwrap().starts_with("Study: "));
    assert!(row.get::<String>(2).unwrap().contains("## Study Plan"));
    assert_eq!(row.get::<String>(3).unwrap(), "in_progress");

    // Verify FTS works on the study
    let mut rows = conn
        .query(
            "SELECT r.id FROM research_fts fts
             JOIN research_items r ON r.rowid = fts.rowid
             WHERE research_fts MATCH ?",
            ["tokio spawn"],
        )
        .await
        .unwrap();
    let row = rows.next().await.unwrap().expect("FTS should find the study");
    assert_eq!(row.get::<String>(0).unwrap(), "res-study001");
}

/// A.2: Add 3 assumptions as hypotheses linked to the study research_item.
#[tokio::test]
async fn spike_a_add_assumptions() {
    let (_db, conn, session_id) = setup_shared().await;

    // Create the study
    conn.execute(
        "INSERT INTO research_items (id, session_id, title, status)
         VALUES (?, ?, ?, ?)",
        libsql::params!["res-study001", session_id.as_str(), "Study: How tokio::spawn works", "in_progress"],
    )
    .await
    .unwrap();

    // Add 3 assumptions as hypotheses
    let assumptions = [
        ("hyp-asm001", "spawn requires Send + 'static bounds on the future"),
        ("hyp-asm002", "spawned tasks can panic without crashing the runtime"),
        ("hyp-asm003", "spawn is zero-cost (no allocation at spawn time)"),
    ];

    for (id, content) in &assumptions {
        conn.execute(
            "INSERT INTO hypotheses (id, research_id, session_id, content, status)
             VALUES (?, ?, ?, ?, ?)",
            libsql::params![*id, "res-study001", session_id.as_str(), *content, "unverified"],
        )
        .await
        .unwrap();
    }

    // Verify all 3 are linked and unverified
    let mut rows = conn
        .query(
            "SELECT id, content, status FROM hypotheses
             WHERE research_id = ? ORDER BY id",
            ["res-study001"],
        )
        .await
        .unwrap();

    let mut count = 0;
    while let Some(row) = rows.next().await.unwrap() {
        assert_eq!(row.get::<String>(2).unwrap(), "unverified");
        count += 1;
    }
    assert_eq!(count, 3);
}

/// A.3: Record a validated test result — finding tagged `test-result` linked via `validates`.
#[tokio::test]
async fn spike_a_record_test_validated() {
    let (_db, conn, session_id) = setup_shared().await;

    // Set up study + assumption
    conn.execute(
        "INSERT INTO research_items (id, session_id, title, status) VALUES (?, ?, ?, ?)",
        libsql::params!["res-study001", session_id.as_str(), "Study: How tokio::spawn works", "in_progress"],
    )
    .await
    .unwrap();

    conn.execute(
        "INSERT INTO hypotheses (id, research_id, session_id, content, status) VALUES (?, ?, ?, ?, ?)",
        libsql::params!["hyp-asm001", "res-study001", session_id.as_str(), "spawn requires Send + 'static bounds", "unverified"],
    )
    .await
    .unwrap();

    // LLM runs code, gets compile error E0277 — records as finding
    conn.execute(
        "INSERT INTO findings (id, research_id, session_id, content, source, confidence) VALUES (?, ?, ?, ?, ?, ?)",
        libsql::params![
            "fnd-test001", "res-study001", session_id.as_str(),
            "Test: spawn with non-Send type produces compile error E0277: `Rc<i32>` cannot be sent between threads safely. Confirms Send bound is enforced at compile time.",
            "manual:spike-code", "high"
        ],
    )
    .await
    .unwrap();

    // Tag as test-result
    conn.execute(
        "INSERT INTO finding_tags (finding_id, tag) VALUES (?, ?)",
        libsql::params!["fnd-test001", "test-result"],
    )
    .await
    .unwrap();

    // Link finding → hypothesis (validates)
    conn.execute(
        "INSERT INTO entity_links (id, source_type, source_id, target_type, target_id, relation)
         VALUES (?, ?, ?, ?, ?, ?)",
        libsql::params!["lnk-val001", "finding", "fnd-test001", "hypothesis", "hyp-asm001", "validates"],
    )
    .await
    .unwrap();

    // Update hypothesis status
    conn.execute(
        "UPDATE hypotheses SET status = 'confirmed', reason = ?, updated_at = datetime('now')
         WHERE id = ?",
        libsql::params!["Compile error E0277 proves Send + 'static is required", "hyp-asm001"],
    )
    .await
    .unwrap();

    // Verify the chain: hypothesis confirmed + finding linked
    let mut rows = conn
        .query("SELECT status, reason FROM hypotheses WHERE id = ?", ["hyp-asm001"])
        .await
        .unwrap();
    let row = rows.next().await.unwrap().unwrap();
    assert_eq!(row.get::<String>(0).unwrap(), "confirmed");
    assert!(row.get::<String>(1).unwrap().contains("E0277"));

    // Verify entity_link exists
    let mut rows = conn
        .query(
            "SELECT relation FROM entity_links
             WHERE source_type = 'finding' AND source_id = ? AND target_id = ?",
            libsql::params!["fnd-test001", "hyp-asm001"],
        )
        .await
        .unwrap();
    let row = rows.next().await.unwrap().unwrap();
    assert_eq!(row.get::<String>(0).unwrap(), "validates");

    // Verify tag
    let mut rows = conn
        .query("SELECT tag FROM finding_tags WHERE finding_id = ?", ["fnd-test001"])
        .await
        .unwrap();
    let row = rows.next().await.unwrap().unwrap();
    assert_eq!(row.get::<String>(0).unwrap(), "test-result");
}

/// A.4: Record an invalidated test result — finding linked via `debunks`.
#[tokio::test]
async fn spike_a_record_test_invalidated() {
    let (_db, conn, session_id) = setup_shared().await;

    conn.execute(
        "INSERT INTO research_items (id, session_id, title, status) VALUES (?, ?, ?, ?)",
        libsql::params!["res-study001", session_id.as_str(), "Study: How tokio::spawn works", "in_progress"],
    )
    .await
    .unwrap();

    conn.execute(
        "INSERT INTO hypotheses (id, research_id, session_id, content, status) VALUES (?, ?, ?, ?, ?)",
        libsql::params!["hyp-asm003", "res-study001", session_id.as_str(), "spawn is zero-cost (no allocation at spawn time)", "unverified"],
    )
    .await
    .unwrap();

    // LLM runs code, discovers spawn DOES allocate — records as finding
    conn.execute(
        "INSERT INTO findings (id, research_id, session_id, content, source, confidence) VALUES (?, ?, ?, ?, ?, ?)",
        libsql::params![
            "fnd-test003", "res-study001", session_id.as_str(),
            "Test: spawn allocates a JoinHandle and task harness on the heap. It is NOT zero-cost. The allocation is ~200 bytes per task on x86_64.",
            "manual:spike-code", "high"
        ],
    )
    .await
    .unwrap();

    conn.execute(
        "INSERT INTO finding_tags (finding_id, tag) VALUES (?, ?)",
        libsql::params!["fnd-test003", "test-result"],
    )
    .await
    .unwrap();

    // Link: finding debunks hypothesis
    conn.execute(
        "INSERT INTO entity_links (id, source_type, source_id, target_type, target_id, relation)
         VALUES (?, ?, ?, ?, ?, ?)",
        libsql::params!["lnk-deb001", "finding", "fnd-test003", "hypothesis", "hyp-asm003", "debunks"],
    )
    .await
    .unwrap();

    conn.execute(
        "UPDATE hypotheses SET status = 'debunked', reason = ?, updated_at = datetime('now') WHERE id = ?",
        libsql::params!["spawn allocates ~200 bytes per task for JoinHandle + task harness", "hyp-asm003"],
    )
    .await
    .unwrap();

    // Verify
    let mut rows = conn
        .query("SELECT status, reason FROM hypotheses WHERE id = ?", ["hyp-asm003"])
        .await
        .unwrap();
    let row = rows.next().await.unwrap().unwrap();
    assert_eq!(row.get::<String>(0).unwrap(), "debunked");

    let mut rows = conn
        .query(
            "SELECT relation FROM entity_links
             WHERE source_id = ? AND target_id = ?",
            libsql::params!["fnd-test003", "hyp-asm003"],
        )
        .await
        .unwrap();
    let row = rows.next().await.unwrap().unwrap();
    assert_eq!(row.get::<String>(0).unwrap(), "debunks");
}

/// A.5: Conclude the study — create insight, resolve research.
#[tokio::test]
async fn spike_a_conclude_study() {
    let (_db, conn, session_id) = setup_shared().await;

    conn.execute(
        "INSERT INTO research_items (id, session_id, title, status) VALUES (?, ?, ?, ?)",
        libsql::params!["res-study001", session_id.as_str(), "Study: How tokio::spawn works", "in_progress"],
    )
    .await
    .unwrap();

    let conclusion = "## Study Conclusions: How tokio::spawn works\n\n\
        ### Confirmed\n\
        - spawn requires Send + 'static bounds (compile-time enforcement via E0277)\n\
        - Spawned tasks can panic without crashing the runtime (JoinHandle returns JoinError)\n\n\
        ### Debunked\n\
        - spawn is NOT zero-cost: allocates ~200 bytes per task for JoinHandle + harness\n\n\
        ### Procedures Learned\n\
        - Use spawn_local on a LocalSet for non-Send data\n\
        - Always .await the JoinHandle or explicitly drop it\n\n\
        ### Open Questions\n\
        - How does spawn interact with structured concurrency patterns?";

    conn.execute(
        "INSERT INTO insights (id, research_id, session_id, content, confidence) VALUES (?, ?, ?, ?, ?)",
        libsql::params!["ins-conc001", "res-study001", session_id.as_str(), conclusion, "high"],
    )
    .await
    .unwrap();

    // Resolve the study
    conn.execute(
        "UPDATE research_items SET status = 'resolved', updated_at = datetime('now') WHERE id = ?",
        ["res-study001"],
    )
    .await
    .unwrap();

    // Verify
    let mut rows = conn
        .query("SELECT status FROM research_items WHERE id = ?", ["res-study001"])
        .await
        .unwrap();
    assert_eq!(
        rows.next().await.unwrap().unwrap().get::<String>(0).unwrap(),
        "resolved"
    );

    let mut rows = conn
        .query("SELECT content FROM insights WHERE research_id = ?", ["res-study001"])
        .await
        .unwrap();
    let row = rows.next().await.unwrap().unwrap();
    assert!(row.get::<String>(0).unwrap().contains("## Study Conclusions"));
}

/// A.6: Query the full study state in a single query.
#[tokio::test]
async fn spike_a_query_full_state() {
    let (_db, conn, session_id) = setup_shared().await;

    // Set up full study scenario
    conn.execute(
        "INSERT INTO research_items (id, session_id, title, description, status) VALUES (?, ?, ?, ?, ?)",
        libsql::params!["res-study001", session_id.as_str(), "Study: How tokio::spawn works", "Study plan...", "in_progress"],
    )
    .await
    .unwrap();

    // 3 hypotheses
    for (id, content, status, reason) in [
        ("hyp-asm001", "spawn requires Send + 'static", "confirmed", "E0277 proves it"),
        ("hyp-asm002", "panic doesn't crash runtime", "confirmed", "JoinHandle catches it"),
        ("hyp-asm003", "spawn is zero-cost", "debunked", "Allocates ~200 bytes"),
    ] {
        conn.execute(
            "INSERT INTO hypotheses (id, research_id, session_id, content, status, reason) VALUES (?, ?, ?, ?, ?, ?)",
            libsql::params![id, "res-study001", session_id.as_str(), content, status, reason],
        )
        .await
        .unwrap();
    }

    // 3 findings tagged test-result
    for (id, content) in [
        ("fnd-test001", "Test: non-Send type → E0277"),
        ("fnd-test002", "Test: panic in task → JoinError, runtime continues"),
        ("fnd-test003", "Test: spawn allocates ~200 bytes"),
    ] {
        conn.execute(
            "INSERT INTO findings (id, research_id, session_id, content, source, confidence) VALUES (?, ?, ?, ?, ?, ?)",
            libsql::params![id, "res-study001", session_id.as_str(), content, "manual:spike-code", "high"],
        )
        .await
        .unwrap();
        conn.execute(
            "INSERT INTO finding_tags (finding_id, tag) VALUES (?, ?)",
            libsql::params![id, "test-result"],
        )
        .await
        .unwrap();
    }

    // 1 insight
    conn.execute(
        "INSERT INTO insights (id, research_id, session_id, content, confidence) VALUES (?, ?, ?, ?, ?)",
        libsql::params!["ins-conc001", "res-study001", session_id.as_str(), "Study conclusions...", "high"],
    )
    .await
    .unwrap();

    // -----------------------------------------------------------------------
    // THE KEY QUERY: Get full study state in one shot
    // -----------------------------------------------------------------------
    let full_state_sql = "
        SELECT
            r.id, r.title, r.description, r.status,
            (SELECT json_group_array(json_object(
                'id', h.id, 'content', h.content, 'status', h.status, 'reason', h.reason
            )) FROM hypotheses h WHERE h.research_id = r.id) as assumptions,
            (SELECT json_group_array(json_object(
                'id', f.id, 'content', f.content, 'confidence', f.confidence
            )) FROM findings f WHERE f.research_id = r.id) as findings,
            (SELECT json_group_array(json_object(
                'id', i.id, 'content', i.content
            )) FROM insights i WHERE i.research_id = r.id) as conclusions
        FROM research_items r
        WHERE r.id = ?
    ";

    let mut rows = conn.query(full_state_sql, ["res-study001"]).await.unwrap();
    let row = rows.next().await.unwrap().expect("expected study state");

    // Verify top-level
    assert_eq!(row.get::<String>(0).unwrap(), "res-study001");
    assert!(row.get::<String>(1).unwrap().starts_with("Study: "));

    // Verify assumptions JSON
    let assumptions_json = row.get::<String>(4).unwrap();
    assert!(assumptions_json.contains("hyp-asm001"));
    assert!(assumptions_json.contains("hyp-asm002"));
    assert!(assumptions_json.contains("hyp-asm003"));
    assert!(assumptions_json.contains("confirmed"));
    assert!(assumptions_json.contains("debunked"));

    // Verify findings JSON
    let findings_json = row.get::<String>(5).unwrap();
    assert!(findings_json.contains("fnd-test001"));
    assert!(findings_json.contains("fnd-test002"));
    assert!(findings_json.contains("fnd-test003"));

    // Verify conclusions JSON
    let conclusions_json = row.get::<String>(6).unwrap();
    assert!(conclusions_json.contains("ins-conc001"));
}

/// A.7: Distinguish studies from regular research.
#[tokio::test]
async fn spike_a_distinguish_from_research() {
    let (_db, conn, session_id) = setup_shared().await;

    // Create a regular research item
    conn.execute(
        "INSERT INTO research_items (id, session_id, title, status) VALUES (?, ?, ?, ?)",
        libsql::params!["res-regular1", session_id.as_str(), "Evaluate HTTP client libraries for Rust", "open"],
    )
    .await
    .unwrap();

    // Create a study
    conn.execute(
        "INSERT INTO research_items (id, session_id, title, status) VALUES (?, ?, ?, ?)",
        libsql::params!["res-study001", session_id.as_str(), "Study: How tokio::spawn works", "in_progress"],
    )
    .await
    .unwrap();

    // Create another regular research
    conn.execute(
        "INSERT INTO research_items (id, session_id, title, status) VALUES (?, ?, ?, ?)",
        libsql::params!["res-regular2", session_id.as_str(), "Check serde compatibility with axum 0.8", "open"],
    )
    .await
    .unwrap();

    // Method 1: Filter by title prefix
    let mut rows = conn
        .query(
            "SELECT id FROM research_items WHERE title LIKE 'Study: %' ORDER BY id",
            (),
        )
        .await
        .unwrap();
    let mut study_ids = Vec::new();
    while let Some(row) = rows.next().await.unwrap() {
        study_ids.push(row.get::<String>(0).unwrap());
    }
    assert_eq!(study_ids, vec!["res-study001"]);

    // Method 2: Filter for NON-studies
    let mut rows = conn
        .query(
            "SELECT id FROM research_items WHERE title NOT LIKE 'Study: %' ORDER BY id",
            (),
        )
        .await
        .unwrap();
    let mut regular_ids = Vec::new();
    while let Some(row) = rows.next().await.unwrap() {
        regular_ids.push(row.get::<String>(0).unwrap());
    }
    assert_eq!(regular_ids, vec!["res-regular1", "res-regular2"]);

    // Method 3: FTS search within studies only
    let mut rows = conn
        .query(
            "SELECT r.id FROM research_fts fts
             JOIN research_items r ON r.rowid = fts.rowid
             WHERE research_fts MATCH 'tokio' AND r.title LIKE 'Study: %'",
            (),
        )
        .await
        .unwrap();
    let row = rows.next().await.unwrap().expect("should find study via FTS + filter");
    assert_eq!(row.get::<String>(0).unwrap(), "res-study001");
}

/// A.8: Track progress — count hypotheses by status.
#[tokio::test]
async fn spike_a_progress_tracking() {
    let (_db, conn, session_id) = setup_shared().await;

    conn.execute(
        "INSERT INTO research_items (id, session_id, title, status) VALUES (?, ?, ?, ?)",
        libsql::params!["res-study001", session_id.as_str(), "Study: How tokio::spawn works", "in_progress"],
    )
    .await
    .unwrap();

    // 2 confirmed, 1 debunked
    for (id, status) in [
        ("hyp-asm001", "confirmed"),
        ("hyp-asm002", "confirmed"),
        ("hyp-asm003", "debunked"),
    ] {
        conn.execute(
            "INSERT INTO hypotheses (id, research_id, session_id, content, status) VALUES (?, ?, ?, ?, ?)",
            libsql::params![id, "res-study001", session_id.as_str(), "assumption content", status],
        )
        .await
        .unwrap();
    }

    // Progress query
    let progress_sql = "
        SELECT
            COUNT(*) as total,
            SUM(CASE WHEN status = 'confirmed' THEN 1 ELSE 0 END) as confirmed,
            SUM(CASE WHEN status = 'debunked' THEN 1 ELSE 0 END) as debunked,
            SUM(CASE WHEN status = 'partially_confirmed' THEN 1 ELSE 0 END) as partial,
            SUM(CASE WHEN status = 'unverified' THEN 1 ELSE 0 END) as untested,
            SUM(CASE WHEN status = 'analyzing' THEN 1 ELSE 0 END) as in_progress,
            SUM(CASE WHEN status = 'inconclusive' THEN 1 ELSE 0 END) as inconclusive
        FROM hypotheses
        WHERE research_id = ?
    ";

    let mut rows = conn.query(progress_sql, ["res-study001"]).await.unwrap();
    let row = rows.next().await.unwrap().expect("expected progress row");

    assert_eq!(row.get::<i64>(0).unwrap(), 3); // total
    assert_eq!(row.get::<i64>(1).unwrap(), 2); // confirmed
    assert_eq!(row.get::<i64>(2).unwrap(), 1); // debunked
    assert_eq!(row.get::<i64>(3).unwrap(), 0); // partial
    assert_eq!(row.get::<i64>(4).unwrap(), 0); // untested
    assert_eq!(row.get::<i64>(5).unwrap(), 0); // in_progress
    assert_eq!(row.get::<i64>(6).unwrap(), 0); // inconclusive
}

// ===========================================================================
// Part B: Hybrid Approach — One New `studies` Table (6 tests)
// ===========================================================================

/// B.1: Create the studies table, insert a study, verify FTS works.
#[tokio::test]
async fn spike_b_create_study_table() {
    let (_db, conn, session_id) = setup_with_studies().await;

    // Also create a research_item to link to (optional but realistic)
    conn.execute(
        "INSERT INTO research_items (id, session_id, title, status) VALUES (?, ?, ?, ?)",
        libsql::params!["res-study001", session_id.as_str(), "How tokio::spawn works", "in_progress"],
    )
    .await
    .unwrap();

    // Insert a study
    conn.execute(
        "INSERT INTO studies (id, session_id, research_id, topic, library, methodology, status)
         VALUES (?, ?, ?, ?, ?, ?, ?)",
        libsql::params![
            "stu-test0001", session_id.as_str(), "res-study001",
            "How tokio::spawn works", "tokio", "explore", "active"
        ],
    )
    .await
    .unwrap();

    // Verify persistence
    let mut rows = conn
        .query("SELECT id, topic, library, methodology, status FROM studies WHERE id = ?", ["stu-test0001"])
        .await
        .unwrap();
    let row = rows.next().await.unwrap().expect("expected study");
    assert_eq!(row.get::<String>(0).unwrap(), "stu-test0001");
    assert_eq!(row.get::<String>(1).unwrap(), "How tokio::spawn works");
    assert_eq!(row.get::<String>(2).unwrap(), "tokio");
    assert_eq!(row.get::<String>(3).unwrap(), "explore");
    assert_eq!(row.get::<String>(4).unwrap(), "active");

    // Verify FTS works on the study
    let mut rows = conn
        .query(
            "SELECT s.id FROM studies_fts fts
             JOIN studies s ON s.rowid = fts.rowid
             WHERE studies_fts MATCH ?",
            ["tokio spawn"],
        )
        .await
        .unwrap();
    let row = rows.next().await.unwrap().expect("FTS should find study");
    assert_eq!(row.get::<String>(0).unwrap(), "stu-test0001");
}

/// B.2: Add assumptions (hypotheses) linked to study via entity_links.
#[tokio::test]
async fn spike_b_add_assumptions_via_links() {
    let (_db, conn, session_id) = setup_with_studies().await;

    conn.execute(
        "INSERT INTO research_items (id, session_id, title, status) VALUES (?, ?, ?, ?)",
        libsql::params!["res-study001", session_id.as_str(), "How tokio::spawn works", "in_progress"],
    )
    .await
    .unwrap();

    conn.execute(
        "INSERT INTO studies (id, session_id, research_id, topic, library) VALUES (?, ?, ?, ?, ?)",
        libsql::params!["stu-test0001", session_id.as_str(), "res-study001", "How tokio::spawn works", "tokio"],
    )
    .await
    .unwrap();

    // Create hypotheses and link to study
    let assumptions = [
        ("hyp-asm001", "spawn requires Send + 'static bounds"),
        ("hyp-asm002", "spawned tasks can panic without crashing runtime"),
        ("hyp-asm003", "spawn is zero-cost (no allocation)"),
    ];

    for (i, (hyp_id, content)) in assumptions.iter().enumerate() {
        // Note: hypotheses don't have a study_id FK — we link via entity_links
        // We can still use research_id for the research_item link
        conn.execute(
            "INSERT INTO hypotheses (id, research_id, session_id, content, status) VALUES (?, ?, ?, ?, ?)",
            libsql::params![*hyp_id, "res-study001", session_id.as_str(), *content, "unverified"],
        )
        .await
        .unwrap();

        // Link study → hypothesis
        let link_id = format!("lnk-sh{:03}", i);
        conn.execute(
            "INSERT INTO entity_links (id, source_type, source_id, target_type, target_id, relation)
             VALUES (?, ?, ?, ?, ?, ?)",
            libsql::params![link_id.as_str(), "study", "stu-test0001", "hypothesis", *hyp_id, "relates-to"],
        )
        .await
        .unwrap();
    }

    // Query hypotheses linked to the study via entity_links
    let mut rows = conn
        .query(
            "SELECT h.id, h.content, h.status
             FROM entity_links el
             JOIN hypotheses h ON h.id = el.target_id
             WHERE el.source_type = 'study' AND el.source_id = ?
               AND el.target_type = 'hypothesis'
             ORDER BY h.id",
            ["stu-test0001"],
        )
        .await
        .unwrap();

    let mut count = 0;
    while let Some(row) = rows.next().await.unwrap() {
        assert_eq!(row.get::<String>(2).unwrap(), "unverified");
        count += 1;
    }
    assert_eq!(count, 3);
}

/// B.3: Record and validate — findings linked to study + hypothesis, update status.
#[tokio::test]
async fn spike_b_record_and_validate() {
    let (_db, conn, session_id) = setup_with_studies().await;

    conn.execute(
        "INSERT INTO research_items (id, session_id, title, status) VALUES (?, ?, ?, ?)",
        libsql::params!["res-study001", session_id.as_str(), "How tokio::spawn works", "in_progress"],
    )
    .await
    .unwrap();

    conn.execute(
        "INSERT INTO studies (id, session_id, research_id, topic, library) VALUES (?, ?, ?, ?, ?)",
        libsql::params!["stu-test0001", session_id.as_str(), "res-study001", "How tokio::spawn works", "tokio"],
    )
    .await
    .unwrap();

    conn.execute(
        "INSERT INTO hypotheses (id, research_id, session_id, content, status) VALUES (?, ?, ?, ?, ?)",
        libsql::params!["hyp-asm001", "res-study001", session_id.as_str(), "spawn requires Send + 'static", "unverified"],
    )
    .await
    .unwrap();

    // Link study → hypothesis
    conn.execute(
        "INSERT INTO entity_links (id, source_type, source_id, target_type, target_id, relation)
         VALUES (?, ?, ?, ?, ?, ?)",
        libsql::params!["lnk-sh001", "study", "stu-test0001", "hypothesis", "hyp-asm001", "relates-to"],
    )
    .await
    .unwrap();

    // Record test result finding
    conn.execute(
        "INSERT INTO findings (id, research_id, session_id, content, source, confidence) VALUES (?, ?, ?, ?, ?, ?)",
        libsql::params![
            "fnd-test001", "res-study001", session_id.as_str(),
            "Test: non-Send type → E0277", "manual:spike-code", "high"
        ],
    )
    .await
    .unwrap();

    // Link study → finding
    conn.execute(
        "INSERT INTO entity_links (id, source_type, source_id, target_type, target_id, relation)
         VALUES (?, ?, ?, ?, ?, ?)",
        libsql::params!["lnk-sf001", "study", "stu-test0001", "finding", "fnd-test001", "relates-to"],
    )
    .await
    .unwrap();

    // Link finding → hypothesis (validates)
    conn.execute(
        "INSERT INTO entity_links (id, source_type, source_id, target_type, target_id, relation)
         VALUES (?, ?, ?, ?, ?, ?)",
        libsql::params!["lnk-fh001", "finding", "fnd-test001", "hypothesis", "hyp-asm001", "validates"],
    )
    .await
    .unwrap();

    // Update hypothesis
    conn.execute(
        "UPDATE hypotheses SET status = 'confirmed', reason = ? WHERE id = ?",
        libsql::params!["E0277 proves Send required", "hyp-asm001"],
    )
    .await
    .unwrap();

    // Verify entity_link chain: study → hypothesis, study → finding, finding → hypothesis
    let mut rows = conn
        .query(
            "SELECT target_type, target_id, relation FROM entity_links
             WHERE source_type = 'study' AND source_id = ?
             ORDER BY target_type, target_id",
            ["stu-test0001"],
        )
        .await
        .unwrap();

    let mut links = Vec::new();
    while let Some(row) = rows.next().await.unwrap() {
        links.push((
            row.get::<String>(0).unwrap(),
            row.get::<String>(1).unwrap(),
            row.get::<String>(2).unwrap(),
        ));
    }
    assert_eq!(links.len(), 2);
    assert!(links.iter().any(|(t, id, _)| t == "finding" && id == "fnd-test001"));
    assert!(links.iter().any(|(t, id, _)| t == "hypothesis" && id == "hyp-asm001"));

    // Verify finding → hypothesis validates link
    let mut rows = conn
        .query(
            "SELECT relation FROM entity_links
             WHERE source_type = 'finding' AND source_id = 'fnd-test001'
               AND target_type = 'hypothesis' AND target_id = 'hyp-asm001'",
            (),
        )
        .await
        .unwrap();
    let row = rows.next().await.unwrap().unwrap();
    assert_eq!(row.get::<String>(0).unwrap(), "validates");
}

/// B.4: Conclude the study — update status, set summary, create insight.
#[tokio::test]
async fn spike_b_conclude_study() {
    let (_db, conn, session_id) = setup_with_studies().await;

    conn.execute(
        "INSERT INTO research_items (id, session_id, title, status) VALUES (?, ?, ?, ?)",
        libsql::params!["res-study001", session_id.as_str(), "How tokio::spawn works", "in_progress"],
    )
    .await
    .unwrap();

    conn.execute(
        "INSERT INTO studies (id, session_id, research_id, topic, library, status) VALUES (?, ?, ?, ?, ?, ?)",
        libsql::params!["stu-test0001", session_id.as_str(), "res-study001", "How tokio::spawn works", "tokio", "active"],
    )
    .await
    .unwrap();

    let summary = "Tokio's spawn is the fundamental task primitive. Requires Send + 'static. \
                   NOT zero-cost (~200B allocation). Panics are caught via JoinHandle.";

    // Update study status + summary
    conn.execute(
        "UPDATE studies SET status = 'completed', summary = ?, updated_at = datetime('now') WHERE id = ?",
        libsql::params![summary, "stu-test0001"],
    )
    .await
    .unwrap();

    // Create insight linked to study
    conn.execute(
        "INSERT INTO insights (id, research_id, session_id, content, confidence) VALUES (?, ?, ?, ?, ?)",
        libsql::params!["ins-conc001", "res-study001", session_id.as_str(), summary, "high"],
    )
    .await
    .unwrap();

    conn.execute(
        "INSERT INTO entity_links (id, source_type, source_id, target_type, target_id, relation)
         VALUES (?, ?, ?, ?, ?, ?)",
        libsql::params!["lnk-si001", "study", "stu-test0001", "insight", "ins-conc001", "relates-to"],
    )
    .await
    .unwrap();

    // Also resolve the research item
    conn.execute(
        "UPDATE research_items SET status = 'resolved' WHERE id = ?",
        ["res-study001"],
    )
    .await
    .unwrap();

    // Verify study status
    let mut rows = conn
        .query("SELECT status, summary FROM studies WHERE id = ?", ["stu-test0001"])
        .await
        .unwrap();
    let row = rows.next().await.unwrap().unwrap();
    assert_eq!(row.get::<String>(0).unwrap(), "completed");
    assert!(row.get::<String>(1).unwrap().contains("Send + 'static"));

    // Verify FTS picks up the summary update
    let mut rows = conn
        .query(
            "SELECT s.id FROM studies_fts fts
             JOIN studies s ON s.rowid = fts.rowid
             WHERE studies_fts MATCH 'Send allocation'",
            (),
        )
        .await
        .unwrap();
    let row = rows.next().await.unwrap().expect("FTS should find updated study");
    assert_eq!(row.get::<String>(0).unwrap(), "stu-test0001");
}

/// B.5: Query full study state with a dedicated join query.
#[tokio::test]
async fn spike_b_query_full_state() {
    let (_db, conn, session_id) = setup_with_studies().await;

    // Set up full scenario
    conn.execute(
        "INSERT INTO research_items (id, session_id, title, status) VALUES (?, ?, ?, ?)",
        libsql::params!["res-study001", session_id.as_str(), "How tokio::spawn works", "in_progress"],
    )
    .await
    .unwrap();

    conn.execute(
        "INSERT INTO studies (id, session_id, research_id, topic, library, methodology, status)
         VALUES (?, ?, ?, ?, ?, ?, ?)",
        libsql::params![
            "stu-test0001", session_id.as_str(), "res-study001",
            "How tokio::spawn works", "tokio", "explore", "active"
        ],
    )
    .await
    .unwrap();

    // Hypotheses linked via entity_links
    for (i, (hyp_id, content, status, reason)) in [
        ("hyp-asm001", "spawn requires Send + 'static", "confirmed", "E0277 proves it"),
        ("hyp-asm002", "panic doesn't crash runtime", "confirmed", "JoinHandle catches it"),
        ("hyp-asm003", "spawn is zero-cost", "debunked", "Allocates ~200 bytes"),
    ]
    .iter()
    .enumerate()
    {
        conn.execute(
            "INSERT INTO hypotheses (id, research_id, session_id, content, status, reason) VALUES (?, ?, ?, ?, ?, ?)",
            libsql::params![*hyp_id, "res-study001", session_id.as_str(), *content, *status, *reason],
        )
        .await
        .unwrap();

        let link_id = format!("lnk-sh{:03}", i);
        conn.execute(
            "INSERT INTO entity_links (id, source_type, source_id, target_type, target_id, relation)
             VALUES (?, ?, ?, ?, ?, ?)",
            libsql::params![link_id.as_str(), "study", "stu-test0001", "hypothesis", *hyp_id, "relates-to"],
        )
        .await
        .unwrap();
    }

    // Findings linked via entity_links
    for (i, (fnd_id, content)) in [
        ("fnd-test001", "Test: non-Send type → E0277"),
        ("fnd-test002", "Test: panic → JoinError"),
        ("fnd-test003", "Test: spawn allocates ~200 bytes"),
    ]
    .iter()
    .enumerate()
    {
        conn.execute(
            "INSERT INTO findings (id, research_id, session_id, content, source, confidence) VALUES (?, ?, ?, ?, ?, ?)",
            libsql::params![*fnd_id, "res-study001", session_id.as_str(), *content, "manual:spike-code", "high"],
        )
        .await
        .unwrap();

        let link_id = format!("lnk-sf{:03}", i);
        conn.execute(
            "INSERT INTO entity_links (id, source_type, source_id, target_type, target_id, relation)
             VALUES (?, ?, ?, ?, ?, ?)",
            libsql::params![link_id.as_str(), "study", "stu-test0001", "finding", *fnd_id, "relates-to"],
        )
        .await
        .unwrap();
    }

    // Insight linked
    conn.execute(
        "INSERT INTO insights (id, research_id, session_id, content, confidence) VALUES (?, ?, ?, ?, ?)",
        libsql::params!["ins-conc001", "res-study001", session_id.as_str(), "Study conclusions...", "high"],
    )
    .await
    .unwrap();

    conn.execute(
        "INSERT INTO entity_links (id, source_type, source_id, target_type, target_id, relation)
         VALUES (?, ?, ?, ?, ?, ?)",
        libsql::params!["lnk-si001", "study", "stu-test0001", "insight", "ins-conc001", "relates-to"],
    )
    .await
    .unwrap();

    // -----------------------------------------------------------------------
    // THE KEY QUERY: Get full study state from the studies table
    // -----------------------------------------------------------------------
    let full_state_sql = "
        SELECT
            s.id, s.topic, s.library, s.methodology, s.status,
            (SELECT json_group_array(json_object(
                'id', h.id, 'content', h.content, 'status', h.status, 'reason', h.reason
            ))
             FROM entity_links el
             JOIN hypotheses h ON h.id = el.target_id
             WHERE el.source_type = 'study' AND el.source_id = s.id
               AND el.target_type = 'hypothesis') as assumptions,
            (SELECT json_group_array(json_object(
                'id', f.id, 'content', f.content, 'confidence', f.confidence
            ))
             FROM entity_links el
             JOIN findings f ON f.id = el.target_id
             WHERE el.source_type = 'study' AND el.source_id = s.id
               AND el.target_type = 'finding') as findings,
            (SELECT json_group_array(json_object(
                'id', i.id, 'content', i.content
            ))
             FROM entity_links el
             JOIN insights i ON i.id = el.target_id
             WHERE el.source_type = 'study' AND el.source_id = s.id
               AND el.target_type = 'insight') as conclusions
        FROM studies s
        WHERE s.id = ?
    ";

    let mut rows = conn.query(full_state_sql, ["stu-test0001"]).await.unwrap();
    let row = rows.next().await.unwrap().expect("expected study state");

    // Verify top-level fields
    assert_eq!(row.get::<String>(0).unwrap(), "stu-test0001");
    assert_eq!(row.get::<String>(1).unwrap(), "How tokio::spawn works");
    assert_eq!(row.get::<String>(2).unwrap(), "tokio");
    assert_eq!(row.get::<String>(3).unwrap(), "explore");
    assert_eq!(row.get::<String>(4).unwrap(), "active");

    // Verify assumptions JSON
    let assumptions_json = row.get::<String>(5).unwrap();
    assert!(assumptions_json.contains("hyp-asm001"));
    assert!(assumptions_json.contains("hyp-asm002"));
    assert!(assumptions_json.contains("hyp-asm003"));

    // Verify findings JSON
    let findings_json = row.get::<String>(6).unwrap();
    assert!(findings_json.contains("fnd-test001"));
    assert!(findings_json.contains("fnd-test003"));

    // Verify conclusions JSON
    let conclusions_json = row.get::<String>(7).unwrap();
    assert!(conclusions_json.contains("ins-conc001"));
}

/// B.6: Progress tracking — count assumptions by status via entity_links.
#[tokio::test]
async fn spike_b_progress_tracking() {
    let (_db, conn, session_id) = setup_with_studies().await;

    conn.execute(
        "INSERT INTO research_items (id, session_id, title, status) VALUES (?, ?, ?, ?)",
        libsql::params!["res-study001", session_id.as_str(), "How tokio::spawn works", "in_progress"],
    )
    .await
    .unwrap();

    conn.execute(
        "INSERT INTO studies (id, session_id, research_id, topic, library) VALUES (?, ?, ?, ?, ?)",
        libsql::params!["stu-test0001", session_id.as_str(), "res-study001", "How tokio::spawn works", "tokio"],
    )
    .await
    .unwrap();

    // 2 confirmed, 1 debunked
    for (i, (hyp_id, status)) in [
        ("hyp-asm001", "confirmed"),
        ("hyp-asm002", "confirmed"),
        ("hyp-asm003", "debunked"),
    ]
    .iter()
    .enumerate()
    {
        conn.execute(
            "INSERT INTO hypotheses (id, research_id, session_id, content, status) VALUES (?, ?, ?, ?, ?)",
            libsql::params![*hyp_id, "res-study001", session_id.as_str(), "assumption content", *status],
        )
        .await
        .unwrap();

        let link_id = format!("lnk-sh{:03}", i);
        conn.execute(
            "INSERT INTO entity_links (id, source_type, source_id, target_type, target_id, relation)
             VALUES (?, ?, ?, ?, ?, ?)",
            libsql::params![link_id.as_str(), "study", "stu-test0001", "hypothesis", *hyp_id, "relates-to"],
        )
        .await
        .unwrap();
    }

    // Progress query via entity_links
    let progress_sql = "
        SELECT
            COUNT(*) as total,
            SUM(CASE WHEN h.status = 'confirmed' THEN 1 ELSE 0 END) as confirmed,
            SUM(CASE WHEN h.status = 'debunked' THEN 1 ELSE 0 END) as debunked,
            SUM(CASE WHEN h.status = 'partially_confirmed' THEN 1 ELSE 0 END) as partial,
            SUM(CASE WHEN h.status = 'unverified' THEN 1 ELSE 0 END) as untested,
            SUM(CASE WHEN h.status = 'analyzing' THEN 1 ELSE 0 END) as in_progress,
            SUM(CASE WHEN h.status = 'inconclusive' THEN 1 ELSE 0 END) as inconclusive
        FROM entity_links el
        JOIN hypotheses h ON h.id = el.target_id
        WHERE el.source_type = 'study' AND el.source_id = ?
          AND el.target_type = 'hypothesis'
    ";

    let mut rows = conn.query(progress_sql, ["stu-test0001"]).await.unwrap();
    let row = rows.next().await.unwrap().expect("expected progress row");

    assert_eq!(row.get::<i64>(0).unwrap(), 3); // total
    assert_eq!(row.get::<i64>(1).unwrap(), 2); // confirmed
    assert_eq!(row.get::<i64>(2).unwrap(), 1); // debunked
    assert_eq!(row.get::<i64>(3).unwrap(), 0); // partial
    assert_eq!(row.get::<i64>(4).unwrap(), 0); // untested
    assert_eq!(row.get::<i64>(5).unwrap(), 0); // in_progress
    assert_eq!(row.get::<i64>(6).unwrap(), 0); // inconclusive
}

// ===========================================================================
// Part C: Comparison (1 test)
// ===========================================================================

/// C.1: Side-by-side comparison of both approaches.
///
/// Prints a comparison table to test output (visible via `cargo test -- --nocapture`).
/// This test always passes — its purpose is to produce data for the design decision.
#[tokio::test]
async fn spike_compare_approaches() {
    // -----------------------------------------------------------------------
    // Metric 1: INSERT count to create a study with 3 assumptions
    // -----------------------------------------------------------------------
    // Approach A: 1 research + 3 hypotheses = 4 INSERTs
    let inserts_a = 4;

    // Approach B: 1 research + 1 study + 3 hypotheses + 3 entity_links = 8 INSERTs
    let inserts_b = 8;

    // -----------------------------------------------------------------------
    // Metric 2: Full state query complexity (approximate lines of SQL)
    // -----------------------------------------------------------------------
    let full_state_a = "
SELECT r.id, r.title, r.description, r.status,
    (SELECT json_group_array(json_object('id', h.id, 'content', h.content, 'status', h.status, 'reason', h.reason))
     FROM hypotheses h WHERE h.research_id = r.id) as assumptions,
    (SELECT json_group_array(json_object('id', f.id, 'content', f.content, 'confidence', f.confidence))
     FROM findings f WHERE f.research_id = r.id) as findings,
    (SELECT json_group_array(json_object('id', i.id, 'content', i.content))
     FROM insights i WHERE i.research_id = r.id) as conclusions
FROM research_items r WHERE r.id = ?";

    let full_state_b = "
SELECT s.id, s.topic, s.library, s.methodology, s.status,
    (SELECT json_group_array(json_object('id', h.id, 'content', h.content, 'status', h.status, 'reason', h.reason))
     FROM entity_links el JOIN hypotheses h ON h.id = el.target_id
     WHERE el.source_type = 'study' AND el.source_id = s.id AND el.target_type = 'hypothesis') as assumptions,
    (SELECT json_group_array(json_object('id', f.id, 'content', f.content, 'confidence', f.confidence))
     FROM entity_links el JOIN findings f ON f.id = el.target_id
     WHERE el.source_type = 'study' AND el.source_id = s.id AND el.target_type = 'finding') as findings,
    (SELECT json_group_array(json_object('id', i.id, 'content', i.content))
     FROM entity_links el JOIN insights i ON i.id = el.target_id
     WHERE el.source_type = 'study' AND el.source_id = s.id AND el.target_type = 'insight') as conclusions
FROM studies s WHERE s.id = ?";

    let lines_a = full_state_a.lines().filter(|l| !l.trim().is_empty()).count();
    let lines_b = full_state_b.lines().filter(|l| !l.trim().is_empty()).count();

    // -----------------------------------------------------------------------
    // Metric 3: Progress query complexity
    // -----------------------------------------------------------------------
    let progress_a = "
SELECT COUNT(*) as total,
    SUM(CASE WHEN status = 'confirmed' THEN 1 ELSE 0 END) as confirmed,
    SUM(CASE WHEN status = 'debunked' THEN 1 ELSE 0 END) as debunked,
    SUM(CASE WHEN status = 'unverified' THEN 1 ELSE 0 END) as untested
FROM hypotheses WHERE research_id = ?";

    let progress_b = "
SELECT COUNT(*) as total,
    SUM(CASE WHEN h.status = 'confirmed' THEN 1 ELSE 0 END) as confirmed,
    SUM(CASE WHEN h.status = 'debunked' THEN 1 ELSE 0 END) as debunked,
    SUM(CASE WHEN h.status = 'unverified' THEN 1 ELSE 0 END) as untested
FROM entity_links el JOIN hypotheses h ON h.id = el.target_id
WHERE el.source_type = 'study' AND el.source_id = ? AND el.target_type = 'hypothesis'";

    let progress_lines_a = progress_a.lines().filter(|l| !l.trim().is_empty()).count();
    let progress_lines_b = progress_b.lines().filter(|l| !l.trim().is_empty()).count();

    // -----------------------------------------------------------------------
    // Metric 4: Filterability — can we distinguish studies from research?
    // -----------------------------------------------------------------------
    let filter_a = "WHERE title LIKE 'Study: %'";  // convention-based, fragile
    let filter_b = "FROM studies WHERE ...";         // type-safe, native

    // -----------------------------------------------------------------------
    // Metric 5: Schema cost
    // -----------------------------------------------------------------------
    let new_tables_a = 0;
    let new_tables_b = 1; // + 1 FTS virtual table + 2 triggers + 4 indexes

    // -----------------------------------------------------------------------
    // Metric 6: Top-level fields available without subquery
    // -----------------------------------------------------------------------
    let top_fields_a = "id, title, description, status"; // no library, no methodology
    let top_fields_b = "id, topic, library, methodology, status, summary"; // purpose-built

    // -----------------------------------------------------------------------
    // Print comparison table
    // -----------------------------------------------------------------------
    println!("\n{}", "=".repeat(72));
    println!("  SPIKE 0.11: STUDIES FEATURE — APPROACH COMPARISON");
    println!("{}\n", "=".repeat(72));
    println!("  {:<35} {:>12} {:>12}", "Metric", "Approach A", "Approach B");
    println!("  {:<35} {:>12} {:>12}", "", "(compose)", "(hybrid)");
    println!("  {:<35} {:>12} {:>12}", "-".repeat(35), "-".repeat(12), "-".repeat(12));
    println!("  {:<35} {:>12} {:>12}", "INSERTs (study + 3 assumptions)", inserts_a, inserts_b);
    println!("  {:<35} {:>12} {:>12}", "Full-state query (SQL lines)", lines_a, lines_b);
    println!("  {:<35} {:>12} {:>12}", "Progress query (SQL lines)", progress_lines_a, progress_lines_b);
    println!("  {:<35} {:>12} {:>12}", "New tables needed", new_tables_a, new_tables_b);
    println!("  {:<35} {:>12} {:>12}", "Filter studies (type-safe?)", "No", "Yes");
    println!("  {:<35} {:>12} {:>12}", "Top-level fields", "4", "6");
    println!();
    println!("  Approach A filter: {filter_a}");
    println!("  Approach B filter: {filter_b}");
    println!();
    println!("  Approach A top fields: {top_fields_a}");
    println!("  Approach B top fields: {top_fields_b}");
    println!();
    println!("  NOTE: Approach A uses research_id FK (direct join),");
    println!("        Approach B uses entity_links (indirect join).");
    println!("        A's subqueries are simpler; B's require JOIN through entity_links.");
    println!();
    println!("  NOTE: Approach B requires 3 extra entity_links INSERTs per assumption");
    println!("        because hypotheses link to study via entity_links, not FK.");
    println!("        Approach A uses the existing research_id FK on hypotheses.");

    // The test passes regardless — it exists to produce data
    assert!(true);
}
