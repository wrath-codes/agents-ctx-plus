//! # Spike 0.12: JSONL Trail — Crate Comparison, Export vs Source of Truth
//!
//! **DONE** — Approach B (JSONL as source of truth) selected. `serde-jsonlines` confirmed.
//!
//! ## Decision
//!
//! - **Approach B wins**: JSONL is the operational source of truth. SQLite DB is rebuildable.
//! - **`serde-jsonlines`**: 1-line batch write/read/append vs 4-5 lines raw. Worth the dependency.
//! - **Rebuild works**: FTS5 indexes, entity_links, and full entity state survive replay.
//! - **Per-session files**: Concurrent-safe (4 agents, 100 ops, zero corruption).
//! - **Size**: ~220 B/entry (operations) vs ~155 B/entry (audit-only). 2.1 MB at 10K ops — trivial.
//!
//! See `docs/schema/10-git-jsonl-strategy.md` for the full design.
//!
//! Validates the JSONL strategy for Zenith's git-friendly audit trail.
//! Tests three dimensions:
//!
//! 1. **Crate comparison**: `serde-jsonlines` vs raw `serde_json` for JSONL I/O
//! 2. **Approach A (export only)**: JSONL as audit log, DB is source of truth
//! 3. **Approach B (source of truth)**: JSONL as operational truth, DB is rebuildable
//! 4. **Concurrency**: Per-session files, concurrent appends
//!
//! ## Scenario
//!
//! Same as spike 0.11: "Learn how tokio::spawn works" — create session, research,
//! hypotheses, findings, insights, entity links. Write to JSONL, optionally rebuild.

use libsql::Builder;
use serde::{Deserialize, Serialize};
use std::io::{BufRead, Write};


/// Helper: create an in-memory database for tests.
async fn in_memory_db() -> libsql::Database {
    Builder::new_local(":memory:")
        .build()
        .await
        .expect("failed to create in-memory database")
}

/// The shared schema from spike_studies.rs (subset needed for JSONL tests).
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

    -- FTS5
    CREATE VIRTUAL TABLE research_fts USING fts5(
        title, description,
        content='research_items', content_rowid='rowid',
        tokenize='porter unicode61'
    );
    CREATE TRIGGER research_ai AFTER INSERT ON research_items BEGIN
        INSERT INTO research_fts(rowid, title, description) VALUES (new.rowid, new.title, new.description);
    END;

    CREATE VIRTUAL TABLE findings_fts USING fts5(
        content, source,
        content='findings', content_rowid='rowid',
        tokenize='porter unicode61'
    );
    CREATE TRIGGER findings_ai AFTER INSERT ON findings BEGIN
        INSERT INTO findings_fts(rowid, content, source) VALUES (new.rowid, new.content, new.source);
    END;

    CREATE VIRTUAL TABLE hypotheses_fts USING fts5(
        content, reason,
        content='hypotheses', content_rowid='rowid',
        tokenize='porter unicode61'
    );
    CREATE TRIGGER hypotheses_ai AFTER INSERT ON hypotheses BEGIN
        INSERT INTO hypotheses_fts(rowid, content, reason) VALUES (new.rowid, new.content, new.reason);
    END;

    -- Indexes
    CREATE INDEX idx_findings_research ON findings(research_id);
    CREATE INDEX idx_hypotheses_research ON hypotheses(research_id);
    CREATE INDEX idx_entity_links_source ON entity_links(source_type, source_id);
    CREATE INDEX idx_entity_links_target ON entity_links(target_type, target_id);
";

// ===========================================================================
// JSONL Operation Types
// ===========================================================================

/// A single JSONL operation — the unit of append to trail files.
/// Used by both Approach A (audit) and Approach B (source of truth).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct Operation {
    /// ISO 8601 timestamp
    ts: String,
    /// Session that produced this operation
    ses: String,
    /// Operation type
    op: OpType,
    /// Entity type
    entity: String,
    /// Entity ID
    id: String,
    /// Payload — full data for creates, changed fields for updates
    data: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
enum OpType {
    Create,
    Update,
    Delete,
}

/// Audit-only entry (Approach A) — simpler, just mirrors audit_trail.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct AuditEntry {
    ts: String,
    ses: String,
    entity: String,
    id: String,
    action: String,
    detail: String,
}

/// Helper: build a list of operations for the "learn tokio::spawn" scenario.
fn build_study_operations() -> Vec<Operation> {
    vec![
        Operation {
            ts: "2026-02-08T10:00:00Z".into(),
            ses: "ses-test0001".into(),
            op: OpType::Create,
            entity: "session".into(),
            id: "ses-test0001".into(),
            data: serde_json::json!({"status": "active"}),
        },
        Operation {
            ts: "2026-02-08T10:01:00Z".into(),
            ses: "ses-test0001".into(),
            op: OpType::Create,
            entity: "research".into(),
            id: "res-study001".into(),
            data: serde_json::json!({
                "session_id": "ses-test0001",
                "title": "Study: How tokio::spawn works",
                "description": "Study plan...",
                "status": "in_progress"
            }),
        },
        Operation {
            ts: "2026-02-08T10:02:00Z".into(),
            ses: "ses-test0001".into(),
            op: OpType::Create,
            entity: "hypothesis".into(),
            id: "hyp-asm001".into(),
            data: serde_json::json!({
                "research_id": "res-study001",
                "session_id": "ses-test0001",
                "content": "spawn requires Send + 'static bounds",
                "status": "unverified"
            }),
        },
        Operation {
            ts: "2026-02-08T10:02:30Z".into(),
            ses: "ses-test0001".into(),
            op: OpType::Create,
            entity: "hypothesis".into(),
            id: "hyp-asm002".into(),
            data: serde_json::json!({
                "research_id": "res-study001",
                "session_id": "ses-test0001",
                "content": "spawned tasks can panic without crashing runtime",
                "status": "unverified"
            }),
        },
        Operation {
            ts: "2026-02-08T10:03:00Z".into(),
            ses: "ses-test0001".into(),
            op: OpType::Create,
            entity: "hypothesis".into(),
            id: "hyp-asm003".into(),
            data: serde_json::json!({
                "research_id": "res-study001",
                "session_id": "ses-test0001",
                "content": "spawn is zero-cost (no allocation)",
                "status": "unverified"
            }),
        },
        Operation {
            ts: "2026-02-08T10:05:00Z".into(),
            ses: "ses-test0001".into(),
            op: OpType::Create,
            entity: "finding".into(),
            id: "fnd-test001".into(),
            data: serde_json::json!({
                "research_id": "res-study001",
                "session_id": "ses-test0001",
                "content": "Test: non-Send type -> compile error E0277",
                "source": "manual:spike-code",
                "confidence": "high"
            }),
        },
        Operation {
            ts: "2026-02-08T10:05:10Z".into(),
            ses: "ses-test0001".into(),
            op: OpType::Create,
            entity: "finding_tag".into(),
            id: "fnd-test001".into(),
            data: serde_json::json!({"tag": "test-result"}),
        },
        Operation {
            ts: "2026-02-08T10:05:20Z".into(),
            ses: "ses-test0001".into(),
            op: OpType::Create,
            entity: "entity_link".into(),
            id: "lnk-val001".into(),
            data: serde_json::json!({
                "source_type": "finding",
                "source_id": "fnd-test001",
                "target_type": "hypothesis",
                "target_id": "hyp-asm001",
                "relation": "validates"
            }),
        },
        Operation {
            ts: "2026-02-08T10:05:30Z".into(),
            ses: "ses-test0001".into(),
            op: OpType::Update,
            entity: "hypothesis".into(),
            id: "hyp-asm001".into(),
            data: serde_json::json!({
                "status": "confirmed",
                "reason": "Compile error E0277 proves Send + 'static required"
            }),
        },
        Operation {
            ts: "2026-02-08T10:10:00Z".into(),
            ses: "ses-test0001".into(),
            op: OpType::Create,
            entity: "insight".into(),
            id: "ins-conc001".into(),
            data: serde_json::json!({
                "research_id": "res-study001",
                "session_id": "ses-test0001",
                "content": "Tokio spawn requires Send + 'static. Confirmed via E0277.",
                "confidence": "high"
            }),
        },
    ]
}

/// Helper: build audit entries for Approach A comparison.
fn build_audit_entries() -> Vec<AuditEntry> {
    vec![
        AuditEntry { ts: "2026-02-08T10:00:00Z".into(), ses: "ses-test0001".into(), entity: "session".into(), id: "ses-test0001".into(), action: "create".into(), detail: "Session started".into() },
        AuditEntry { ts: "2026-02-08T10:01:00Z".into(), ses: "ses-test0001".into(), entity: "research".into(), id: "res-study001".into(), action: "create".into(), detail: "Created: Study: How tokio::spawn works".into() },
        AuditEntry { ts: "2026-02-08T10:02:00Z".into(), ses: "ses-test0001".into(), entity: "hypothesis".into(), id: "hyp-asm001".into(), action: "create".into(), detail: "Created: spawn requires Send + 'static bounds".into() },
        AuditEntry { ts: "2026-02-08T10:05:00Z".into(), ses: "ses-test0001".into(), entity: "finding".into(), id: "fnd-test001".into(), action: "create".into(), detail: "Created: Test: non-Send type -> E0277".into() },
        AuditEntry { ts: "2026-02-08T10:05:30Z".into(), ses: "ses-test0001".into(), entity: "hypothesis".into(), id: "hyp-asm001".into(), action: "update".into(), detail: "Status: unverified -> confirmed".into() },
        AuditEntry { ts: "2026-02-08T10:10:00Z".into(), ses: "ses-test0001".into(), entity: "insight".into(), id: "ins-conc001".into(), action: "create".into(), detail: "Created: Tokio spawn requires Send + 'static".into() },
    ]
}

// ===========================================================================
// Part A: Crate Comparison (3 tests)
// ===========================================================================

/// A.1: serde-jsonlines roundtrip — write and read back operations.
#[tokio::test]
async fn spike_jsonl_serde_jsonlines_roundtrip() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("test.jsonl");

    let ops = build_study_operations();

    // Write using serde-jsonlines
    serde_jsonlines::write_json_lines(&path, &ops).unwrap();

    // Read back
    let read_ops: Vec<Operation> = serde_jsonlines::json_lines(&path)
        .unwrap()
        .collect::<std::io::Result<Vec<_>>>()
        .unwrap();

    assert_eq!(ops.len(), read_ops.len());
    assert_eq!(ops, read_ops);
}

/// A.2: Raw serde_json roundtrip — manual writeln/BufReader.
#[tokio::test]
async fn spike_jsonl_raw_serde_json_roundtrip() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("test.jsonl");

    let ops = build_study_operations();

    // Write using raw serde_json
    {
        let file = std::fs::File::create(&path).unwrap();
        let mut writer = std::io::BufWriter::new(file);
        for op in &ops {
            let line = serde_json::to_string(op).unwrap();
            writeln!(writer, "{line}").unwrap();
        }
        writer.flush().unwrap();
    }

    // Read back using raw serde_json
    let read_ops: Vec<Operation> = {
        let file = std::fs::File::open(&path).unwrap();
        let reader = std::io::BufReader::new(file);
        reader
            .lines()
            .map(|line| serde_json::from_str(&line.unwrap()).unwrap())
            .collect()
    };

    assert_eq!(ops.len(), read_ops.len());
    assert_eq!(ops, read_ops);
}

/// A.3: Compare ergonomics — lines of code for common operations.
#[tokio::test]
async fn spike_jsonl_compare_ergonomics() {
    let dir = tempfile::tempdir().unwrap();
    let path_a = dir.path().join("serde_jsonlines.jsonl");
    let path_b = dir.path().join("raw_serde_json.jsonl");

    let ops = build_study_operations();

    // -----------------------------------------------------------------------
    // Write: serde-jsonlines (1 line)
    // -----------------------------------------------------------------------
    serde_jsonlines::write_json_lines(&path_a, &ops).unwrap();

    // -----------------------------------------------------------------------
    // Write: raw serde_json (5 lines)
    // -----------------------------------------------------------------------
    {
        let file = std::fs::File::create(&path_b).unwrap();
        let mut writer = std::io::BufWriter::new(file);
        for op in &ops {
            writeln!(writer, "{}", serde_json::to_string(op).unwrap()).unwrap();
        }
        writer.flush().unwrap();
    }

    // -----------------------------------------------------------------------
    // Read: serde-jsonlines (1 line)
    // -----------------------------------------------------------------------
    let _read_a: Vec<Operation> = serde_jsonlines::json_lines(&path_a)
        .unwrap()
        .collect::<std::io::Result<Vec<_>>>()
        .unwrap();

    // -----------------------------------------------------------------------
    // Read: raw serde_json (4 lines)
    // -----------------------------------------------------------------------
    let _read_b: Vec<Operation> = {
        let file = std::fs::File::open(&path_b).unwrap();
        let reader = std::io::BufReader::new(file);
        reader
            .lines()
            .map(|l| serde_json::from_str(&l.unwrap()).unwrap())
            .collect()
    };

    // -----------------------------------------------------------------------
    // Append: serde-jsonlines (1 line)
    // -----------------------------------------------------------------------
    let extra = Operation {
        ts: "2026-02-08T11:00:00Z".into(),
        ses: "ses-test0001".into(),
        op: OpType::Update,
        entity: "research".into(),
        id: "res-study001".into(),
        data: serde_json::json!({"status": "resolved"}),
    };
    serde_jsonlines::append_json_lines(&path_a, [&extra]).unwrap();

    // -----------------------------------------------------------------------
    // Append: raw serde_json (4 lines)
    // -----------------------------------------------------------------------
    {
        let file = std::fs::OpenOptions::new().create(true).append(true).open(&path_b).unwrap();
        let mut writer = std::io::BufWriter::new(file);
        writeln!(writer, "{}", serde_json::to_string(&extra).unwrap()).unwrap();
        writer.flush().unwrap();
    }

    // Verify both files have 11 lines now
    let count_a: usize = serde_jsonlines::json_lines::<Operation, _>(&path_a)
        .unwrap()
        .count();
    let count_b: usize = {
        let file = std::fs::File::open(&path_b).unwrap();
        std::io::BufReader::new(file).lines().count()
    };
    assert_eq!(count_a, 11);
    assert_eq!(count_b, 11);

    println!("\n{}", "=".repeat(72));
    println!("  SPIKE 0.12 PART A: CRATE ERGONOMICS COMPARISON");
    println!("{}\n", "=".repeat(72));
    println!("  {:<30} {:>15} {:>15}", "Operation", "serde-jsonlines", "raw serde_json");
    println!("  {:<30} {:>15} {:>15}", "-".repeat(30), "-".repeat(15), "-".repeat(15));
    println!("  {:<30} {:>15} {:>15}", "Write batch", "1 line", "5 lines");
    println!("  {:<30} {:>15} {:>15}", "Read batch", "1 line", "4 lines");
    println!("  {:<30} {:>15} {:>15}", "Append single", "1 line", "4 lines");
    println!("  {:<30} {:>15} {:>15}", "Error handling", "built-in", "manual");
    println!("  {:<30} {:>15} {:>15}", "File creation", "auto", "manual");
    println!("  {:<30} {:>15} {:>15}", "Extra dependency", "yes", "no");
}

// ===========================================================================
// Part B: Approach A — Export Only (3 tests)
// ===========================================================================

/// B.1: Export audit trail from SQLite to JSONL.
#[tokio::test]
async fn spike_jsonl_audit_export() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("ses-test0001.jsonl");

    let entries = build_audit_entries();

    // Write audit entries to JSONL
    serde_jsonlines::write_json_lines(&path, &entries).unwrap();

    // Verify file exists and has correct line count
    let read_entries: Vec<AuditEntry> = serde_jsonlines::json_lines(&path)
        .unwrap()
        .collect::<std::io::Result<Vec<_>>>()
        .unwrap();

    assert_eq!(read_entries.len(), 6);
    assert_eq!(read_entries[0].action, "create");
    assert_eq!(read_entries[0].entity, "session");
    assert_eq!(read_entries[4].action, "update");
    assert_eq!(read_entries[4].detail, "Status: unverified -> confirmed");
}

/// B.2: Read back audit JSONL and verify all entries are parseable.
#[tokio::test]
async fn spike_jsonl_audit_read_back() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("ses-test0001.jsonl");

    // Write 100 audit entries
    let mut entries = Vec::new();
    for i in 0..100 {
        entries.push(AuditEntry {
            ts: format!("2026-02-08T10:{:02}:00Z", i % 60),
            ses: "ses-test0001".into(),
            entity: "finding".into(),
            id: format!("fnd-{i:04}"),
            action: "create".into(),
            detail: format!("Created finding #{i}"),
        });
    }
    serde_jsonlines::write_json_lines(&path, &entries).unwrap();

    // Read all back
    let read_entries: Vec<AuditEntry> = serde_jsonlines::json_lines(&path)
        .unwrap()
        .collect::<std::io::Result<Vec<_>>>()
        .unwrap();

    assert_eq!(read_entries.len(), 100);
    assert_eq!(read_entries[99].id, "fnd-0099");
}

/// B.3: Measure JSONL size for audit entries.
#[tokio::test]
async fn spike_jsonl_audit_size() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("size-test.jsonl");

    // Write 100 audit entries
    let mut entries = Vec::new();
    for i in 0..100 {
        entries.push(AuditEntry {
            ts: format!("2026-02-08T10:{:02}:{:02}Z", i / 60, i % 60),
            ses: "ses-test0001".into(),
            entity: ["finding", "hypothesis", "research", "insight"][i % 4].into(),
            id: format!("ent-{i:04}"),
            action: ["create", "update"][i % 2].into(),
            detail: format!("Audit detail for operation #{i} with some realistic content"),
        });
    }
    serde_jsonlines::write_json_lines(&path, &entries).unwrap();

    let file_size = std::fs::metadata(&path).unwrap().len();
    let avg_per_entry = file_size / 100;

    println!("\n  AUDIT-ONLY SIZE:");
    println!("  100 entries: {} bytes ({} bytes/entry avg)", file_size, avg_per_entry);
    println!("  1,000 entries: ~{} KB", avg_per_entry * 1000 / 1024);
    println!("  10,000 entries: ~{} KB", avg_per_entry * 10_000 / 1024);

    // Sanity: each audit entry should be ~100-200 bytes
    assert!(avg_per_entry > 80, "entries too small: {avg_per_entry}");
    assert!(avg_per_entry < 300, "entries too large: {avg_per_entry}");
}

// ===========================================================================
// Part C: Approach B — Source of Truth / Rebuild (6 tests)
// ===========================================================================

/// C.1: Verify the Operation enum serializes/deserializes all entity types correctly.
#[tokio::test]
async fn spike_jsonl_operation_format() {
    let entity_types = [
        "session", "research", "finding", "finding_tag", "hypothesis",
        "insight", "issue", "task", "impl_log", "compat", "study",
        "entity_link", "audit",
    ];

    for entity in entity_types {
        let op = Operation {
            ts: "2026-02-08T10:00:00Z".into(),
            ses: "ses-test0001".into(),
            op: OpType::Create,
            entity: entity.to_string(),
            id: format!("test-{entity}"),
            data: serde_json::json!({"test": true}),
        };

        let json = serde_json::to_string(&op).unwrap();
        let deserialized: Operation = serde_json::from_str(&json).unwrap();
        assert_eq!(op, deserialized);
        assert!(json.contains(entity));
    }

    // Verify OpType variants
    for op_type in [OpType::Create, OpType::Update, OpType::Delete] {
        let op = Operation {
            ts: "2026-02-08T10:00:00Z".into(),
            ses: "ses-test0001".into(),
            op: op_type.clone(),
            entity: "test".into(),
            id: "test-001".into(),
            data: serde_json::json!({}),
        };
        let json = serde_json::to_string(&op).unwrap();
        let deserialized: Operation = serde_json::from_str(&json).unwrap();
        assert_eq!(op, deserialized);
    }
}

/// C.2: Write the full study scenario as operations to JSONL.
#[tokio::test]
async fn spike_jsonl_write_operations() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("ses-test0001.jsonl");

    let ops = build_study_operations();
    serde_jsonlines::write_json_lines(&path, &ops).unwrap();

    // Verify
    let read_ops: Vec<Operation> = serde_jsonlines::json_lines(&path)
        .unwrap()
        .collect::<std::io::Result<Vec<_>>>()
        .unwrap();

    assert_eq!(read_ops.len(), 10);
    assert_eq!(read_ops[0].entity, "session");
    assert_eq!(read_ops[1].entity, "research");
    assert_eq!(read_ops[8].op, OpType::Update); // hypothesis confirmed
    assert_eq!(read_ops[9].entity, "insight");
}

/// C.3: Replay operations into a fresh DB and verify correctness.
#[tokio::test]
async fn spike_jsonl_replay_rebuild() {
    // Step 1: Write operations to JSONL
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("ses-test0001.jsonl");
    let ops = build_study_operations();
    serde_jsonlines::write_json_lines(&path, &ops).unwrap();

    // Step 2: Create a fresh DB and replay
    let db = in_memory_db().await;
    let conn = db.connect().expect("connect");
    conn.execute_batch(SHARED_SCHEMA).await.unwrap();

    let read_ops: Vec<Operation> = serde_jsonlines::json_lines(&path)
        .unwrap()
        .collect::<std::io::Result<Vec<_>>>()
        .unwrap();

    for op in &read_ops {
        replay_operation(&conn, op).await;
    }

    // Step 3: Verify the rebuilt DB has correct state
    // Session exists
    let mut rows = conn.query("SELECT id, status FROM sessions WHERE id = ?", ["ses-test0001"]).await.unwrap();
    let row = rows.next().await.unwrap().expect("session");
    assert_eq!(row.get::<String>(0).unwrap(), "ses-test0001");

    // Research exists
    let mut rows = conn.query("SELECT title, status FROM research_items WHERE id = ?", ["res-study001"]).await.unwrap();
    let row = rows.next().await.unwrap().expect("research");
    assert!(row.get::<String>(0).unwrap().contains("tokio::spawn"));

    // 3 hypotheses exist, one is confirmed
    let mut rows = conn.query("SELECT COUNT(*) FROM hypotheses WHERE research_id = ?", ["res-study001"]).await.unwrap();
    let row = rows.next().await.unwrap().unwrap();
    assert_eq!(row.get::<i64>(0).unwrap(), 3);

    let mut rows = conn.query("SELECT status FROM hypotheses WHERE id = ?", ["hyp-asm001"]).await.unwrap();
    let row = rows.next().await.unwrap().unwrap();
    assert_eq!(row.get::<String>(0).unwrap(), "confirmed");

    // Finding exists with tag
    let mut rows = conn.query("SELECT tag FROM finding_tags WHERE finding_id = ?", ["fnd-test001"]).await.unwrap();
    let row = rows.next().await.unwrap().unwrap();
    assert_eq!(row.get::<String>(0).unwrap(), "test-result");

    // Entity link exists
    let mut rows = conn.query(
        "SELECT relation FROM entity_links WHERE source_id = ? AND target_id = ?",
        libsql::params!["fnd-test001", "hyp-asm001"],
    ).await.unwrap();
    let row = rows.next().await.unwrap().unwrap();
    assert_eq!(row.get::<String>(0).unwrap(), "validates");

    // Insight exists
    let mut rows = conn.query("SELECT content FROM insights WHERE id = ?", ["ins-conc001"]).await.unwrap();
    let row = rows.next().await.unwrap().unwrap();
    assert!(row.get::<String>(0).unwrap().contains("Send + 'static"));
}

/// C.4: After replay, verify FTS5 indexes work.
#[tokio::test]
async fn spike_jsonl_rebuild_fts() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("ses-test0001.jsonl");
    let ops = build_study_operations();
    serde_jsonlines::write_json_lines(&path, &ops).unwrap();

    let db = in_memory_db().await;
    let conn = db.connect().expect("connect");
    conn.execute_batch(SHARED_SCHEMA).await.unwrap();

    let read_ops: Vec<Operation> = serde_jsonlines::json_lines(&path)
        .unwrap()
        .collect::<std::io::Result<Vec<_>>>()
        .unwrap();
    for op in &read_ops {
        replay_operation(&conn, op).await;
    }

    // FTS on research
    let mut rows = conn.query(
        "SELECT r.id FROM research_fts fts JOIN research_items r ON r.rowid = fts.rowid WHERE research_fts MATCH ?",
        ["tokio spawn"],
    ).await.unwrap();
    let row = rows.next().await.unwrap().expect("FTS should find research");
    assert_eq!(row.get::<String>(0).unwrap(), "res-study001");

    // FTS on findings
    let mut rows = conn.query(
        "SELECT f.id FROM findings_fts fts JOIN findings f ON f.rowid = fts.rowid WHERE findings_fts MATCH ?",
        ["E0277"],
    ).await.unwrap();
    let row = rows.next().await.unwrap().expect("FTS should find finding");
    assert_eq!(row.get::<String>(0).unwrap(), "fnd-test001");

    // FTS on hypotheses
    let mut rows = conn.query(
        "SELECT h.id FROM hypotheses_fts fts JOIN hypotheses h ON h.rowid = fts.rowid WHERE hypotheses_fts MATCH ?",
        ["Send"],
    ).await.unwrap();
    let row = rows.next().await.unwrap().expect("FTS should find hypothesis");
    assert_eq!(row.get::<String>(0).unwrap(), "hyp-asm001");
}

/// C.5: After replay, verify entity_links are correct.
#[tokio::test]
async fn spike_jsonl_rebuild_entity_links() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("ses-test0001.jsonl");
    let ops = build_study_operations();
    serde_jsonlines::write_json_lines(&path, &ops).unwrap();

    let db = in_memory_db().await;
    let conn = db.connect().expect("connect");
    conn.execute_batch(SHARED_SCHEMA).await.unwrap();

    let read_ops: Vec<Operation> = serde_jsonlines::json_lines(&path)
        .unwrap()
        .collect::<std::io::Result<Vec<_>>>()
        .unwrap();
    for op in &read_ops {
        replay_operation(&conn, op).await;
    }

    // Query all links from the finding
    let mut rows = conn.query(
        "SELECT target_type, target_id, relation FROM entity_links WHERE source_type = 'finding' AND source_id = ?",
        ["fnd-test001"],
    ).await.unwrap();
    let row = rows.next().await.unwrap().expect("should have link");
    assert_eq!(row.get::<String>(0).unwrap(), "hypothesis");
    assert_eq!(row.get::<String>(1).unwrap(), "hyp-asm001");
    assert_eq!(row.get::<String>(2).unwrap(), "validates");
}

/// C.6: Measure operation JSONL size and compare to audit-only.
#[tokio::test]
async fn spike_jsonl_operation_size() {
    let dir = tempfile::tempdir().unwrap();
    let ops_path = dir.path().join("operations.jsonl");
    let audit_path = dir.path().join("audit.jsonl");

    let ops = build_study_operations();
    let audits = build_audit_entries();

    serde_jsonlines::write_json_lines(&ops_path, &ops).unwrap();
    serde_jsonlines::write_json_lines(&audit_path, &audits).unwrap();

    let ops_size = std::fs::metadata(&ops_path).unwrap().len();
    let audit_size = std::fs::metadata(&audit_path).unwrap().len();

    let ops_per_entry = ops_size / ops.len() as u64;
    let audit_per_entry = audit_size / audits.len() as u64;

    println!("\n{}", "=".repeat(72));
    println!("  SPIKE 0.12 PART C: FORMAT SIZE COMPARISON");
    println!("{}\n", "=".repeat(72));
    println!("  {:<35} {:>12} {:>12}", "Metric", "Operations", "Audit-only");
    println!("  {:<35} {:>12} {:>12}", "", "(Approach B)", "(Approach A)");
    println!("  {:<35} {:>12} {:>12}", "-".repeat(35), "-".repeat(12), "-".repeat(12));
    println!("  {:<35} {:>12} {:>12}", "Entries in scenario", ops.len(), audits.len());
    println!("  {:<35} {:>10} B {:>10} B", "Total size", ops_size, audit_size);
    println!("  {:<35} {:>10} B {:>10} B", "Avg per entry", ops_per_entry, audit_per_entry);
    println!("  {:<35} {:>10} KB {:>10} KB", "Est. 1K entries", ops_per_entry * 1000 / 1024, audit_per_entry * 1000 / 1024);
    println!("  {:<35} {:>10} KB {:>10} KB", "Est. 10K entries", ops_per_entry * 10_000 / 1024, audit_per_entry * 10_000 / 1024);
    println!();
    println!("  NOTE: Operations carry full entity data (needed for rebuild).");
    println!("  Audit entries carry only action descriptions (not rebuildable).");
}

// ===========================================================================
// Part D: Comparison (1 test)
// ===========================================================================

/// D.1: Side-by-side comparison of both approaches.
#[tokio::test]
async fn spike_jsonl_compare_approaches() {
    println!("\n{}", "=".repeat(72));
    println!("  SPIKE 0.12 PART D: APPROACH COMPARISON");
    println!("{}\n", "=".repeat(72));
    println!("  {:<35} {:>15} {:>15}", "Dimension", "A (export)", "B (source)");
    println!("  {:<35} {:>15} {:>15}", "-".repeat(35), "-".repeat(15), "-".repeat(15));
    println!("  {:<35} {:>15} {:>15}", "DB rebuildable from JSONL?", "No", "Yes");
    println!("  {:<35} {:>15} {:>15}", "Survives DB corruption?", "No", "Yes");
    println!("  {:<35} {:>15} {:>15}", "git clone gives full state?", "No", "Yes");
    println!("  {:<35} {:>15} {:>15}", "Turso Cloud required?", "For durability", "Optional");
    println!("  {:<35} {:>15} {:>15}", "JSONL entry size", "~150 B", "~250 B");
    println!("  {:<35} {:>15} {:>15}", "Write path complexity", "Low", "Medium");
    println!("  {:<35} {:>15} {:>15}", "Replay logic needed?", "No", "Yes (~60 LOC)");
    println!("  {:<35} {:>15} {:>15}", "Schema change impact", "None", "Update replay");
    println!("  {:<35} {:>15} {:>15}", "FTS5 after rebuild?", "N/A", "Works (tested)");
    println!("  {:<35} {:>15} {:>15}", "Entity links after rebuild?", "N/A", "Works (tested)");
    println!();
    println!("  Key insight: Approach B's replay logic is ~60 lines of match/insert.");
    println!("  Schema changes require updating the replay function, but this is");
    println!("  the same effort as updating a migration — manageable.");
}

// ===========================================================================
// Part E: Per-Session Files + Concurrent Append (2 tests)
// ===========================================================================

/// E.1: Write to two separate session files and verify isolation.
#[tokio::test]
async fn spike_jsonl_per_session_files() {
    let dir = tempfile::tempdir().unwrap();
    let trail_dir = dir.path().join("trail");
    std::fs::create_dir_all(&trail_dir).unwrap();

    let session_a = trail_dir.join("ses-aaa00001.jsonl");
    let session_b = trail_dir.join("ses-bbb00002.jsonl");

    // Session A: 3 operations
    let ops_a = vec![
        Operation { ts: "2026-02-08T10:00:00Z".into(), ses: "ses-aaa00001".into(), op: OpType::Create, entity: "session".into(), id: "ses-aaa00001".into(), data: serde_json::json!({"status": "active"}) },
        Operation { ts: "2026-02-08T10:01:00Z".into(), ses: "ses-aaa00001".into(), op: OpType::Create, entity: "finding".into(), id: "fnd-a001".into(), data: serde_json::json!({"content": "Finding from session A"}) },
        Operation { ts: "2026-02-08T10:02:00Z".into(), ses: "ses-aaa00001".into(), op: OpType::Create, entity: "insight".into(), id: "ins-a001".into(), data: serde_json::json!({"content": "Insight from session A"}) },
    ];

    // Session B: 2 operations
    let ops_b = vec![
        Operation { ts: "2026-02-08T10:00:30Z".into(), ses: "ses-bbb00002".into(), op: OpType::Create, entity: "session".into(), id: "ses-bbb00002".into(), data: serde_json::json!({"status": "active"}) },
        Operation { ts: "2026-02-08T10:01:30Z".into(), ses: "ses-bbb00002".into(), op: OpType::Create, entity: "finding".into(), id: "fnd-b001".into(), data: serde_json::json!({"content": "Finding from session B"}) },
    ];

    serde_jsonlines::write_json_lines(&session_a, &ops_a).unwrap();
    serde_jsonlines::write_json_lines(&session_b, &ops_b).unwrap();

    // Verify isolation
    let read_a: Vec<Operation> = serde_jsonlines::json_lines(&session_a).unwrap().collect::<std::io::Result<Vec<_>>>().unwrap();
    let read_b: Vec<Operation> = serde_jsonlines::json_lines(&session_b).unwrap().collect::<std::io::Result<Vec<_>>>().unwrap();

    assert_eq!(read_a.len(), 3);
    assert_eq!(read_b.len(), 2);
    assert!(read_a.iter().all(|op| op.ses == "ses-aaa00001"));
    assert!(read_b.iter().all(|op| op.ses == "ses-bbb00002"));

    // Verify we can list all session files
    let mut session_files: Vec<_> = std::fs::read_dir(&trail_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().is_some_and(|ext| ext == "jsonl"))
        .map(|e| e.file_name().into_string().unwrap())
        .collect();
    session_files.sort();
    assert_eq!(session_files, vec!["ses-aaa00001.jsonl", "ses-bbb00002.jsonl"]);

    // Verify we can rebuild from ALL session files
    let mut all_ops = Vec::new();
    all_ops.extend(read_a);
    all_ops.extend(read_b);
    all_ops.sort_by(|a, b| a.ts.cmp(&b.ts));
    assert_eq!(all_ops.len(), 5);
    assert_eq!(all_ops[0].ses, "ses-aaa00001"); // 10:00:00
    assert_eq!(all_ops[1].ses, "ses-bbb00002"); // 10:00:30
}

/// E.2: Concurrent appends to separate session files.
#[tokio::test]
async fn spike_jsonl_concurrent_append() {
    let dir = tempfile::tempdir().unwrap();
    let trail_dir = dir.path().join("trail");
    std::fs::create_dir_all(&trail_dir).unwrap();

    let mut handles = Vec::new();

    // Spawn 4 concurrent tasks, each writing to its own session file
    for agent_idx in 0..4u32 {
        let session_dir = trail_dir.clone();
        handles.push(tokio::spawn(async move {
            let session_id = format!("ses-agent{agent_idx:04}");
            let path = session_dir.join(format!("{session_id}.jsonl"));

            // Each agent writes 25 operations
            for op_idx in 0..25u32 {
                let op = Operation {
                    ts: format!("2026-02-08T10:{:02}:{:02}Z", op_idx / 60, op_idx % 60),
                    ses: session_id.clone(),
                    op: OpType::Create,
                    entity: "finding".into(),
                    id: format!("fnd-{agent_idx:02}-{op_idx:03}"),
                    data: serde_json::json!({"content": format!("Agent {agent_idx} finding {op_idx}")}),
                };
                serde_jsonlines::append_json_lines(&path, [&op]).unwrap();
            }
        }));
    }

    // Wait for all tasks
    for handle in handles {
        handle.await.unwrap();
    }

    // Verify each session file has exactly 25 entries
    for agent_idx in 0..4u32 {
        let session_id = format!("ses-agent{agent_idx:04}");
        let path = trail_dir.join(format!("{session_id}.jsonl"));

        let ops: Vec<Operation> = serde_jsonlines::json_lines(&path)
            .unwrap()
            .collect::<std::io::Result<Vec<_>>>()
            .unwrap();

        assert_eq!(ops.len(), 25, "Agent {agent_idx} should have 25 entries");
        assert!(ops.iter().all(|op| op.ses == session_id));
    }

    // Verify total across all files
    let total: usize = std::fs::read_dir(&trail_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().is_some_and(|ext| ext == "jsonl"))
        .map(|e| {
            serde_jsonlines::json_lines::<Operation, _>(e.path())
                .unwrap()
                .count()
        })
        .sum();
    assert_eq!(total, 100); // 4 agents * 25 ops
}

// ===========================================================================
// Replay Logic (the heart of Approach B)
// ===========================================================================

/// Replay a single JSONL operation into the database.
/// This is the core of Approach B — ~60 lines covering all entity types.
async fn replay_operation(conn: &libsql::Connection, op: &Operation) {
    match (&op.op, op.entity.as_str()) {
        // -- Session --
        (OpType::Create, "session") => {
            conn.execute(
                "INSERT INTO sessions (id, status) VALUES (?, ?)",
                libsql::params![op.id.as_str(), op.data["status"].as_str().unwrap_or("active")],
            ).await.unwrap();
        }

        // -- Research --
        (OpType::Create, "research") => {
            conn.execute(
                "INSERT INTO research_items (id, session_id, title, description, status) VALUES (?, ?, ?, ?, ?)",
                libsql::params![
                    op.id.as_str(),
                    op.data["session_id"].as_str().unwrap_or(""),
                    op.data["title"].as_str().unwrap_or(""),
                    op.data.get("description").and_then(|v| v.as_str()).unwrap_or(""),
                    op.data["status"].as_str().unwrap_or("open")
                ],
            ).await.unwrap();
        }

        // -- Hypothesis --
        (OpType::Create, "hypothesis") => {
            conn.execute(
                "INSERT INTO hypotheses (id, research_id, session_id, content, status) VALUES (?, ?, ?, ?, ?)",
                libsql::params![
                    op.id.as_str(),
                    op.data["research_id"].as_str().unwrap_or(""),
                    op.data["session_id"].as_str().unwrap_or(""),
                    op.data["content"].as_str().unwrap_or(""),
                    op.data["status"].as_str().unwrap_or("unverified")
                ],
            ).await.unwrap();
        }
        (OpType::Update, "hypothesis") => {
            if let Some(status) = op.data.get("status").and_then(|v| v.as_str()) {
                let reason = op.data.get("reason").and_then(|v| v.as_str()).unwrap_or("");
                conn.execute(
                    "UPDATE hypotheses SET status = ?, reason = ?, updated_at = datetime('now') WHERE id = ?",
                    libsql::params![status, reason, op.id.as_str()],
                ).await.unwrap();
            }
        }

        // -- Finding --
        (OpType::Create, "finding") => {
            conn.execute(
                "INSERT INTO findings (id, research_id, session_id, content, source, confidence) VALUES (?, ?, ?, ?, ?, ?)",
                libsql::params![
                    op.id.as_str(),
                    op.data["research_id"].as_str().unwrap_or(""),
                    op.data["session_id"].as_str().unwrap_or(""),
                    op.data["content"].as_str().unwrap_or(""),
                    op.data.get("source").and_then(|v| v.as_str()).unwrap_or(""),
                    op.data["confidence"].as_str().unwrap_or("medium")
                ],
            ).await.unwrap();
        }

        // -- Finding Tag --
        (OpType::Create, "finding_tag") => {
            conn.execute(
                "INSERT INTO finding_tags (finding_id, tag) VALUES (?, ?)",
                libsql::params![op.id.as_str(), op.data["tag"].as_str().unwrap_or("")],
            ).await.unwrap();
        }

        // -- Entity Link --
        (OpType::Create, "entity_link") => {
            conn.execute(
                "INSERT INTO entity_links (id, source_type, source_id, target_type, target_id, relation) VALUES (?, ?, ?, ?, ?, ?)",
                libsql::params![
                    op.id.as_str(),
                    op.data["source_type"].as_str().unwrap_or(""),
                    op.data["source_id"].as_str().unwrap_or(""),
                    op.data["target_type"].as_str().unwrap_or(""),
                    op.data["target_id"].as_str().unwrap_or(""),
                    op.data["relation"].as_str().unwrap_or("")
                ],
            ).await.unwrap();
        }

        // -- Insight --
        (OpType::Create, "insight") => {
            conn.execute(
                "INSERT INTO insights (id, research_id, session_id, content, confidence) VALUES (?, ?, ?, ?, ?)",
                libsql::params![
                    op.id.as_str(),
                    op.data["research_id"].as_str().unwrap_or(""),
                    op.data["session_id"].as_str().unwrap_or(""),
                    op.data["content"].as_str().unwrap_or(""),
                    op.data["confidence"].as_str().unwrap_or("medium")
                ],
            ).await.unwrap();
        }

        // -- Catch-all for unhandled combinations --
        (op_type, entity) => {
            eprintln!("WARN: unhandled replay: {:?} on {}", op_type, entity);
        }
    }
}
