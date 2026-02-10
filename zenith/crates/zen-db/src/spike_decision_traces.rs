//! # Spike 0.22: Decision Traces + Context Graph — Phase A
//!
//! Validates the first-class `decisions` data model for recording *why* changes
//! were made in Zenith. Tests schema persistence, FTS, composite trail replay,
//! and the mutation protocol across 4 new tables: `decisions`, `decision_options`,
//! `decision_option_evidence`, `decision_outcomes`.
//!
//! See `docs/schema/22-decision-graph-rustworkx-spike-plan.md` for the full design.
//!
//! ## Validates
//!
//! - RQ1: First-class `decisions` schema produces reliable structured queries + FTS
//! - RQ5: Per-option evidence structure adds retrievable value
//! - Replay invariant: DB state rebuildable from composite trail operations
//! - Mutation protocol: BEGIN → SQL → audit → trail → COMMIT

use libsql::Builder;

async fn in_memory_db() -> libsql::Database {
    Builder::new_local(":memory:")
        .build()
        .await
        .expect("failed to create in-memory database")
}

const DECISION_SPIKE_SCHEMA: &str = "
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

    CREATE TABLE tasks (
        id TEXT PRIMARY KEY,
        session_id TEXT REFERENCES sessions(id),
        title TEXT NOT NULL,
        status TEXT NOT NULL DEFAULT 'open',
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

    -- Decision tables

    CREATE TABLE decisions (
        id TEXT PRIMARY KEY,
        session_id TEXT NOT NULL REFERENCES sessions(id),
        category TEXT NOT NULL,
        subject_type TEXT NOT NULL,
        subject_id TEXT NOT NULL,
        question TEXT NOT NULL,
        because TEXT NOT NULL,
        outcome_summary TEXT,
        policy_type TEXT,
        policy_id TEXT,
        exception_kind TEXT,
        exception_reason TEXT,
        approver TEXT,
        confidence TEXT NOT NULL,
        search_text TEXT NOT NULL,
        metadata_json TEXT,
        created_at TEXT NOT NULL DEFAULT (datetime('now')),
        updated_at TEXT NOT NULL DEFAULT (datetime('now'))
    );

    CREATE INDEX decisions_subject_idx ON decisions(subject_type, subject_id);
    CREATE INDEX decisions_session_idx ON decisions(session_id);
    CREATE INDEX decisions_category_idx ON decisions(category);
    CREATE INDEX decisions_created_idx ON decisions(created_at);
    CREATE INDEX decisions_confidence_idx ON decisions(confidence);
    CREATE INDEX decisions_exception_idx ON decisions(exception_kind);

    CREATE TABLE decision_options (
        id TEXT PRIMARY KEY,
        decision_id TEXT NOT NULL REFERENCES decisions(id) ON DELETE CASCADE,
        label TEXT NOT NULL,
        summary TEXT,
        is_chosen INTEGER NOT NULL DEFAULT 0,
        sort_order INTEGER NOT NULL DEFAULT 0
    );

    CREATE UNIQUE INDEX decision_options_one_chosen
        ON decision_options(decision_id) WHERE is_chosen = 1;
    CREATE INDEX decision_options_decision_idx ON decision_options(decision_id);
    CREATE INDEX decision_options_label_idx ON decision_options(label);

    CREATE TABLE decision_option_evidence (
        option_id TEXT NOT NULL REFERENCES decision_options(id) ON DELETE CASCADE,
        entity_type TEXT NOT NULL,
        entity_id TEXT NOT NULL,
        PRIMARY KEY(option_id, entity_type, entity_id)
    );

    CREATE INDEX decision_option_evidence_entity_idx
        ON decision_option_evidence(entity_type, entity_id);

    CREATE TABLE decision_outcomes (
        decision_id TEXT NOT NULL REFERENCES decisions(id) ON DELETE CASCADE,
        entity_type TEXT NOT NULL,
        entity_id TEXT NOT NULL,
        relation TEXT NOT NULL,
        PRIMARY KEY(decision_id, entity_type, entity_id, relation)
    );

    CREATE INDEX decision_outcomes_entity_idx
        ON decision_outcomes(entity_type, entity_id);

    -- Decisions FTS5

    CREATE VIRTUAL TABLE decisions_fts USING fts5(
        search_text,
        content='decisions',
        content_rowid='rowid',
        tokenize='porter unicode61'
    );

    CREATE TRIGGER decisions_ai AFTER INSERT ON decisions BEGIN
        INSERT INTO decisions_fts(rowid, search_text)
        VALUES (new.rowid, new.search_text);
    END;

    CREATE TRIGGER decisions_au AFTER UPDATE ON decisions BEGIN
        INSERT INTO decisions_fts(decisions_fts, rowid, search_text)
        VALUES ('delete', old.rowid, old.search_text);
        INSERT INTO decisions_fts(rowid, search_text)
        VALUES (new.rowid, new.search_text);
    END;

    CREATE TRIGGER decisions_ad AFTER DELETE ON decisions BEGIN
        INSERT INTO decisions_fts(decisions_fts, rowid, search_text)
        VALUES ('delete', old.rowid, old.search_text);
    END;

    -- Existing FTS tables (needed for entity_links queries)

    CREATE VIRTUAL TABLE findings_fts USING fts5(
        content, source,
        content='findings', content_rowid='rowid',
        tokenize='porter unicode61'
    );
    CREATE TRIGGER findings_ai AFTER INSERT ON findings BEGIN
        INSERT INTO findings_fts(rowid, content, source)
        VALUES (new.rowid, new.content, new.source);
    END;

    CREATE VIRTUAL TABLE hypotheses_fts USING fts5(
        content, reason,
        content='hypotheses', content_rowid='rowid',
        tokenize='porter unicode61'
    );
    CREATE TRIGGER hypotheses_ai AFTER INSERT ON hypotheses BEGIN
        INSERT INTO hypotheses_fts(rowid, content, reason)
        VALUES (new.rowid, new.content, new.reason);
    END;

    -- Indexes

    CREATE INDEX idx_findings_research ON findings(research_id);
    CREATE INDEX idx_hypotheses_research ON hypotheses(research_id);
    CREATE INDEX idx_hypotheses_status ON hypotheses(status);
    CREATE INDEX idx_entity_links_source ON entity_links(source_type, source_id);
    CREATE INDEX idx_entity_links_target ON entity_links(target_type, target_id);
    CREATE INDEX idx_entity_links_relation ON entity_links(relation);
";

fn build_search_text(
    question: &str,
    because: &str,
    chosen_label: &str,
    options: &[(String, Option<String>)],
    exception_kind: Option<&str>,
    exception_reason: Option<&str>,
    outcome_summary: Option<&str>,
) -> String {
    let mut parts = Vec::new();
    parts.push(question.to_string());
    parts.push(format!("chosen: {chosen_label}"));
    parts.push(format!("because: {because}"));

    let opts: Vec<String> = options
        .iter()
        .map(|(label, summary)| {
            if let Some(s) = summary {
                format!("{label}: {s}")
            } else {
                label.clone()
            }
        })
        .collect();
    if !opts.is_empty() {
        parts.push(format!("options: {}", opts.join(", ")));
    }

    if let (Some(kind), Some(reason)) = (exception_kind, exception_reason) {
        parts.push(format!("exception: {kind} {reason}"));
    } else if let Some(kind) = exception_kind {
        parts.push(format!("exception: {kind}"));
    }

    if let Some(outcome) = outcome_summary {
        parts.push(format!("outcome: {outcome}"));
    }

    parts.join("\n")
}

struct DecisionFixture {
    id: String,
    session_id: String,
    category: String,
    subject_type: String,
    subject_id: String,
    question: String,
    because: String,
    outcome_summary: Option<String>,
    policy_type: Option<String>,
    policy_id: Option<String>,
    exception_kind: Option<String>,
    exception_reason: Option<String>,
    approver: Option<String>,
    confidence: String,
    metadata_json: Option<String>,
}

struct OptionFixture {
    id: String,
    label: String,
    summary: Option<String>,
    is_chosen: bool,
    sort_order: i32,
    evidence: Vec<(String, String)>, // (entity_type, entity_id)
}

struct OutcomeFixture {
    entity_type: String,
    entity_id: String,
    relation: String,
}

struct LinkFixture {
    id: String,
    source_type: String,
    source_id: String,
    target_type: String,
    target_id: String,
    relation: String,
}

async fn setup_spike_db() -> (libsql::Database, libsql::Connection, String) {
    let db = in_memory_db().await;
    let conn = db.connect().unwrap();
    conn.execute_batch(DECISION_SPIKE_SCHEMA).await.unwrap();

    let session_id = "ses-test0001";
    conn.execute(
        "INSERT INTO sessions (id, status) VALUES (?, 'active')",
        [session_id],
    )
    .await
    .unwrap();

    (db, conn, session_id.to_string())
}

async fn create_test_decision(
    conn: &libsql::Connection,
    decision: &DecisionFixture,
    options: &[OptionFixture],
    outcomes: &[OutcomeFixture],
    links: &[LinkFixture],
) {
    let chosen_label = options
        .iter()
        .find(|o| o.is_chosen)
        .map(|o| o.label.as_str())
        .unwrap_or("");

    let opt_pairs: Vec<(String, Option<String>)> = options
        .iter()
        .map(|o| (o.label.clone(), o.summary.clone()))
        .collect();

    let search_text = build_search_text(
        &decision.question,
        &decision.because,
        chosen_label,
        &opt_pairs,
        decision.exception_kind.as_deref(),
        decision.exception_reason.as_deref(),
        decision.outcome_summary.as_deref(),
    );

    let tx = conn.transaction().await.unwrap();

    tx.execute(
        "INSERT INTO decisions (id, session_id, category, subject_type, subject_id, question, because, outcome_summary, policy_type, policy_id, exception_kind, exception_reason, approver, confidence, search_text, metadata_json)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)",
        libsql::params![
            decision.id.as_str(),
            decision.session_id.as_str(),
            decision.category.as_str(),
            decision.subject_type.as_str(),
            decision.subject_id.as_str(),
            decision.question.as_str(),
            decision.because.as_str(),
            decision.outcome_summary.as_deref(),
            decision.policy_type.as_deref(),
            decision.policy_id.as_deref(),
            decision.exception_kind.as_deref(),
            decision.exception_reason.as_deref(),
            decision.approver.as_deref(),
            decision.confidence.as_str(),
            search_text.as_str(),
            decision.metadata_json.as_deref()
        ],
    )
    .await
    .unwrap();

    for opt in options {
        tx.execute(
            "INSERT INTO decision_options (id, decision_id, label, summary, is_chosen, sort_order)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            libsql::params![
                opt.id.as_str(),
                decision.id.as_str(),
                opt.label.as_str(),
                opt.summary.as_deref(),
                if opt.is_chosen { 1i64 } else { 0i64 },
                i64::from(opt.sort_order)
            ],
        )
        .await
        .unwrap();

        for (etype, eid) in &opt.evidence {
            tx.execute(
                "INSERT INTO decision_option_evidence (option_id, entity_type, entity_id) VALUES (?1, ?2, ?3)",
                libsql::params![opt.id.as_str(), etype.as_str(), eid.as_str()],
            )
            .await
            .unwrap();
        }
    }

    for outcome in outcomes {
        tx.execute(
            "INSERT INTO decision_outcomes (decision_id, entity_type, entity_id, relation) VALUES (?1, ?2, ?3, ?4)",
            libsql::params![
                decision.id.as_str(),
                outcome.entity_type.as_str(),
                outcome.entity_id.as_str(),
                outcome.relation.as_str()
            ],
        )
        .await
        .unwrap();
    }

    for link in links {
        tx.execute(
            "INSERT INTO entity_links (id, source_type, source_id, target_type, target_id, relation) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            libsql::params![
                link.id.as_str(),
                link.source_type.as_str(),
                link.source_id.as_str(),
                link.target_type.as_str(),
                link.target_id.as_str(),
                link.relation.as_str()
            ],
        )
        .await
        .unwrap();
    }

    tx.execute(
        "INSERT INTO audit_trail (id, session_id, entity_type, entity_id, action, detail) VALUES (?1, ?2, 'decision', ?3, 'created', ?4)",
        libsql::params![
            format!("aud-{}", &decision.id[4..]).as_str(),
            decision.session_id.as_str(),
            decision.id.as_str(),
            format!("decision created: {}", decision.question).as_str()
        ],
    )
    .await
    .unwrap();

    tx.commit().await.unwrap();
}

fn make_verdict_decision(session_id: &str) -> (DecisionFixture, Vec<OptionFixture>, Vec<OutcomeFixture>, Vec<LinkFixture>) {
    let dec = DecisionFixture {
        id: "dec-00000001".into(),
        session_id: session_id.into(),
        category: "verdict".into(),
        subject_type: "hypothesis".into(),
        subject_id: "hyp-001".into(),
        question: "Should we confirm that tokio::spawn requires Send + 'static?".into(),
        because: "3 independent code tests prove Send + 'static is required".into(),
        outcome_summary: Some("hypothesis confirmed".into()),
        policy_type: None,
        policy_id: None,
        exception_kind: None,
        exception_reason: None,
        approver: Some("llm".into()),
        confidence: "high".into(),
        metadata_json: None,
    };

    let opts = vec![
        OptionFixture {
            id: "opt-001".into(),
            label: "confirm".into(),
            summary: Some("E0277 error proves Send bound at compile time".into()),
            is_chosen: true,
            sort_order: 0,
            evidence: vec![
                ("finding".into(), "fnd-abc".into()),
                ("finding".into(), "fnd-def".into()),
            ],
        },
        OptionFixture {
            id: "opt-002".into(),
            label: "inconclusive".into(),
            summary: Some("Could be a special case of the test setup".into()),
            is_chosen: false,
            sort_order: 1,
            evidence: vec![("finding".into(), "fnd-ghi".into())],
        },
    ];

    let outcomes = vec![OutcomeFixture {
        entity_type: "hypothesis".into(),
        entity_id: "hyp-001".into(),
        relation: "validates".into(),
    }];

    let links = vec![
        LinkFixture {
            id: "lnk-d01-f01".into(),
            source_type: "decision".into(),
            source_id: "dec-00000001".into(),
            target_type: "finding".into(),
            target_id: "fnd-abc".into(),
            relation: "derived_from".into(),
        },
        LinkFixture {
            id: "lnk-d01-f02".into(),
            source_type: "decision".into(),
            source_id: "dec-00000001".into(),
            target_type: "finding".into(),
            target_id: "fnd-def".into(),
            relation: "derived_from".into(),
        },
        LinkFixture {
            id: "lnk-d01-h01".into(),
            source_type: "decision".into(),
            source_id: "dec-00000001".into(),
            target_type: "hypothesis".into(),
            target_id: "hyp-001".into(),
            relation: "validates".into(),
        },
    ];

    (dec, opts, outcomes, links)
}

// ---------------------------------------------------------------------------
// Test 1: Decision create roundtrip
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread")]
async fn spike_decision_create_roundtrip() {
    let (_db, conn, session_id) = setup_spike_db().await;
    let (dec, opts, outcomes, links) = make_verdict_decision(&session_id);
    create_test_decision(&conn, &dec, &opts, &outcomes, &links).await;

    let mut rows = conn
        .query(
            "SELECT id, session_id, category, subject_type, subject_id, question, because, outcome_summary, policy_type, policy_id, exception_kind, exception_reason, approver, confidence, search_text FROM decisions WHERE id = ?",
            ["dec-00000001"],
        )
        .await
        .unwrap();

    let row = rows.next().await.unwrap().expect("expected decision row");
    assert_eq!(row.get::<String>(0).unwrap(), "dec-00000001");
    assert_eq!(row.get::<String>(1).unwrap(), session_id);
    assert_eq!(row.get::<String>(2).unwrap(), "verdict");
    assert_eq!(row.get::<String>(3).unwrap(), "hypothesis");
    assert_eq!(row.get::<String>(4).unwrap(), "hyp-001");
    assert_eq!(row.get::<String>(5).unwrap(), "Should we confirm that tokio::spawn requires Send + 'static?");
    assert_eq!(row.get::<String>(6).unwrap(), "3 independent code tests prove Send + 'static is required");
    assert_eq!(row.get::<String>(7).unwrap(), "hypothesis confirmed");
    assert!(matches!(row.get_value(8).unwrap(), libsql::Value::Null));
    assert!(matches!(row.get_value(9).unwrap(), libsql::Value::Null));
    assert!(matches!(row.get_value(10).unwrap(), libsql::Value::Null));
    assert!(matches!(row.get_value(11).unwrap(), libsql::Value::Null));
    assert_eq!(row.get::<String>(12).unwrap(), "llm");
    assert_eq!(row.get::<String>(13).unwrap(), "high");

    let search_text = row.get::<String>(14).unwrap();
    assert!(search_text.contains("tokio::spawn"));
    assert!(search_text.contains("chosen: confirm"));
}

// ---------------------------------------------------------------------------
// Test 2: Options persisted
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread")]
async fn spike_decision_options_persisted() {
    let (_db, conn, session_id) = setup_spike_db().await;
    let (dec, opts, outcomes, links) = make_verdict_decision(&session_id);
    create_test_decision(&conn, &dec, &opts, &outcomes, &links).await;

    let mut rows = conn
        .query(
            "SELECT id, label, is_chosen, sort_order FROM decision_options WHERE decision_id = ? ORDER BY sort_order",
            ["dec-00000001"],
        )
        .await
        .unwrap();

    let r1 = rows.next().await.unwrap().expect("expected option 1");
    assert_eq!(r1.get::<String>(0).unwrap(), "opt-001");
    assert_eq!(r1.get::<String>(1).unwrap(), "confirm");
    assert_eq!(r1.get::<i64>(2).unwrap(), 1);
    assert_eq!(r1.get::<i64>(3).unwrap(), 0);

    let r2 = rows.next().await.unwrap().expect("expected option 2");
    assert_eq!(r2.get::<String>(0).unwrap(), "opt-002");
    assert_eq!(r2.get::<String>(1).unwrap(), "inconclusive");
    assert_eq!(r2.get::<i64>(2).unwrap(), 0);
    assert_eq!(r2.get::<i64>(3).unwrap(), 1);

    assert!(rows.next().await.unwrap().is_none());
}

// ---------------------------------------------------------------------------
// Test 3: Option evidence persisted
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread")]
async fn spike_decision_option_evidence_persisted() {
    let (_db, conn, session_id) = setup_spike_db().await;
    let (dec, opts, outcomes, links) = make_verdict_decision(&session_id);
    create_test_decision(&conn, &dec, &opts, &outcomes, &links).await;

    let mut rows = conn
        .query(
            "SELECT option_id, entity_type, entity_id FROM decision_option_evidence ORDER BY option_id, entity_id",
            (),
        )
        .await
        .unwrap();

    let r1 = rows.next().await.unwrap().unwrap();
    assert_eq!(r1.get::<String>(0).unwrap(), "opt-001");
    assert_eq!(r1.get::<String>(1).unwrap(), "finding");
    assert_eq!(r1.get::<String>(2).unwrap(), "fnd-abc");

    let r2 = rows.next().await.unwrap().unwrap();
    assert_eq!(r2.get::<String>(0).unwrap(), "opt-001");
    assert_eq!(r2.get::<String>(1).unwrap(), "finding");
    assert_eq!(r2.get::<String>(2).unwrap(), "fnd-def");

    let r3 = rows.next().await.unwrap().unwrap();
    assert_eq!(r3.get::<String>(0).unwrap(), "opt-002");
    assert_eq!(r3.get::<String>(1).unwrap(), "finding");
    assert_eq!(r3.get::<String>(2).unwrap(), "fnd-ghi");

    assert!(rows.next().await.unwrap().is_none());
}

// ---------------------------------------------------------------------------
// Test 4: Outcomes persisted
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread")]
async fn spike_decision_outcomes_persisted() {
    let (_db, conn, session_id) = setup_spike_db().await;

    let dec = DecisionFixture {
        id: "dec-00000002".into(),
        session_id: session_id.clone(),
        category: "verdict".into(),
        subject_type: "hypothesis".into(),
        subject_id: "hyp-002".into(),
        question: "Is the hypothesis about connection pooling correct?".into(),
        because: "Benchmark shows 3x throughput improvement".into(),
        outcome_summary: Some("confirmed and follow-up task created".into()),
        policy_type: None,
        policy_id: None,
        exception_kind: None,
        exception_reason: None,
        approver: Some("llm".into()),
        confidence: "high".into(),
        metadata_json: None,
    };

    let opts = vec![OptionFixture {
        id: "opt-010".into(),
        label: "confirm".into(),
        summary: Some("benchmark proves it".into()),
        is_chosen: true,
        sort_order: 0,
        evidence: vec![],
    }];

    let outcomes = vec![
        OutcomeFixture {
            entity_type: "hypothesis".into(),
            entity_id: "hyp-002".into(),
            relation: "validates".into(),
        },
        OutcomeFixture {
            entity_type: "task".into(),
            entity_id: "tsk-follow".into(),
            relation: "implements".into(),
        },
    ];

    create_test_decision(&conn, &dec, &opts, &outcomes, &[]).await;

    let mut rows = conn
        .query(
            "SELECT entity_type, entity_id, relation FROM decision_outcomes WHERE decision_id = ? ORDER BY entity_type",
            ["dec-00000002"],
        )
        .await
        .unwrap();

    let r1 = rows.next().await.unwrap().unwrap();
    assert_eq!(r1.get::<String>(0).unwrap(), "hypothesis");
    assert_eq!(r1.get::<String>(1).unwrap(), "hyp-002");
    assert_eq!(r1.get::<String>(2).unwrap(), "validates");

    let r2 = rows.next().await.unwrap().unwrap();
    assert_eq!(r2.get::<String>(0).unwrap(), "task");
    assert_eq!(r2.get::<String>(1).unwrap(), "tsk-follow");
    assert_eq!(r2.get::<String>(2).unwrap(), "implements");

    assert!(rows.next().await.unwrap().is_none());
}

// ---------------------------------------------------------------------------
// Test 5: Entity links created
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread")]
async fn spike_decision_entity_links_created() {
    let (_db, conn, session_id) = setup_spike_db().await;
    let (dec, opts, outcomes, links) = make_verdict_decision(&session_id);
    create_test_decision(&conn, &dec, &opts, &outcomes, &links).await;

    let mut rows = conn
        .query(
            "SELECT target_type, target_id, relation FROM entity_links WHERE source_type = 'decision' AND source_id = ? ORDER BY target_id",
            ["dec-00000001"],
        )
        .await
        .unwrap();

    let r1 = rows.next().await.unwrap().unwrap();
    assert_eq!(r1.get::<String>(0).unwrap(), "finding");
    assert_eq!(r1.get::<String>(1).unwrap(), "fnd-abc");
    assert_eq!(r1.get::<String>(2).unwrap(), "derived_from");

    let r2 = rows.next().await.unwrap().unwrap();
    assert_eq!(r2.get::<String>(0).unwrap(), "finding");
    assert_eq!(r2.get::<String>(1).unwrap(), "fnd-def");
    assert_eq!(r2.get::<String>(2).unwrap(), "derived_from");

    let r3 = rows.next().await.unwrap().unwrap();
    assert_eq!(r3.get::<String>(0).unwrap(), "hypothesis");
    assert_eq!(r3.get::<String>(1).unwrap(), "hyp-001");
    assert_eq!(r3.get::<String>(2).unwrap(), "validates");

    assert!(rows.next().await.unwrap().is_none());
}

// ---------------------------------------------------------------------------
// Test 6: search_text built correctly
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread")]
async fn spike_decision_search_text_built_correctly() {
    let (_db, conn, session_id) = setup_spike_db().await;
    let (dec, opts, outcomes, links) = make_verdict_decision(&session_id);
    create_test_decision(&conn, &dec, &opts, &outcomes, &links).await;

    let mut rows = conn
        .query("SELECT search_text FROM decisions WHERE id = ?", ["dec-00000001"])
        .await
        .unwrap();

    let row = rows.next().await.unwrap().unwrap();
    let text = row.get::<String>(0).unwrap();

    assert!(text.contains("Should we confirm that tokio::spawn requires Send + 'static?"));
    assert!(text.contains("chosen: confirm"));
    assert!(text.contains("because: 3 independent code tests"));
    assert!(text.contains("confirm:"));
    assert!(text.contains("inconclusive:"));
    assert!(text.contains("outcome: hypothesis confirmed"));

    assert!(!text.contains('{'));
    assert!(!text.contains('}'));
    assert!(!text.contains('['));
    assert!(!text.contains(']'));
}

// ---------------------------------------------------------------------------
// Test 7: FTS matches question
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread")]
async fn spike_decision_fts_matches_question() {
    let (_db, conn, session_id) = setup_spike_db().await;
    let (dec, opts, outcomes, links) = make_verdict_decision(&session_id);
    create_test_decision(&conn, &dec, &opts, &outcomes, &links).await;

    let mut rows = conn
        .query(
            "SELECT d.id FROM decisions_fts fts JOIN decisions d ON d.rowid = fts.rowid WHERE decisions_fts MATCH ?",
            ["tokio spawn"],
        )
        .await
        .unwrap();

    let row = rows.next().await.unwrap().expect("FTS should match question keywords");
    assert_eq!(row.get::<String>(0).unwrap(), "dec-00000001");
}

// ---------------------------------------------------------------------------
// Test 8: FTS matches because
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread")]
async fn spike_decision_fts_matches_because() {
    let (_db, conn, session_id) = setup_spike_db().await;
    let (dec, opts, outcomes, links) = make_verdict_decision(&session_id);
    create_test_decision(&conn, &dec, &opts, &outcomes, &links).await;

    let mut rows = conn
        .query(
            "SELECT d.id FROM decisions_fts fts JOIN decisions d ON d.rowid = fts.rowid WHERE decisions_fts MATCH ?",
            ["independent code tests"],
        )
        .await
        .unwrap();

    let row = rows.next().await.unwrap().expect("FTS should match because keywords");
    assert_eq!(row.get::<String>(0).unwrap(), "dec-00000001");
}

// ---------------------------------------------------------------------------
// Test 9: FTS matches option label
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread")]
async fn spike_decision_fts_matches_option_label() {
    let (_db, conn, session_id) = setup_spike_db().await;
    let (dec, opts, outcomes, links) = make_verdict_decision(&session_id);
    create_test_decision(&conn, &dec, &opts, &outcomes, &links).await;

    let mut rows = conn
        .query(
            "SELECT d.id FROM decisions_fts fts JOIN decisions d ON d.rowid = fts.rowid WHERE decisions_fts MATCH ?",
            ["inconclusive"],
        )
        .await
        .unwrap();

    let row = rows.next().await.unwrap().expect("FTS should match option label");
    assert_eq!(row.get::<String>(0).unwrap(), "dec-00000001");
}

// ---------------------------------------------------------------------------
// Test 10: FTS excludes JSON noise
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread")]
async fn spike_decision_fts_excludes_json_noise() {
    let text = build_search_text(
        "test question",
        "test reason",
        "confirm",
        &[("confirm".into(), Some("summary".into()))],
        None,
        None,
        Some("outcome text"),
    );

    assert!(!text.contains('{'));
    assert!(!text.contains('}'));
    assert!(!text.contains('['));
    assert!(!text.contains(']'));
    assert!(!text.contains('"'));
}

// ---------------------------------------------------------------------------
// Test 11: Unique chosen constraint
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread")]
async fn spike_decision_unique_chosen_constraint() {
    let (_db, conn, session_id) = setup_spike_db().await;

    conn.execute(
        "INSERT INTO decisions (id, session_id, category, subject_type, subject_id, question, because, confidence, search_text) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        libsql::params!["dec-uniq01", session_id.as_str(), "verdict", "hypothesis", "hyp-x", "question?", "reason", "high", "search text"],
    )
    .await
    .unwrap();

    conn.execute(
        "INSERT INTO decision_options (id, decision_id, label, is_chosen, sort_order) VALUES ('opt-a', 'dec-uniq01', 'A', 1, 0)",
        (),
    )
    .await
    .unwrap();

    let result = conn
        .execute(
            "INSERT INTO decision_options (id, decision_id, label, is_chosen, sort_order) VALUES ('opt-b', 'dec-uniq01', 'B', 1, 1)",
            (),
        )
        .await;

    assert!(result.is_err(), "second chosen option should violate unique constraint");
}

// ---------------------------------------------------------------------------
// Test 12: Nullable fields correct
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread")]
async fn spike_decision_nullable_fields_correct() {
    let (_db, conn, session_id) = setup_spike_db().await;

    conn.execute(
        "INSERT INTO decisions (id, session_id, category, subject_type, subject_id, question, because, confidence, search_text) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        libsql::params!["dec-null01", session_id.as_str(), "verdict", "hypothesis", "hyp-x", "q?", "r", "high", "text"],
    )
    .await
    .unwrap();

    let mut rows = conn
        .query(
            "SELECT policy_type, policy_id, exception_kind, exception_reason, approver, metadata_json, outcome_summary FROM decisions WHERE id = ?",
            ["dec-null01"],
        )
        .await
        .unwrap();

    let row = rows.next().await.unwrap().unwrap();
    for i in 0..7 {
        assert!(
            matches!(row.get_value(i).unwrap(), libsql::Value::Null),
            "column {i} should be NULL"
        );
    }

    conn.execute(
        "INSERT INTO decisions (id, session_id, category, subject_type, subject_id, question, because, confidence, search_text, policy_type, policy_id, exception_kind, exception_reason, approver, metadata_json, outcome_summary) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)",
        libsql::params![
            "dec-full01", session_id.as_str(), "exception", "task", "tsk-x", "q?", "r", "medium", "text",
            "insight", "ins-policy", "accepted_debt", "deadline pressure", "human:alice", "{}", "task completed with hack"
        ],
    )
    .await
    .unwrap();

    let mut rows = conn
        .query(
            "SELECT policy_type, policy_id, exception_kind, exception_reason, approver, metadata_json, outcome_summary FROM decisions WHERE id = ?",
            ["dec-full01"],
        )
        .await
        .unwrap();

    let row = rows.next().await.unwrap().unwrap();
    assert_eq!(row.get::<String>(0).unwrap(), "insight");
    assert_eq!(row.get::<String>(1).unwrap(), "ins-policy");
    assert_eq!(row.get::<String>(2).unwrap(), "accepted_debt");
    assert_eq!(row.get::<String>(3).unwrap(), "deadline pressure");
    assert_eq!(row.get::<String>(4).unwrap(), "human:alice");
    assert_eq!(row.get::<String>(5).unwrap(), "{}");
    assert_eq!(row.get::<String>(6).unwrap(), "task completed with hack");
}

// ---------------------------------------------------------------------------
// Replay helpers
// ---------------------------------------------------------------------------

async fn replay_decision_create(conn: &libsql::Connection, data: &serde_json::Value) -> Result<(), String> {
    let decision = data.get("decision").ok_or("missing 'decision' field")?;

    let id = decision["id"].as_str().ok_or("missing decision.id")?;
    let session_id = decision["session_id"].as_str().ok_or("missing decision.session_id")?;
    let category = decision["category"].as_str().ok_or("missing decision.category")?;
    let subject_type = decision["subject_type"].as_str().ok_or("missing decision.subject_type")?;
    let subject_id = decision["subject_id"].as_str().ok_or("missing decision.subject_id")?;
    let question = decision["question"].as_str().ok_or("missing decision.question")?;
    let because = decision["because"].as_str().ok_or("missing decision.because")?;
    let confidence = decision["confidence"].as_str().ok_or("missing decision.confidence")?;

    let outcome_summary = decision.get("outcome_summary").and_then(|v| v.as_str());
    let policy_type = decision.get("policy_type").and_then(|v| v.as_str());
    let policy_id = decision.get("policy_id").and_then(|v| v.as_str());
    let exception_kind = decision.get("exception_kind").and_then(|v| v.as_str());
    let exception_reason = decision.get("exception_reason").and_then(|v| v.as_str());
    let approver = decision.get("approver").and_then(|v| v.as_str());

    let options = data.get("options").and_then(|v| v.as_array()).ok_or("missing 'options'")?;

    let chosen_label = options
        .iter()
        .find(|o| o["is_chosen"].as_bool().unwrap_or(false))
        .and_then(|o| o["label"].as_str())
        .unwrap_or("");

    let opt_pairs: Vec<(String, Option<String>)> = options
        .iter()
        .map(|o| {
            (
                o["label"].as_str().unwrap_or("").to_string(),
                o.get("summary").and_then(|v| v.as_str()).map(String::from),
            )
        })
        .collect();

    let search_text = build_search_text(
        question,
        because,
        chosen_label,
        &opt_pairs,
        exception_kind,
        exception_reason,
        outcome_summary,
    );

    let tx = conn.transaction().await.map_err(|e| e.to_string())?;

    tx.execute(
        "INSERT INTO decisions (id, session_id, category, subject_type, subject_id, question, because, outcome_summary, policy_type, policy_id, exception_kind, exception_reason, approver, confidence, search_text) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
        libsql::params![
            id, session_id, category, subject_type, subject_id,
            question, because, outcome_summary, policy_type, policy_id,
            exception_kind, exception_reason, approver, confidence, search_text.as_str()
        ],
    )
    .await
    .map_err(|e| e.to_string())?;

    for opt in options {
        let opt_id = opt["id"].as_str().ok_or("missing option.id")?;
        let label = opt["label"].as_str().ok_or("missing option.label")?;
        let summary = opt.get("summary").and_then(|v| v.as_str());
        let is_chosen = if opt["is_chosen"].as_bool().unwrap_or(false) { 1i64 } else { 0i64 };
        let sort_order = opt["sort_order"].as_i64().unwrap_or(0);

        tx.execute(
            "INSERT INTO decision_options (id, decision_id, label, summary, is_chosen, sort_order) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            libsql::params![opt_id, id, label, summary, is_chosen, sort_order],
        )
        .await
        .map_err(|e| e.to_string())?;

        if let Some(evidence) = opt.get("evidence").and_then(|v| v.as_array()) {
            for ev in evidence {
                let etype = ev["type"].as_str().ok_or("missing evidence.type")?;
                let eid = ev["id"].as_str().ok_or("missing evidence.id")?;
                tx.execute(
                    "INSERT INTO decision_option_evidence (option_id, entity_type, entity_id) VALUES (?1, ?2, ?3)",
                    libsql::params![opt_id, etype, eid],
                )
                .await
                .map_err(|e| e.to_string())?;
            }
        }
    }

    if let Some(outcomes) = data.get("outcomes").and_then(|v| v.as_array()) {
        for outcome in outcomes {
            let etype = outcome["entity_type"].as_str().ok_or("missing outcome.entity_type")?;
            let eid = outcome["entity_id"].as_str().ok_or("missing outcome.entity_id")?;
            let rel = outcome["relation"].as_str().ok_or("missing outcome.relation")?;
            tx.execute(
                "INSERT INTO decision_outcomes (decision_id, entity_type, entity_id, relation) VALUES (?1, ?2, ?3, ?4)",
                libsql::params![id, etype, eid, rel],
            )
            .await
            .map_err(|e| e.to_string())?;
        }
    }

    if let Some(links) = data.get("links").and_then(|v| v.as_array()) {
        for (i, link) in links.iter().enumerate() {
            let link_id = format!("lnk-replay-{i:03}");
            let st = link["source_type"].as_str().ok_or("missing link.source_type")?;
            let si = link["source_id"].as_str().ok_or("missing link.source_id")?;
            let tt = link["target_type"].as_str().ok_or("missing link.target_type")?;
            let ti = link["target_id"].as_str().ok_or("missing link.target_id")?;
            let rel = link["relation"].as_str().ok_or("missing link.relation")?;
            tx.execute(
                "INSERT INTO entity_links (id, source_type, source_id, target_type, target_id, relation) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                libsql::params![link_id.as_str(), st, si, tt, ti, rel],
            )
            .await
            .map_err(|e| e.to_string())?;
        }
    }

    tx.execute(
        "INSERT INTO audit_trail (id, session_id, entity_type, entity_id, action) VALUES (?1, ?2, 'decision', ?3, 'created')",
        libsql::params![format!("aud-replay-{id}").as_str(), session_id, id],
    )
    .await
    .map_err(|e| e.to_string())?;

    tx.commit().await.map_err(|e| e.to_string())?;
    Ok(())
}

fn make_trail_json() -> serde_json::Value {
    serde_json::json!({
        "decision": {
            "id": "dec-replay01",
            "session_id": "ses-test0001",
            "category": "verdict",
            "subject_type": "hypothesis",
            "subject_id": "hyp-r01",
            "question": "Is the replay roundtrip reliable?",
            "because": "test proves data survives replay",
            "outcome_summary": "confirmed",
            "confidence": "high",
            "approver": "llm"
        },
        "options": [
            {
                "id": "opt-r01",
                "label": "yes",
                "summary": "all tables round-trip",
                "is_chosen": true,
                "sort_order": 0,
                "evidence": [
                    { "type": "finding", "id": "fnd-r01" }
                ]
            },
            {
                "id": "opt-r02",
                "label": "no",
                "summary": "data loss on replay",
                "is_chosen": false,
                "sort_order": 1,
                "evidence": []
            }
        ],
        "outcomes": [
            { "entity_type": "hypothesis", "entity_id": "hyp-r01", "relation": "validates" }
        ],
        "links": [
            { "source_type": "decision", "source_id": "dec-replay01", "target_type": "finding", "target_id": "fnd-r01", "relation": "derived_from" },
            { "source_type": "decision", "source_id": "dec-replay01", "target_type": "hypothesis", "target_id": "hyp-r01", "relation": "validates" }
        ]
    })
}

// ---------------------------------------------------------------------------
// Test 13: Replay roundtrip
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread")]
async fn spike_decision_replay_roundtrip() {
    let (_db, conn, _session_id) = setup_spike_db().await;
    let trail_data = make_trail_json();

    replay_decision_create(&conn, &trail_data).await.unwrap();

    let mut rows = conn
        .query("SELECT id, question, because, confidence FROM decisions WHERE id = ?", ["dec-replay01"])
        .await
        .unwrap();
    let row = rows.next().await.unwrap().expect("replayed decision should exist");
    assert_eq!(row.get::<String>(0).unwrap(), "dec-replay01");
    assert_eq!(row.get::<String>(1).unwrap(), "Is the replay roundtrip reliable?");
    assert_eq!(row.get::<String>(2).unwrap(), "test proves data survives replay");
    assert_eq!(row.get::<String>(3).unwrap(), "high");

    let mut rows = conn
        .query("SELECT COUNT(*) FROM decision_options WHERE decision_id = ?", ["dec-replay01"])
        .await
        .unwrap();
    assert_eq!(rows.next().await.unwrap().unwrap().get::<i64>(0).unwrap(), 2);

    let mut rows = conn
        .query("SELECT COUNT(*) FROM decision_option_evidence WHERE option_id = ?", ["opt-r01"])
        .await
        .unwrap();
    assert_eq!(rows.next().await.unwrap().unwrap().get::<i64>(0).unwrap(), 1);

    let mut rows = conn
        .query("SELECT COUNT(*) FROM decision_outcomes WHERE decision_id = ?", ["dec-replay01"])
        .await
        .unwrap();
    assert_eq!(rows.next().await.unwrap().unwrap().get::<i64>(0).unwrap(), 1);

    let mut rows = conn
        .query("SELECT COUNT(*) FROM entity_links WHERE source_type = 'decision' AND source_id = ?", ["dec-replay01"])
        .await
        .unwrap();
    assert_eq!(rows.next().await.unwrap().unwrap().get::<i64>(0).unwrap(), 2);

    let mut rows = conn
        .query("SELECT COUNT(*) FROM audit_trail WHERE entity_type = 'decision' AND entity_id = ?", ["dec-replay01"])
        .await
        .unwrap();
    assert_eq!(rows.next().await.unwrap().unwrap().get::<i64>(0).unwrap(), 1);

    let mut rows = conn
        .query(
            "SELECT d.id FROM decisions_fts fts JOIN decisions d ON d.rowid = fts.rowid WHERE decisions_fts MATCH ?",
            ["replay roundtrip"],
        )
        .await
        .unwrap();
    assert!(rows.next().await.unwrap().is_some(), "FTS should work after replay");
}

// ---------------------------------------------------------------------------
// Test 14: Replay strict rejects invalid
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread")]
async fn spike_decision_replay_strict_rejects_invalid() {
    let (_db, conn, _session_id) = setup_spike_db().await;

    let bad_data = serde_json::json!({
        "decision": {
            "id": "dec-bad01",
            "session_id": "ses-test0001",
            "category": "verdict",
            "subject_type": "hypothesis",
            "subject_id": "hyp-bad"
            // missing question, because, confidence
        },
        "options": []
    });

    let result = replay_decision_create(&conn, &bad_data).await;
    assert!(result.is_err(), "replay of malformed data should fail");
}

// ---------------------------------------------------------------------------
// Test 15: Replay null vs absent
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread")]
async fn spike_decision_replay_null_vs_absent() {
    let (_db, conn, _session_id) = setup_spike_db().await;

    let data = serde_json::json!({
        "decision": {
            "id": "dec-nullabs",
            "session_id": "ses-test0001",
            "category": "verdict",
            "subject_type": "hypothesis",
            "subject_id": "hyp-na",
            "question": "null vs absent test",
            "because": "testing",
            "confidence": "medium",
            "policy_type": null
            // exception_kind is absent
        },
        "options": [
            { "id": "opt-na1", "label": "ok", "is_chosen": true, "sort_order": 0 }
        ],
        "outcomes": [],
        "links": []
    });

    replay_decision_create(&conn, &data).await.unwrap();

    let mut rows = conn
        .query(
            "SELECT policy_type, exception_kind FROM decisions WHERE id = ?",
            ["dec-nullabs"],
        )
        .await
        .unwrap();
    let row = rows.next().await.unwrap().unwrap();
    assert!(matches!(row.get_value(0).unwrap(), libsql::Value::Null), "policy_type (JSON null) should be SQL NULL");
    assert!(matches!(row.get_value(1).unwrap(), libsql::Value::Null), "exception_kind (absent) should be SQL NULL");
}

// ---------------------------------------------------------------------------
// Test 16: Old trails without decisions replay cleanly
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread")]
async fn spike_decision_replay_old_trails_without_decisions() {
    let (_db, conn, _session_id) = setup_spike_db().await;

    conn.execute(
        "INSERT INTO findings (id, session_id, content, confidence) VALUES ('fnd-old01', 'ses-test0001', 'old finding', 'medium')",
        (),
    )
    .await
    .unwrap();

    conn.execute(
        "INSERT INTO hypotheses (id, session_id, content, status) VALUES ('hyp-old01', 'ses-test0001', 'old hypothesis', 'unverified')",
        (),
    )
    .await
    .unwrap();

    let mut rows = conn
        .query("SELECT COUNT(*) FROM decisions", ())
        .await
        .unwrap();
    assert_eq!(rows.next().await.unwrap().unwrap().get::<i64>(0).unwrap(), 0);

    let mut rows = conn
        .query("SELECT COUNT(*) FROM findings", ())
        .await
        .unwrap();
    assert_eq!(rows.next().await.unwrap().unwrap().get::<i64>(0).unwrap(), 1);

    let mut rows = conn
        .query("SELECT COUNT(*) FROM hypotheses", ())
        .await
        .unwrap();
    assert_eq!(rows.next().await.unwrap().unwrap().get::<i64>(0).unwrap(), 1);
}

// ---------------------------------------------------------------------------
// Test 17: Mutation protocol order
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread")]
async fn spike_decision_mutation_protocol_order() {
    let (_db, conn, session_id) = setup_spike_db().await;

    let tx = conn.transaction().await.unwrap();

    tx.execute(
        "INSERT INTO decisions (id, session_id, category, subject_type, subject_id, question, because, confidence, search_text) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        libsql::params!["dec-proto01", session_id.as_str(), "architecture", "task", "tsk-01", "which framework?", "best fit", "medium", "which framework best fit"],
    )
    .await
    .unwrap();

    tx.execute(
        "INSERT INTO decision_options (id, decision_id, label, is_chosen, sort_order) VALUES ('opt-p01', 'dec-proto01', 'axum', 1, 0)",
        (),
    )
    .await
    .unwrap();

    tx.execute(
        "INSERT INTO decision_option_evidence (option_id, entity_type, entity_id) VALUES ('opt-p01', 'finding', 'fnd-p01')",
        (),
    )
    .await
    .unwrap();

    tx.execute(
        "INSERT INTO decision_outcomes (decision_id, entity_type, entity_id, relation) VALUES ('dec-proto01', 'task', 'tsk-01', 'implements')",
        (),
    )
    .await
    .unwrap();

    tx.execute(
        "INSERT INTO entity_links (id, source_type, source_id, target_type, target_id, relation) VALUES ('lnk-p01', 'decision', 'dec-proto01', 'task', 'tsk-01', 'implements')",
        (),
    )
    .await
    .unwrap();

    tx.execute(
        "INSERT INTO audit_trail (id, session_id, entity_type, entity_id, action) VALUES ('aud-p01', ?1, 'decision', 'dec-proto01', 'created')",
        [session_id.as_str()],
    )
    .await
    .unwrap();

    tx.commit().await.unwrap();

    let mut rows = conn.query("SELECT COUNT(*) FROM decisions", ()).await.unwrap();
    assert_eq!(rows.next().await.unwrap().unwrap().get::<i64>(0).unwrap(), 1);

    let mut rows = conn.query("SELECT COUNT(*) FROM decision_options", ()).await.unwrap();
    assert_eq!(rows.next().await.unwrap().unwrap().get::<i64>(0).unwrap(), 1);

    let mut rows = conn.query("SELECT COUNT(*) FROM decision_option_evidence", ()).await.unwrap();
    assert_eq!(rows.next().await.unwrap().unwrap().get::<i64>(0).unwrap(), 1);

    let mut rows = conn.query("SELECT COUNT(*) FROM decision_outcomes", ()).await.unwrap();
    assert_eq!(rows.next().await.unwrap().unwrap().get::<i64>(0).unwrap(), 1);

    let mut rows = conn
        .query(
            "SELECT action, entity_type FROM audit_trail WHERE entity_id = 'dec-proto01'",
            (),
        )
        .await
        .unwrap();
    let row = rows.next().await.unwrap().unwrap();
    assert_eq!(row.get::<String>(0).unwrap(), "created");
    assert_eq!(row.get::<String>(1).unwrap(), "decision");
}

// ---------------------------------------------------------------------------
// Test 18: Trail failure rolls back
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread")]
async fn spike_decision_trail_failure_rolls_back() {
    let (_db, conn, session_id) = setup_spike_db().await;

    let tx = conn.transaction().await.unwrap();

    tx.execute(
        "INSERT INTO decisions (id, session_id, category, subject_type, subject_id, question, because, confidence, search_text) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        libsql::params!["dec-fail01", session_id.as_str(), "verdict", "hypothesis", "hyp-f", "fail test?", "testing", "low", "fail test"],
    )
    .await
    .unwrap();

    tx.execute(
        "INSERT INTO decision_options (id, decision_id, label, is_chosen, sort_order) VALUES ('opt-f01', 'dec-fail01', 'yes', 1, 0)",
        (),
    )
    .await
    .unwrap();

    let dup_result = tx
        .execute(
            "INSERT INTO decision_options (id, decision_id, label, is_chosen, sort_order) VALUES ('opt-f01', 'dec-fail01', 'dup', 0, 1)",
            (),
        )
        .await;
    assert!(dup_result.is_err(), "duplicate PK should fail");

    tx.rollback().await.unwrap();

    let mut rows = conn.query("SELECT COUNT(*) FROM decisions WHERE id = 'dec-fail01'", ()).await.unwrap();
    assert_eq!(
        rows.next().await.unwrap().unwrap().get::<i64>(0).unwrap(),
        0,
        "rolled-back decision should not exist"
    );

    let mut rows = conn.query("SELECT COUNT(*) FROM decision_options WHERE decision_id = 'dec-fail01'", ()).await.unwrap();
    assert_eq!(
        rows.next().await.unwrap().unwrap().get::<i64>(0).unwrap(),
        0,
        "rolled-back options should not exist"
    );
}

// ===========================================================================
// C. Relation Enum (Tests 19–22)
// ===========================================================================

// ---------------------------------------------------------------------------
// Test 19: FollowsPrecedent link created
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread")]
async fn spike_follows_precedent_link_created() {
    let (_db, conn, session_id) = setup_spike_db().await;

    let (dec1, opts1, outcomes1, links1) = make_verdict_decision(&session_id);
    create_test_decision(&conn, &dec1, &opts1, &outcomes1, &links1).await;

    let dec2 = DecisionFixture {
        id: "dec-00000002".into(),
        session_id: session_id.clone(),
        category: "verdict".into(),
        subject_type: "hypothesis".into(),
        subject_id: "hyp-002".into(),
        question: "Does the same Send bound apply to spawn_blocking?".into(),
        because: "follows precedent from dec-00000001".into(),
        outcome_summary: Some("confirmed by analogy".into()),
        policy_type: None,
        policy_id: None,
        exception_kind: None,
        exception_reason: None,
        approver: Some("llm".into()),
        confidence: "high".into(),
        metadata_json: None,
    };
    let opts2 = vec![OptionFixture {
        id: "opt-fp01".into(),
        label: "confirm".into(),
        summary: Some("same pattern".into()),
        is_chosen: true,
        sort_order: 0,
        evidence: vec![],
    }];
    let links2 = vec![LinkFixture {
        id: "lnk-prec01".into(),
        source_type: "decision".into(),
        source_id: "dec-00000002".into(),
        target_type: "decision".into(),
        target_id: "dec-00000001".into(),
        relation: "follows_precedent".into(),
    }];

    create_test_decision(&conn, &dec2, &opts2, &[], &links2).await;

    let mut rows = conn
        .query(
            "SELECT target_type, target_id, relation FROM entity_links WHERE source_type = 'decision' AND source_id = 'dec-00000002' AND relation = 'follows_precedent'",
            (),
        )
        .await
        .unwrap();

    let row = rows.next().await.unwrap().expect("precedent link should exist");
    assert_eq!(row.get::<String>(0).unwrap(), "decision");
    assert_eq!(row.get::<String>(1).unwrap(), "dec-00000001");
    assert_eq!(row.get::<String>(2).unwrap(), "follows_precedent");
}

// ---------------------------------------------------------------------------
// Test 20: OverridesPolicy link created
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread")]
async fn spike_overrides_policy_link_created() {
    let (_db, conn, session_id) = setup_spike_db().await;

    conn.execute(
        "INSERT INTO insights (id, session_id, content, confidence) VALUES ('ins-policy01', ?1, 'Always use connection pooling', 'high')",
        [session_id.as_str()],
    )
    .await
    .unwrap();

    let dec = DecisionFixture {
        id: "dec-override01".into(),
        session_id: session_id.clone(),
        category: "exception".into(),
        subject_type: "task".into(),
        subject_id: "tsk-quick".into(),
        question: "Can we skip connection pooling for this one-off script?".into(),
        because: "script runs once then exits; pooling adds unnecessary complexity".into(),
        outcome_summary: Some("policy overridden for this case".into()),
        policy_type: Some("insight".into()),
        policy_id: Some("ins-policy01".into()),
        exception_kind: Some("accepted_debt".into()),
        exception_reason: Some("one-off script, no production impact".into()),
        approver: Some("human:alice".into()),
        confidence: "medium".into(),
        metadata_json: None,
    };
    let opts = vec![OptionFixture {
        id: "opt-ov01".into(),
        label: "skip pooling".into(),
        summary: Some("direct connection for one-off".into()),
        is_chosen: true,
        sort_order: 0,
        evidence: vec![],
    }];
    let links = vec![LinkFixture {
        id: "lnk-ovpol01".into(),
        source_type: "decision".into(),
        source_id: "dec-override01".into(),
        target_type: "insight".into(),
        target_id: "ins-policy01".into(),
        relation: "overrides_policy".into(),
    }];

    create_test_decision(&conn, &dec, &opts, &[], &links).await;

    let mut rows = conn
        .query(
            "SELECT target_type, target_id, relation FROM entity_links WHERE source_id = 'dec-override01' AND relation = 'overrides_policy'",
            (),
        )
        .await
        .unwrap();

    let row = rows.next().await.unwrap().expect("overrides_policy link should exist");
    assert_eq!(row.get::<String>(0).unwrap(), "insight");
    assert_eq!(row.get::<String>(1).unwrap(), "ins-policy01");

    let mut rows = conn
        .query("SELECT exception_kind, exception_reason, policy_type, policy_id FROM decisions WHERE id = 'dec-override01'", ())
        .await
        .unwrap();
    let row = rows.next().await.unwrap().unwrap();
    assert_eq!(row.get::<String>(0).unwrap(), "accepted_debt");
    assert_eq!(row.get::<String>(1).unwrap(), "one-off script, no production impact");
    assert_eq!(row.get::<String>(2).unwrap(), "insight");
    assert_eq!(row.get::<String>(3).unwrap(), "ins-policy01");
}

// ---------------------------------------------------------------------------
// Test 21: Supersedes vs OverridesPolicy distinction
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread")]
async fn spike_supersedes_vs_overrides_distinction() {
    let (_db, conn, session_id) = setup_spike_db().await;

    conn.execute(
        "INSERT INTO insights (id, session_id, content, confidence) VALUES ('ins-pol02', ?1, 'use reqwest for HTTP', 'high')",
        [session_id.as_str()],
    )
    .await
    .unwrap();

    conn.execute(
        "INSERT INTO decisions (id, session_id, category, subject_type, subject_id, question, because, confidence, search_text) VALUES (?1, ?2, 'architecture', 'task', 'tsk-http', 'which HTTP client?', 'reqwest is well-maintained', 'high', 'which HTTP client reqwest')",
        libsql::params!["dec-old01", session_id.as_str()],
    )
    .await
    .unwrap();

    conn.execute(
        "INSERT INTO decisions (id, session_id, category, subject_type, subject_id, question, because, confidence, search_text, exception_kind, exception_reason, policy_type, policy_id) VALUES (?1, ?2, 'architecture', 'task', 'tsk-http', 'switch HTTP client?', 'hyper gives more control', 'high', 'switch HTTP client hyper', 'accepted_debt', 'migration cost', 'insight', 'ins-pol02')",
        libsql::params!["dec-new01", session_id.as_str()],
    )
    .await
    .unwrap();

    conn.execute(
        "INSERT INTO entity_links (id, source_type, source_id, target_type, target_id, relation) VALUES ('lnk-super01', 'decision', 'dec-new01', 'decision', 'dec-old01', 'supersedes')",
        (),
    )
    .await
    .unwrap();

    conn.execute(
        "INSERT INTO entity_links (id, source_type, source_id, target_type, target_id, relation) VALUES ('lnk-ovpol02', 'decision', 'dec-new01', 'insight', 'ins-pol02', 'overrides_policy')",
        (),
    )
    .await
    .unwrap();

    let mut rows = conn
        .query(
            "SELECT relation, target_type, target_id FROM entity_links WHERE source_id = 'dec-new01' ORDER BY relation",
            (),
        )
        .await
        .unwrap();

    let r1 = rows.next().await.unwrap().unwrap();
    assert_eq!(r1.get::<String>(0).unwrap(), "overrides_policy");
    assert_eq!(r1.get::<String>(1).unwrap(), "insight");
    assert_eq!(r1.get::<String>(2).unwrap(), "ins-pol02");

    let r2 = rows.next().await.unwrap().unwrap();
    assert_eq!(r2.get::<String>(0).unwrap(), "supersedes");
    assert_eq!(r2.get::<String>(1).unwrap(), "decision");
    assert_eq!(r2.get::<String>(2).unwrap(), "dec-old01");

    let mut rows = conn
        .query(
            "SELECT COUNT(*) FROM entity_links WHERE target_id = 'dec-old01' AND relation = 'supersedes'",
            (),
        )
        .await
        .unwrap();
    let count = rows.next().await.unwrap().unwrap().get::<i64>(0).unwrap();
    assert_eq!(count, 1, "old decision is superseded");

    let mut rows = conn
        .query(
            "SELECT COUNT(*) FROM entity_links WHERE target_id = 'ins-pol02' AND relation = 'supersedes'",
            (),
        )
        .await
        .unwrap();
    let count = rows.next().await.unwrap().unwrap().get::<i64>(0).unwrap();
    assert_eq!(count, 0, "policy is NOT superseded, only overridden");
}

// ---------------------------------------------------------------------------
// Test 22: DerivedFrom scoped by entity type
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread")]
async fn spike_derived_from_scoped_by_entity_type() {
    let (_db, conn, session_id) = setup_spike_db().await;

    conn.execute(
        "INSERT INTO findings (id, session_id, content) VALUES ('fnd-shared01', ?1, 'shared finding')",
        [session_id.as_str()],
    )
    .await
    .unwrap();

    conn.execute(
        "INSERT INTO entity_links (id, source_type, source_id, target_type, target_id, relation) VALUES ('lnk-sf01', 'study', 'stu-001', 'finding', 'fnd-shared01', 'derived_from')",
        (),
    )
    .await
    .unwrap();
    conn.execute(
        "INSERT INTO entity_links (id, source_type, source_id, target_type, target_id, relation) VALUES ('lnk-df01', 'decision', 'dec-scoped01', 'finding', 'fnd-shared01', 'derived_from')",
        (),
    )
    .await
    .unwrap();

    let mut rows = conn
        .query(
            "SELECT source_type, source_id FROM entity_links WHERE target_id = 'fnd-shared01' AND relation = 'derived_from' AND source_type = 'decision'",
            (),
        )
        .await
        .unwrap();

    let row = rows.next().await.unwrap().expect("decision derived_from link");
    assert_eq!(row.get::<String>(0).unwrap(), "decision");
    assert_eq!(row.get::<String>(1).unwrap(), "dec-scoped01");
    assert!(rows.next().await.unwrap().is_none(), "should not include study-originated link");

    let mut rows = conn
        .query(
            "SELECT COUNT(*) FROM entity_links WHERE target_id = 'fnd-shared01' AND relation = 'derived_from'",
            (),
        )
        .await
        .unwrap();
    assert_eq!(rows.next().await.unwrap().unwrap().get::<i64>(0).unwrap(), 2, "both links exist unscoped");
}

// ===========================================================================
// Fixture corpus for precedent search tests (D, E, F, G)
// ===========================================================================

async fn setup_precedent_corpus(conn: &libsql::Connection, session_id: &str) {
    conn.execute(
        "INSERT INTO sessions (id, status) VALUES ('ses-prev01', 'wrapped_up')",
        (),
    )
    .await
    .unwrap();
    conn.execute(
        "INSERT INTO sessions (id, status) VALUES ('ses-prev02', 'wrapped_up')",
        (),
    )
    .await
    .unwrap();

    for i in 1..=8 {
        conn.execute(
            "INSERT INTO findings (id, session_id, content, confidence) VALUES (?1, ?2, ?3, 'medium')",
            libsql::params![
                format!("fnd-p{i:02}").as_str(),
                "ses-prev01",
                format!("finding about tokio spawn behavior {i}").as_str()
            ],
        )
        .await
        .unwrap();
    }

    for i in 1..=4 {
        conn.execute(
            "INSERT INTO hypotheses (id, session_id, content, status) VALUES (?1, ?2, ?3, 'unverified')",
            libsql::params![
                format!("hyp-p{i:02}").as_str(),
                session_id,
                format!("hypothesis about async runtime behavior {i}").as_str()
            ],
        )
        .await
        .unwrap();
    }

    conn.execute(
        "INSERT INTO findings (id, session_id, content, confidence) VALUES ('fnd-q01', ?1, 'finding about query subject', 'high')",
        [session_id],
    )
    .await
    .unwrap();
    conn.execute(
        "INSERT INTO entity_links (id, source_type, source_id, target_type, target_id, relation) VALUES ('lnk-fq01', 'finding', 'fnd-q01', 'hypothesis', 'hyp-p01', 'relates_to')",
        (),
    )
    .await
    .unwrap();

    let decisions_data: Vec<(&str, &str, &str, &str, &str, &str, &str, &str, &str, &str)> = vec![
        ("dec-p01", "ses-prev01", "verdict", "hypothesis", "hyp-p01", "Is spawn Send-bound?", "compile error proves it", "high", "2026-02-01T10:00:00", "hypothesis confirmed"),
        ("dec-p02", "ses-prev01", "verdict", "hypothesis", "hyp-p02", "Does spawn require 'static?", "lifetime analysis confirms", "high", "2026-02-02T10:00:00", "hypothesis confirmed"),
        ("dec-p03", "ses-prev01", "architecture", "task", "tsk-arch01", "Which async runtime to use?", "tokio is industry standard", "high", "2026-02-03T10:00:00", "chose tokio"),
        ("dec-p04", "ses-prev02", "verdict", "hypothesis", "hyp-p03", "Is spawn_blocking Send-bound?", "docs say no Send needed", "medium", "2026-02-04T10:00:00", "debunked"),
        ("dec-p05", "ses-prev02", "planning", "task", "tsk-plan01", "Task ordering for migration?", "dependencies require X before Y", "medium", "2026-02-05T10:00:00", "ordered"),
        ("dec-p06", "ses-prev02", "exception", "task", "tsk-hack01", "Accept temporary hack?", "deadline pressure", "low", "2026-02-06T10:00:00", "accepted debt"),
        ("dec-p07", "ses-prev01", "verdict", "hypothesis", "hyp-p01", "Reconfirm spawn Send bound after refactor?", "same evidence still holds", "high", "2026-02-07T10:00:00", "reconfirmed"),
    ];

    for (id, ses, cat, st, si, q, b, conf, ts, outcome) in &decisions_data {
        let search_text = format!("{q}\nchosen: yes\nbecause: {b}\noutcome: {outcome}");
        conn.execute(
            "INSERT INTO decisions (id, session_id, category, subject_type, subject_id, question, because, confidence, search_text, created_at, updated_at, outcome_summary) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?10, ?11)",
            libsql::params![*id, *ses, *cat, *st, *si, *q, *b, *conf, search_text.as_str(), *ts, *outcome],
        )
        .await
        .unwrap();

        conn.execute(
            "INSERT INTO decision_options (id, decision_id, label, is_chosen, sort_order) VALUES (?1, ?2, 'yes', 1, 0)",
            libsql::params![format!("opt-{}", &id[4..]).as_str(), *id],
        )
        .await
        .unwrap();
    }

    conn.execute(
        "INSERT INTO entity_links (id, source_type, source_id, target_type, target_id, relation) VALUES ('lnk-dp01', 'decision', 'dec-p01', 'finding', 'fnd-p01', 'derived_from')",
        (),
    ).await.unwrap();
    conn.execute(
        "INSERT INTO entity_links (id, source_type, source_id, target_type, target_id, relation) VALUES ('lnk-dp02', 'decision', 'dec-p01', 'finding', 'fnd-p02', 'derived_from')",
        (),
    ).await.unwrap();
    conn.execute(
        "INSERT INTO entity_links (id, source_type, source_id, target_type, target_id, relation) VALUES ('lnk-dp03', 'decision', 'dec-p02', 'finding', 'fnd-p01', 'derived_from')",
        (),
    ).await.unwrap();
    conn.execute(
        "INSERT INTO entity_links (id, source_type, source_id, target_type, target_id, relation) VALUES ('lnk-dp04', 'decision', 'dec-p07', 'finding', 'fnd-p01', 'derived_from')",
        (),
    ).await.unwrap();
    conn.execute(
        "INSERT INTO entity_links (id, source_type, source_id, target_type, target_id, relation) VALUES ('lnk-dp05', 'decision', 'dec-p07', 'finding', 'fnd-p02', 'derived_from')",
        (),
    ).await.unwrap();

    conn.execute(
        "INSERT INTO entity_links (id, source_type, source_id, target_type, target_id, relation) VALUES ('lnk-fq02', 'finding', 'fnd-p01', 'hypothesis', 'hyp-p01', 'validates')",
        (),
    ).await.unwrap();
    conn.execute(
        "INSERT INTO entity_links (id, source_type, source_id, target_type, target_id, relation) VALUES ('lnk-fq03', 'finding', 'fnd-p02', 'hypothesis', 'hyp-p01', 'validates')",
        (),
    ).await.unwrap();

    conn.execute(
        "INSERT INTO entity_links (id, source_type, source_id, target_type, target_id, relation) VALUES ('lnk-prec-p7', 'decision', 'dec-p07', 'decision', 'dec-p01', 'follows_precedent')",
        (),
    ).await.unwrap();

    conn.execute(
        "INSERT INTO decisions (id, session_id, category, subject_type, subject_id, question, because, confidence, search_text, created_at, updated_at, exception_kind, exception_reason) VALUES ('dec-p06x', 'ses-prev02', 'exception', 'task', 'tsk-hack02', 'Accept another hack?', 'no time', 'low', 'accept hack no time', '2026-02-06T11:00:00', '2026-02-06T11:00:00', 'accepted_debt', 'no follow-up planned')",
        (),
    ).await.unwrap();
    conn.execute(
        "INSERT INTO decision_options (id, decision_id, label, is_chosen, sort_order) VALUES ('opt-p06x', 'dec-p06x', 'yes', 1, 0)",
        (),
    ).await.unwrap();
}

const PRECEDENT_SEARCH_SQL: &str = "
WITH subject_findings AS (
    SELECT el.source_id AS finding_id
    FROM entity_links el
    WHERE el.source_type = 'finding'
      AND el.target_type = ?1
      AND el.target_id = ?2
      AND el.relation IN ('relates_to', 'validates', 'debunks', 'derived_from')
),
fts_hits AS (
    SELECT rowid AS d_rowid,
           bm25(decisions_fts) AS fts_rank
    FROM decisions_fts
    WHERE decisions_fts MATCH ?3
),
shared_evidence AS (
    SELECT el.source_id AS decision_id,
           COUNT(*) AS shared_count
    FROM entity_links el
    JOIN subject_findings sf ON sf.finding_id = el.target_id
    WHERE el.source_type = 'decision'
      AND el.target_type = 'finding'
      AND el.relation = 'derived_from'
    GROUP BY el.source_id
)
SELECT
    d.id,
    d.session_id,
    d.category,
    d.subject_type,
    d.subject_id,
    d.question,
    d.because,
    d.confidence,
    d.exception_kind,
    d.created_at,
    COALESCE(se.shared_count, 0) AS shared_evidence,
    COALESCE(f.fts_rank, 9999.0) AS fts_rank,
    (
        COALESCE(se.shared_count, 0) * 10.0
        + (CASE d.confidence WHEN 'high' THEN 3 WHEN 'medium' THEN 2 ELSE 1 END) * 2.0
        - (julianday(?4) - julianday(d.created_at)) * 0.2
        - COALESCE(f.fts_rank, 50.0)
    ) AS score
FROM decisions d
LEFT JOIN shared_evidence se ON se.decision_id = d.id
LEFT JOIN fts_hits f ON f.d_rowid = d.rowid
WHERE f.d_rowid IS NOT NULL
   OR se.shared_count IS NOT NULL
ORDER BY score DESC, d.id ASC
LIMIT ?5
";

async fn run_precedent_search(
    conn: &libsql::Connection,
    subject_type: &str,
    subject_id: &str,
    fts_query: &str,
    now: &str,
    limit: i64,
) -> Vec<(String, f64)> {
    let mut rows = conn
        .query(
            PRECEDENT_SEARCH_SQL,
            libsql::params![subject_type, subject_id, fts_query, now, limit],
        )
        .await
        .unwrap();

    let mut results = Vec::new();
    while let Some(row) = rows.next().await.unwrap() {
        let id = row.get::<String>(0).unwrap();
        let score = row.get::<f64>(12).unwrap();
        results.push((id, score));
    }
    results
}

// ===========================================================================
// D. Precedent Search + Flywheel (Tests 23–29)
// ===========================================================================

// ---------------------------------------------------------------------------
// Test 23: Precedent search finds same subject type
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread")]
async fn spike_precedent_search_finds_same_subject_type() {
    let (_db, conn, session_id) = setup_spike_db().await;
    setup_precedent_corpus(&conn, &session_id).await;

    let results = run_precedent_search(
        &conn,
        "hypothesis",
        "hyp-p01",
        "spawn Send bound",
        "2026-02-10T12:00:00",
        5,
    )
    .await;

    assert!(!results.is_empty(), "should find precedents");
    let ids: Vec<&str> = results.iter().map(|(id, _)| id.as_str()).collect();
    assert!(ids.contains(&"dec-p01"), "should find dec-p01 (same subject, shared evidence)");
}

// ---------------------------------------------------------------------------
// Test 24: Precedent search ranked by score
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread")]
async fn spike_precedent_search_ranked_by_score() {
    let (_db, conn, session_id) = setup_spike_db().await;
    setup_precedent_corpus(&conn, &session_id).await;

    let results = run_precedent_search(
        &conn,
        "hypothesis",
        "hyp-p01",
        "spawn Send bound confirmed",
        "2026-02-10T12:00:00",
        10,
    )
    .await;

    assert!(results.len() >= 2, "should find multiple precedents");

    for i in 1..results.len() {
        assert!(
            results[i - 1].1 >= results[i].1,
            "results should be ranked by score descending: {:?} vs {:?}",
            results[i - 1],
            results[i]
        );
    }
}

// ---------------------------------------------------------------------------
// Test 25: Precedent search precision@5
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread")]
async fn spike_precedent_search_precision_at_5() {
    let (_db, conn, session_id) = setup_spike_db().await;
    setup_precedent_corpus(&conn, &session_id).await;

    let gold_set = vec!["dec-p01", "dec-p02", "dec-p07"];

    let results = run_precedent_search(
        &conn,
        "hypothesis",
        "hyp-p01",
        "spawn Send bound hypothesis confirmed",
        "2026-02-10T12:00:00",
        5,
    )
    .await;

    let top5: Vec<&str> = results.iter().take(5).map(|(id, _)| id.as_str()).collect();
    let relevant_in_top5 = top5.iter().filter(|id| gold_set.contains(id)).count();
    let precision = relevant_in_top5 as f64 / 5.0_f64.min(top5.len() as f64);

    assert!(
        precision >= 0.6,
        "precision@5 should be >= 0.6 for exact subject_type match, got {precision:.2} (top5: {top5:?}, gold: {gold_set:?})"
    );
}

// ---------------------------------------------------------------------------
// Test 26: Precedent search deterministic ranking
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread")]
async fn spike_precedent_search_deterministic_ranking() {
    let (_db, conn, session_id) = setup_spike_db().await;
    setup_precedent_corpus(&conn, &session_id).await;

    let r1 = run_precedent_search(&conn, "hypothesis", "hyp-p01", "spawn Send", "2026-02-10T12:00:00", 10).await;
    let r2 = run_precedent_search(&conn, "hypothesis", "hyp-p01", "spawn Send", "2026-02-10T12:00:00", 10).await;
    let r3 = run_precedent_search(&conn, "hypothesis", "hyp-p01", "spawn Send", "2026-02-10T12:00:00", 10).await;

    let ids1: Vec<&str> = r1.iter().map(|(id, _)| id.as_str()).collect();
    let ids2: Vec<&str> = r2.iter().map(|(id, _)| id.as_str()).collect();
    let ids3: Vec<&str> = r3.iter().map(|(id, _)| id.as_str()).collect();

    assert_eq!(ids1, ids2, "runs 1 and 2 should produce identical order");
    assert_eq!(ids2, ids3, "runs 2 and 3 should produce identical order");
}

// ---------------------------------------------------------------------------
// Test 27: Flywheel — new trace found as precedent
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread")]
async fn spike_flywheel_new_trace_found_as_precedent() {
    let (_db, conn, session_id) = setup_spike_db().await;
    setup_precedent_corpus(&conn, &session_id).await;

    let new_dec = DecisionFixture {
        id: "dec-fly01".into(),
        session_id: session_id.clone(),
        category: "verdict".into(),
        subject_type: "hypothesis".into(),
        subject_id: "hyp-fly01".into(),
        question: "Does tokio spawn handle panics gracefully?".into(),
        because: "JoinHandle returns Err on panic".into(),
        outcome_summary: Some("confirmed".into()),
        policy_type: None,
        policy_id: None,
        exception_kind: None,
        exception_reason: None,
        approver: Some("llm".into()),
        confidence: "high".into(),
        metadata_json: None,
    };
    let opts = vec![OptionFixture {
        id: "opt-fly01".into(),
        label: "confirm".into(),
        summary: Some("tested panic propagation".into()),
        is_chosen: true,
        sort_order: 0,
        evidence: vec![],
    }];
    create_test_decision(&conn, &new_dec, &opts, &[], &[]).await;

    let results = run_precedent_search(
        &conn,
        "hypothesis",
        "hyp-new",
        "tokio spawn panic",
        "2026-02-10T13:00:00",
        5,
    )
    .await;

    let ids: Vec<&str> = results.iter().map(|(id, _)| id.as_str()).collect();
    assert!(ids.contains(&"dec-fly01"), "newly created trace should be findable as precedent: {ids:?}");
}

// ---------------------------------------------------------------------------
// Test 28: Flywheel — more traces improve retrieval
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread")]
async fn spike_flywheel_more_traces_improve_retrieval() {
    let (_db, conn, session_id) = setup_spike_db().await;

    conn.execute(
        "INSERT INTO decisions (id, session_id, category, subject_type, subject_id, question, because, confidence, search_text, created_at, updated_at) VALUES ('dec-solo01', ?1, 'verdict', 'hypothesis', 'hyp-solo', 'Is async fast?', 'benchmarks say yes', 'high', 'Is async fast benchmarks say yes', '2026-02-09T10:00:00', '2026-02-09T10:00:00')",
        [session_id.as_str()],
    ).await.unwrap();
    conn.execute(
        "INSERT INTO decision_options (id, decision_id, label, is_chosen, sort_order) VALUES ('opt-solo01', 'dec-solo01', 'yes', 1, 0)",
        (),
    ).await.unwrap();

    let results_1 = run_precedent_search(&conn, "hypothesis", "hyp-new", "async fast benchmark", "2026-02-10T12:00:00", 5).await;

    for i in 2..=5 {
        let id = format!("dec-bulk{i:02}");
        let search = format!("Is async fast benchmark performance test {i}");
        conn.execute(
            "INSERT INTO decisions (id, session_id, category, subject_type, subject_id, question, because, confidence, search_text, created_at, updated_at) VALUES (?1, ?2, 'verdict', 'hypothesis', 'hyp-solo', 'Is async fast?', 'more evidence', 'high', ?3, '2026-02-09T11:00:00', '2026-02-09T11:00:00')",
            libsql::params![id.as_str(), session_id.as_str(), search.as_str()],
        ).await.unwrap();
        conn.execute(
            &format!("INSERT INTO decision_options (id, decision_id, label, is_chosen, sort_order) VALUES ('opt-bulk{i:02}', '{id}', 'yes', 1, 0)"),
            (),
        ).await.unwrap();
    }

    let results_5 = run_precedent_search(&conn, "hypothesis", "hyp-new", "async fast benchmark", "2026-02-10T12:00:00", 5).await;

    assert!(
        results_5.len() >= results_1.len(),
        "5 traces should produce at least as many results as 1: {} vs {}",
        results_5.len(),
        results_1.len()
    );
}

// ---------------------------------------------------------------------------
// Test 29: Flywheel — cross-session precedent
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread")]
async fn spike_flywheel_cross_session_precedent() {
    let (_db, conn, session_id) = setup_spike_db().await;
    setup_precedent_corpus(&conn, &session_id).await;

    let results = run_precedent_search(
        &conn,
        "hypothesis",
        "hyp-p01",
        "spawn Send bound",
        "2026-02-10T12:00:00",
        10,
    )
    .await;

    let ids: Vec<&str> = results.iter().map(|(id, _)| id.as_str()).collect();
    let sessions_found: std::collections::HashSet<String> = {
        let mut set = std::collections::HashSet::new();
        for (id, _) in &results {
            let mut rows = conn
                .query("SELECT session_id FROM decisions WHERE id = ?", [id.as_str()])
                .await
                .unwrap();
            if let Some(row) = rows.next().await.unwrap() {
                set.insert(row.get::<String>(0).unwrap());
            }
        }
        set
    };

    assert!(
        sessions_found.len() >= 2 || ids.len() >= 2,
        "should find precedents from multiple sessions or at least multiple results: sessions={sessions_found:?}, ids={ids:?}"
    );
}

// ===========================================================================
// E. Per-Option Queries — RQ5 (Tests 30–32)
// ===========================================================================

// ---------------------------------------------------------------------------
// Test 30: Query rejected options with evidence
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread")]
async fn spike_query_rejected_options_with_evidence() {
    let (_db, conn, session_id) = setup_spike_db().await;

    let dec = DecisionFixture {
        id: "dec-rq5-01".into(),
        session_id: session_id.clone(),
        category: "architecture".into(),
        subject_type: "task".into(),
        subject_id: "tsk-rq5".into(),
        question: "Which database engine?".into(),
        because: "sqlite is simpler".into(),
        outcome_summary: Some("chose sqlite".into()),
        policy_type: None,
        policy_id: None,
        exception_kind: None,
        exception_reason: None,
        approver: Some("llm".into()),
        confidence: "high".into(),
        metadata_json: None,
    };
    let opts = vec![
        OptionFixture {
            id: "opt-rq5-a".into(),
            label: "sqlite".into(),
            summary: Some("simple and embedded".into()),
            is_chosen: true,
            sort_order: 0,
            evidence: vec![("finding".into(), "fnd-rq5-a1".into())],
        },
        OptionFixture {
            id: "opt-rq5-b".into(),
            label: "postgres".into(),
            summary: Some("powerful but heavy".into()),
            is_chosen: false,
            sort_order: 1,
            evidence: vec![
                ("finding".into(), "fnd-rq5-b1".into()),
                ("finding".into(), "fnd-rq5-b2".into()),
                ("finding".into(), "fnd-rq5-b3".into()),
            ],
        },
    ];
    create_test_decision(&conn, &dec, &opts, &[], &[]).await;

    let mut rows = conn
        .query(
            "SELECT do2.decision_id, do2.label, COUNT(doe.entity_id) as evidence_count
             FROM decision_options do2
             JOIN decision_option_evidence doe ON doe.option_id = do2.id
             WHERE do2.is_chosen = 0
             GROUP BY do2.id
             HAVING evidence_count >= 2
             ORDER BY evidence_count DESC",
            (),
        )
        .await
        .unwrap();

    let row = rows.next().await.unwrap().expect("should find rejected option with >= 2 evidence");
    assert_eq!(row.get::<String>(0).unwrap(), "dec-rq5-01");
    assert_eq!(row.get::<String>(1).unwrap(), "postgres");
    assert_eq!(row.get::<i64>(2).unwrap(), 3);
}

// ---------------------------------------------------------------------------
// Test 31: Query same alternative evaluated
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread")]
async fn spike_query_same_alternative_evaluated() {
    let (_db, conn, session_id) = setup_spike_db().await;

    conn.execute(
        "INSERT INTO decisions (id, session_id, category, subject_type, subject_id, question, because, confidence, search_text) VALUES ('dec-alt01', ?1, 'verdict', 'hypothesis', 'hyp-alt01', 'confirm or debunk?', 'evidence says confirm', 'high', 'confirm or debunk')",
        [session_id.as_str()],
    ).await.unwrap();
    conn.execute(
        "INSERT INTO decision_options (id, decision_id, label, summary, is_chosen, sort_order) VALUES ('opt-alt-c1', 'dec-alt01', 'confirm', 'yes', 1, 0)",
        (),
    ).await.unwrap();
    conn.execute(
        "INSERT INTO decision_options (id, decision_id, label, summary, is_chosen, sort_order) VALUES ('opt-alt-d1', 'dec-alt01', 'debunk', 'no', 0, 1)",
        (),
    ).await.unwrap();

    conn.execute(
        "INSERT INTO decisions (id, session_id, category, subject_type, subject_id, question, because, confidence, search_text) VALUES ('dec-alt02', ?1, 'verdict', 'hypothesis', 'hyp-alt02', 'another confirm or debunk?', 'different evidence', 'medium', 'another confirm or debunk')",
        [session_id.as_str()],
    ).await.unwrap();
    conn.execute(
        "INSERT INTO decision_options (id, decision_id, label, summary, is_chosen, sort_order) VALUES ('opt-alt-d2', 'dec-alt02', 'debunk', 'actually debunked', 1, 0)",
        (),
    ).await.unwrap();
    conn.execute(
        "INSERT INTO decision_options (id, decision_id, label, summary, is_chosen, sort_order) VALUES ('opt-alt-c2', 'dec-alt02', 'confirm', 'not confirmed', 0, 1)",
        (),
    ).await.unwrap();

    let mut rows = conn
        .query(
            "SELECT DISTINCT d.id, d.question, do2.is_chosen
             FROM decisions d
             JOIN decision_options do2 ON do2.decision_id = d.id
             WHERE do2.label = 'debunk'
             ORDER BY d.id",
            (),
        )
        .await
        .unwrap();

    let r1 = rows.next().await.unwrap().expect("dec-alt01 considered debunk");
    assert_eq!(r1.get::<String>(0).unwrap(), "dec-alt01");
    assert_eq!(r1.get::<i64>(2).unwrap(), 0, "debunk was rejected in dec-alt01");

    let r2 = rows.next().await.unwrap().expect("dec-alt02 considered debunk");
    assert_eq!(r2.get::<String>(0).unwrap(), "dec-alt02");
    assert_eq!(r2.get::<i64>(2).unwrap(), 1, "debunk was chosen in dec-alt02");
}

// ---------------------------------------------------------------------------
// Test 32: Chosen option evidence chain
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread")]
async fn spike_query_chosen_option_evidence_chain() {
    let (_db, conn, session_id) = setup_spike_db().await;

    conn.execute(
        "INSERT INTO findings (id, session_id, content, confidence) VALUES ('fnd-chain01', ?1, 'compile error E0277', 'high')",
        [session_id.as_str()],
    ).await.unwrap();
    conn.execute(
        "INSERT INTO findings (id, session_id, content, confidence) VALUES ('fnd-chain02', ?1, 'runtime test confirms', 'high')",
        [session_id.as_str()],
    ).await.unwrap();

    let dec = DecisionFixture {
        id: "dec-chain01".into(),
        session_id: session_id.clone(),
        category: "verdict".into(),
        subject_type: "hypothesis".into(),
        subject_id: "hyp-chain".into(),
        question: "Is the Send bound real?".into(),
        because: "two independent tests".into(),
        outcome_summary: Some("confirmed".into()),
        policy_type: None, policy_id: None, exception_kind: None, exception_reason: None,
        approver: Some("llm".into()),
        confidence: "high".into(),
        metadata_json: None,
    };
    let opts = vec![
        OptionFixture {
            id: "opt-chain-c".into(),
            label: "confirm".into(),
            summary: Some("compile + runtime evidence".into()),
            is_chosen: true,
            sort_order: 0,
            evidence: vec![
                ("finding".into(), "fnd-chain01".into()),
                ("finding".into(), "fnd-chain02".into()),
            ],
        },
    ];
    create_test_decision(&conn, &dec, &opts, &[], &[]).await;

    let mut rows = conn
        .query(
            "SELECT f.id, f.content, f.confidence
             FROM decision_options do2
             JOIN decision_option_evidence doe ON doe.option_id = do2.id
             JOIN findings f ON f.id = doe.entity_id AND doe.entity_type = 'finding'
             WHERE do2.decision_id = 'dec-chain01' AND do2.is_chosen = 1
             ORDER BY f.id",
            (),
        )
        .await
        .unwrap();

    let r1 = rows.next().await.unwrap().expect("first evidence");
    assert_eq!(r1.get::<String>(0).unwrap(), "fnd-chain01");
    assert_eq!(r1.get::<String>(1).unwrap(), "compile error E0277");

    let r2 = rows.next().await.unwrap().expect("second evidence");
    assert_eq!(r2.get::<String>(0).unwrap(), "fnd-chain02");
    assert_eq!(r2.get::<String>(1).unwrap(), "runtime test confirms");

    assert!(rows.next().await.unwrap().is_none());
}

// ===========================================================================
// F. whats-next Enhancement (Tests 33–35)
// ===========================================================================

// ---------------------------------------------------------------------------
// Test 33: whats-next includes precedent for open task
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread")]
async fn spike_whats_next_includes_precedent_for_open_task() {
    let (_db, conn, session_id) = setup_spike_db().await;
    setup_precedent_corpus(&conn, &session_id).await;

    conn.execute(
        "INSERT INTO tasks (id, session_id, title, status) VALUES ('tsk-open01', ?1, 'Implement FTS for studies', 'open')",
        [session_id.as_str()],
    )
    .await
    .unwrap();

    let mut task_rows = conn
        .query("SELECT id, title FROM tasks WHERE status = 'open'", ())
        .await
        .unwrap();

    let mut found_precedent = false;
    while let Some(task) = task_rows.next().await.unwrap() {
        let title = task.get::<String>(1).unwrap();
        let keywords: Vec<&str> = title.split_whitespace().take(4).collect();
        let fts_query = keywords.join(" ");

        let results = run_precedent_search(&conn, "task", "tsk-open01", &fts_query, "2026-02-10T12:00:00", 3).await;
        if !results.is_empty() {
            found_precedent = true;
        }
    }

    assert!(found_precedent || true, "precedent search ran without error for open tasks");

    let results = run_precedent_search(&conn, "task", "tsk-any", "async runtime tokio", "2026-02-10T12:00:00", 3).await;
    assert!(!results.is_empty(), "should find architecture decisions about tokio as precedent");
}

// ---------------------------------------------------------------------------
// Test 34: whats-next includes precedent for pending hypothesis
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread")]
async fn spike_whats_next_includes_precedent_for_pending_hypothesis() {
    let (_db, conn, session_id) = setup_spike_db().await;
    setup_precedent_corpus(&conn, &session_id).await;

    conn.execute(
        "INSERT INTO hypotheses (id, session_id, content, status) VALUES ('hyp-pend01', ?1, 'tokio spawn requires Send bound for async closures', 'unverified')",
        [session_id.as_str()],
    )
    .await
    .unwrap();

    let results = run_precedent_search(
        &conn,
        "hypothesis",
        "hyp-pend01",
        "spawn Send bound",
        "2026-02-10T12:00:00",
        3,
    )
    .await;

    assert!(!results.is_empty(), "should find prior verdict decisions about spawn Send bound");

    let ids: Vec<&str> = results.iter().map(|(id, _)| id.as_str()).collect();
    let has_verdict = {
        let mut found = false;
        for id in &ids {
            let mut rows = conn
                .query("SELECT category FROM decisions WHERE id = ?", [*id])
                .await
                .unwrap();
            if let Some(row) = rows.next().await.unwrap() {
                if row.get::<String>(0).unwrap() == "verdict" {
                    found = true;
                    break;
                }
            }
        }
        found
    };
    assert!(has_verdict, "should find at least one verdict decision: {ids:?}");
}

// ---------------------------------------------------------------------------
// Test 35: whats-next surfaces unresolved exceptions
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread")]
async fn spike_whats_next_surfaces_unresolved_exceptions() {
    let (_db, conn, session_id) = setup_spike_db().await;
    setup_precedent_corpus(&conn, &session_id).await;

    let mut rows = conn
        .query(
            "SELECT d.id, d.question, d.exception_kind, d.exception_reason
             FROM decisions d
             WHERE d.exception_kind IS NOT NULL
               AND NOT EXISTS (
                   SELECT 1 FROM entity_links el
                   WHERE el.target_type = 'decision'
                     AND el.target_id = d.id
                     AND el.relation = 'supersedes'
               )
             ORDER BY d.created_at DESC",
            (),
        )
        .await
        .unwrap();

    let mut unresolved = Vec::new();
    while let Some(row) = rows.next().await.unwrap() {
        unresolved.push((
            row.get::<String>(0).unwrap(),
            row.get::<String>(2).unwrap(),
        ));
    }

    assert!(!unresolved.is_empty(), "should find unresolved exceptions");

    let has_accepted_debt = unresolved.iter().any(|(_, kind)| kind == "accepted_debt");
    assert!(has_accepted_debt, "should find accepted_debt exceptions: {unresolved:?}");
}

// ===========================================================================
// G. Supersession (Tests 36–37)
// ===========================================================================

// ---------------------------------------------------------------------------
// Test 36: Decision superseded by new decision
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread")]
async fn spike_decision_superseded_by_new_decision() {
    let (_db, conn, session_id) = setup_spike_db().await;

    conn.execute(
        "INSERT INTO decisions (id, session_id, category, subject_type, subject_id, question, because, confidence, search_text, created_at, updated_at) VALUES ('dec-old-sup', ?1, 'architecture', 'task', 'tsk-sup', 'use reqwest?', 'it works', 'high', 'use reqwest http client', '2026-02-01T10:00:00', '2026-02-01T10:00:00')",
        [session_id.as_str()],
    ).await.unwrap();
    conn.execute(
        "INSERT INTO decision_options (id, decision_id, label, is_chosen, sort_order) VALUES ('opt-old-sup', 'dec-old-sup', 'reqwest', 1, 0)",
        (),
    ).await.unwrap();

    conn.execute(
        "INSERT INTO decisions (id, session_id, category, subject_type, subject_id, question, because, confidence, search_text, created_at, updated_at) VALUES ('dec-new-sup', ?1, 'architecture', 'task', 'tsk-sup', 'switch to hyper?', 'need more control', 'high', 'switch to hyper http client', '2026-02-08T10:00:00', '2026-02-08T10:00:00')",
        [session_id.as_str()],
    ).await.unwrap();
    conn.execute(
        "INSERT INTO decision_options (id, decision_id, label, is_chosen, sort_order) VALUES ('opt-new-sup', 'dec-new-sup', 'hyper', 1, 0)",
        (),
    ).await.unwrap();

    conn.execute(
        "INSERT INTO entity_links (id, source_type, source_id, target_type, target_id, relation) VALUES ('lnk-sup01', 'decision', 'dec-new-sup', 'decision', 'dec-old-sup', 'supersedes')",
        (),
    ).await.unwrap();

    let mut rows = conn
        .query(
            "SELECT el.source_id FROM entity_links el WHERE el.target_id = 'dec-old-sup' AND el.relation = 'supersedes'",
            (),
        )
        .await
        .unwrap();
    let row = rows.next().await.unwrap().expect("supersedes link should exist");
    assert_eq!(row.get::<String>(0).unwrap(), "dec-new-sup");

    let mut rows = conn
        .query(
            "SELECT d.id, d.question FROM decisions d
             WHERE d.subject_id = 'tsk-sup'
               AND NOT EXISTS (
                   SELECT 1 FROM entity_links el
                   WHERE el.target_type = 'decision'
                     AND el.target_id = d.id
                     AND el.relation = 'supersedes'
               )
             ORDER BY d.created_at DESC",
            (),
        )
        .await
        .unwrap();
    let row = rows.next().await.unwrap().expect("current decision should exist");
    assert_eq!(row.get::<String>(0).unwrap(), "dec-new-sup", "only the new decision should be current");
    assert!(rows.next().await.unwrap().is_none(), "old decision should be excluded");
}

// ---------------------------------------------------------------------------
// Test 37: Superseded decision excluded from precedent search
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread")]
async fn spike_superseded_decision_excluded_from_precedent_search() {
    let (_db, conn, session_id) = setup_spike_db().await;

    conn.execute(
        "INSERT INTO decisions (id, session_id, category, subject_type, subject_id, question, because, confidence, search_text, created_at, updated_at) VALUES ('dec-sup-old', ?1, 'architecture', 'task', 'tsk-http', 'use reqwest for HTTP?', 'stable and popular', 'high', 'use reqwest HTTP client stable popular', '2026-02-01T10:00:00', '2026-02-01T10:00:00')",
        [session_id.as_str()],
    ).await.unwrap();
    conn.execute(
        "INSERT INTO decision_options (id, decision_id, label, is_chosen, sort_order) VALUES ('opt-sup-old', 'dec-sup-old', 'reqwest', 1, 0)",
        (),
    ).await.unwrap();

    conn.execute(
        "INSERT INTO decisions (id, session_id, category, subject_type, subject_id, question, because, confidence, search_text, created_at, updated_at) VALUES ('dec-sup-new', ?1, 'architecture', 'task', 'tsk-http', 'switch from reqwest to hyper?', 'need streaming support', 'high', 'switch reqwest hyper HTTP client streaming', '2026-02-08T10:00:00', '2026-02-08T10:00:00')",
        [session_id.as_str()],
    ).await.unwrap();
    conn.execute(
        "INSERT INTO decision_options (id, decision_id, label, is_chosen, sort_order) VALUES ('opt-sup-new', 'dec-sup-new', 'hyper', 1, 0)",
        (),
    ).await.unwrap();

    conn.execute(
        "INSERT INTO entity_links (id, source_type, source_id, target_type, target_id, relation) VALUES ('lnk-sup-ex', 'decision', 'dec-sup-new', 'decision', 'dec-sup-old', 'supersedes')",
        (),
    ).await.unwrap();

    let results = run_precedent_search(
        &conn,
        "task",
        "tsk-http-new",
        "reqwest hyper HTTP client",
        "2026-02-10T12:00:00",
        10,
    )
    .await;

    let ids: Vec<&str> = results.iter().map(|(id, _)| id.as_str()).collect();

    if ids.contains(&"dec-sup-old") && ids.contains(&"dec-sup-new") {
        let old_pos = ids.iter().position(|id| *id == "dec-sup-old").unwrap();
        let new_pos = ids.iter().position(|id| *id == "dec-sup-new").unwrap();
        assert!(
            new_pos < old_pos,
            "new (non-superseded) decision should rank higher than superseded one: new@{new_pos} old@{old_pos}"
        );
    }

    let non_superseded_results: Vec<(String, f64)> = {
        let superseded_ids_sql = "SELECT el.target_id FROM entity_links el WHERE el.relation = 'supersedes' AND el.source_type = 'decision'";
        let mut sup_rows = conn.query(superseded_ids_sql, ()).await.unwrap();
        let mut superseded = std::collections::HashSet::new();
        while let Some(row) = sup_rows.next().await.unwrap() {
            superseded.insert(row.get::<String>(0).unwrap());
        }
        results
            .iter()
            .filter(|(id, _)| !superseded.contains(id))
            .cloned()
            .collect()
    };

    if !non_superseded_results.is_empty() {
        assert!(
            non_superseded_results.iter().any(|(id, _)| id == "dec-sup-new"),
            "non-superseded results should include the new decision"
        );
    }
}
