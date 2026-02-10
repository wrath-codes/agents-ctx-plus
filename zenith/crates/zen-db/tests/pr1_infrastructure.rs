//! PR 1 Infrastructure Integration Tests
//!
//! Tests for Phase 2 Section A:
//! - Audit repo: append, query with filters, FTS search
//! - Session repo: start, end, transitions, orphan detection, list, snapshot
//! - Trail writer: file creation, roundtrip, per-session files, disabled noop
//! - Service: construction

use chrono::Utc;
use tempfile::TempDir;

use zen_core::entities::AuditEntry;
use zen_core::enums::{AuditAction, EntityType, SessionStatus, TrailOp};
use zen_core::trail::TrailOperation;
use zen_db::repos::audit::AuditFilter;
use zen_db::service::ZenService;
use zen_db::trail::writer::TrailWriter;

async fn test_service() -> ZenService {
    ZenService::new_local(":memory:", None).await.unwrap()
}

async fn test_service_with_trail(trail_dir: &std::path::Path) -> ZenService {
    ZenService::new_local(":memory:", Some(trail_dir.to_path_buf()))
        .await
        .unwrap()
}

// ---------------------------------------------------------------------------
// Service tests
// ---------------------------------------------------------------------------

#[tokio::test]
async fn service_new_local() {
    let svc = ZenService::new_local(":memory:", None).await.unwrap();
    assert!(!svc.trail().is_enabled());
}

// ---------------------------------------------------------------------------
// Audit tests
// ---------------------------------------------------------------------------

#[tokio::test]
async fn audit_append_and_query() {
    let svc = test_service().await;
    let now = Utc::now();

    svc.db()
        .conn()
        .execute(
            "INSERT INTO sessions (id, status) VALUES ('ses-00000001', 'active')",
            (),
        )
        .await
        .unwrap();

    for i in 0..3 {
        svc.append_audit(&AuditEntry {
            id: format!("aud-{i:08x}"),
            session_id: Some("ses-00000001".to_string()),
            entity_type: EntityType::Finding,
            entity_id: format!("fnd-{i:08x}"),
            action: AuditAction::Created,
            detail: None,
            created_at: now,
        })
        .await
        .unwrap();
    }

    let results = svc.query_audit(&AuditFilter::default()).await.unwrap();
    assert_eq!(results.len(), 3);
}

#[tokio::test]
async fn audit_filter_by_entity() {
    let svc = test_service().await;
    let now = Utc::now();

    svc.db()
        .conn()
        .execute(
            "INSERT INTO sessions (id, status) VALUES ('ses-00000001', 'active')",
            (),
        )
        .await
        .unwrap();

    svc.append_audit(&AuditEntry {
        id: "aud-00000001".into(),
        session_id: Some("ses-00000001".into()),
        entity_type: EntityType::Finding,
        entity_id: "fnd-00000001".into(),
        action: AuditAction::Created,
        detail: None,
        created_at: now,
    })
    .await
    .unwrap();

    svc.append_audit(&AuditEntry {
        id: "aud-00000002".into(),
        session_id: Some("ses-00000001".into()),
        entity_type: EntityType::Task,
        entity_id: "tsk-00000001".into(),
        action: AuditAction::Created,
        detail: None,
        created_at: now,
    })
    .await
    .unwrap();

    let results = svc
        .query_audit(&AuditFilter {
            entity_type: Some(EntityType::Finding),
            ..AuditFilter::default()
        })
        .await
        .unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].entity_type, EntityType::Finding);
}

#[tokio::test]
async fn audit_filter_by_action() {
    let svc = test_service().await;
    let now = Utc::now();

    svc.db()
        .conn()
        .execute(
            "INSERT INTO sessions (id, status) VALUES ('ses-00000001', 'active')",
            (),
        )
        .await
        .unwrap();

    svc.append_audit(&AuditEntry {
        id: "aud-00000001".into(),
        session_id: Some("ses-00000001".into()),
        entity_type: EntityType::Finding,
        entity_id: "fnd-00000001".into(),
        action: AuditAction::Created,
        detail: None,
        created_at: now,
    })
    .await
    .unwrap();

    svc.append_audit(&AuditEntry {
        id: "aud-00000002".into(),
        session_id: Some("ses-00000001".into()),
        entity_type: EntityType::Finding,
        entity_id: "fnd-00000001".into(),
        action: AuditAction::Updated,
        detail: None,
        created_at: now,
    })
    .await
    .unwrap();

    let results = svc
        .query_audit(&AuditFilter {
            action: Some(AuditAction::Created),
            ..AuditFilter::default()
        })
        .await
        .unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].action, AuditAction::Created);
}

#[tokio::test]
async fn audit_filter_by_session() {
    let svc = test_service().await;
    let now = Utc::now();

    svc.db()
        .conn()
        .execute(
            "INSERT INTO sessions (id, status) VALUES ('ses-00000001', 'active')",
            (),
        )
        .await
        .unwrap();
    svc.db()
        .conn()
        .execute(
            "INSERT INTO sessions (id, status) VALUES ('ses-00000002', 'active')",
            (),
        )
        .await
        .unwrap();

    svc.append_audit(&AuditEntry {
        id: "aud-00000001".into(),
        session_id: Some("ses-00000001".into()),
        entity_type: EntityType::Finding,
        entity_id: "fnd-00000001".into(),
        action: AuditAction::Created,
        detail: None,
        created_at: now,
    })
    .await
    .unwrap();

    svc.append_audit(&AuditEntry {
        id: "aud-00000002".into(),
        session_id: Some("ses-00000002".into()),
        entity_type: EntityType::Finding,
        entity_id: "fnd-00000002".into(),
        action: AuditAction::Created,
        detail: None,
        created_at: now,
    })
    .await
    .unwrap();

    let results = svc
        .query_audit(&AuditFilter {
            session_id: Some("ses-00000001".into()),
            ..AuditFilter::default()
        })
        .await
        .unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].session_id, Some("ses-00000001".to_string()));
}

#[tokio::test]
async fn audit_search_fts() {
    let svc = test_service().await;
    let now = Utc::now();

    svc.db()
        .conn()
        .execute(
            "INSERT INTO sessions (id, status) VALUES ('ses-00000001', 'active')",
            (),
        )
        .await
        .unwrap();

    svc.append_audit(&AuditEntry {
        id: "aud-00000001".into(),
        session_id: Some("ses-00000001".into()),
        entity_type: EntityType::Finding,
        entity_id: "fnd-00000001".into(),
        action: AuditAction::Created,
        detail: Some(serde_json::json!({"note": "tokio runtime compatibility"})),
        created_at: now,
    })
    .await
    .unwrap();

    let results = svc.search_audit("runtime", 10).await.unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].id, "aud-00000001");
}

// ---------------------------------------------------------------------------
// Session tests
// ---------------------------------------------------------------------------

#[tokio::test]
async fn session_start_creates_active() {
    let tmp = TempDir::new().unwrap();
    let svc = test_service_with_trail(tmp.path()).await;

    let (session, prev) = svc.start_session().await.unwrap();
    assert_eq!(session.status, SessionStatus::Active);
    assert!(session.ended_at.is_none());
    assert!(prev.is_none());
}

#[tokio::test]
async fn session_end_transitions() {
    let tmp = TempDir::new().unwrap();
    let svc = test_service_with_trail(tmp.path()).await;

    let (session, _) = svc.start_session().await.unwrap();
    let ended = svc.end_session(&session.id, "Test summary").await.unwrap();

    assert_eq!(ended.status, SessionStatus::WrappedUp);
    assert!(ended.ended_at.is_some());
    assert_eq!(ended.summary, Some("Test summary".to_string()));
}

#[tokio::test]
async fn session_end_invalid_transition() {
    let tmp = TempDir::new().unwrap();
    let svc = test_service_with_trail(tmp.path()).await;

    let (session, _) = svc.start_session().await.unwrap();
    svc.end_session(&session.id, "Done").await.unwrap();

    let result = svc.end_session(&session.id, "Again").await;
    assert!(result.is_err());
}

#[tokio::test]
async fn session_orphan_detection() {
    let tmp = TempDir::new().unwrap();
    let svc = test_service_with_trail(tmp.path()).await;

    let (first, _) = svc.start_session().await.unwrap();
    let first_id = first.id.clone();

    let (second, prev) = svc.start_session().await.unwrap();
    assert_ne!(second.id, first_id);
    assert!(prev.is_some());

    let first_after = svc.get_session(&first_id).await.unwrap();
    assert_eq!(first_after.status, SessionStatus::Abandoned);
}

#[tokio::test]
async fn session_list_by_status() {
    let tmp = TempDir::new().unwrap();
    let svc = test_service_with_trail(tmp.path()).await;

    let (s1, _) = svc.start_session().await.unwrap();
    svc.end_session(&s1.id, "Done").await.unwrap();
    let (_s2, _) = svc.start_session().await.unwrap();

    let active = svc
        .list_sessions(Some(SessionStatus::Active), 10)
        .await
        .unwrap();
    assert_eq!(active.len(), 1);

    let wrapped = svc
        .list_sessions(Some(SessionStatus::WrappedUp), 10)
        .await
        .unwrap();
    assert_eq!(wrapped.len(), 1);

    let all = svc.list_sessions(None, 10).await.unwrap();
    assert_eq!(all.len(), 2);
}

#[tokio::test]
async fn session_snapshot_aggregates() {
    let svc = test_service().await;

    svc.db()
        .conn()
        .execute(
            "INSERT INTO sessions (id, status) VALUES ('ses-snap', 'active')",
            (),
        )
        .await
        .unwrap();

    svc.db()
        .conn()
        .execute(
            "INSERT INTO tasks (id, session_id, title, status) VALUES ('tsk-1', 'ses-snap', 'Task 1', 'open')",
            (),
        )
        .await
        .unwrap();
    svc.db()
        .conn()
        .execute(
            "INSERT INTO tasks (id, session_id, title, status) VALUES ('tsk-2', 'ses-snap', 'Task 2', 'in_progress')",
            (),
        )
        .await
        .unwrap();
    svc.db()
        .conn()
        .execute(
            "INSERT INTO hypotheses (id, session_id, content, status) VALUES ('hyp-1', 'ses-snap', 'Hyp 1', 'unverified')",
            (),
        )
        .await
        .unwrap();

    let snapshot = svc
        .create_snapshot("ses-snap", "Test snapshot")
        .await
        .unwrap();
    assert_eq!(snapshot.open_tasks, 1);
    assert_eq!(snapshot.in_progress_tasks, 1);
    assert_eq!(snapshot.pending_hypotheses, 1);
    assert_eq!(snapshot.summary, "Test snapshot");
}

// ---------------------------------------------------------------------------
// Trail writer tests
// ---------------------------------------------------------------------------

#[tokio::test]
async fn trail_writer_creates_file() {
    let tmp = TempDir::new().unwrap();
    let writer = TrailWriter::new(tmp.path().to_path_buf()).unwrap();

    let op = TrailOperation {
        v: 1,
        ts: Utc::now().to_rfc3339(),
        ses: "ses-00000001".to_string(),
        op: TrailOp::Create,
        entity: EntityType::Finding,
        id: "fnd-00000001".to_string(),
        data: serde_json::json!({"content": "test"}),
    };
    writer.append(&op).unwrap();

    let path = tmp.path().join("ses-00000001.jsonl");
    assert!(path.exists());
}

#[tokio::test]
async fn trail_writer_appends_valid_json() {
    let tmp = TempDir::new().unwrap();
    let writer = TrailWriter::new(tmp.path().to_path_buf()).unwrap();

    let op = TrailOperation {
        v: 1,
        ts: Utc::now().to_rfc3339(),
        ses: "ses-00000001".to_string(),
        op: TrailOp::Create,
        entity: EntityType::Finding,
        id: "fnd-00000001".to_string(),
        data: serde_json::json!({"content": "test finding"}),
    };
    writer.append(&op).unwrap();

    let path = tmp.path().join("ses-00000001.jsonl");
    let ops: Vec<TrailOperation> = serde_jsonlines::json_lines(&path)
        .unwrap()
        .collect::<std::io::Result<Vec<_>>>()
        .unwrap();
    assert_eq!(ops.len(), 1);
    assert_eq!(ops[0].id, "fnd-00000001");
    assert_eq!(ops[0].op, TrailOp::Create);
}

#[tokio::test]
async fn trail_writer_per_session_files() {
    let tmp = TempDir::new().unwrap();
    let writer = TrailWriter::new(tmp.path().to_path_buf()).unwrap();

    let op1 = TrailOperation {
        v: 1,
        ts: Utc::now().to_rfc3339(),
        ses: "ses-00000001".to_string(),
        op: TrailOp::Create,
        entity: EntityType::Finding,
        id: "fnd-00000001".to_string(),
        data: serde_json::json!({}),
    };
    let op2 = TrailOperation {
        v: 1,
        ts: Utc::now().to_rfc3339(),
        ses: "ses-00000002".to_string(),
        op: TrailOp::Create,
        entity: EntityType::Task,
        id: "tsk-00000001".to_string(),
        data: serde_json::json!({}),
    };
    writer.append(&op1).unwrap();
    writer.append(&op2).unwrap();

    assert!(tmp.path().join("ses-00000001.jsonl").exists());
    assert!(tmp.path().join("ses-00000002.jsonl").exists());
}

#[tokio::test]
async fn trail_writer_disabled_noop() {
    let tmp = TempDir::new().unwrap();
    let writer = TrailWriter::disabled();

    let op = TrailOperation {
        v: 1,
        ts: Utc::now().to_rfc3339(),
        ses: "ses-00000001".to_string(),
        op: TrailOp::Create,
        entity: EntityType::Finding,
        id: "fnd-00000001".to_string(),
        data: serde_json::json!({}),
    };
    writer.append(&op).unwrap();

    assert!(!tmp.path().join("ses-00000001.jsonl").exists());
}

#[tokio::test]
async fn trail_validation_warns_on_invalid() {
    let tmp = TempDir::new().unwrap();
    let writer = TrailWriter::new(tmp.path().to_path_buf()).unwrap();
    let schema = zen_schema::SchemaRegistry::new();

    let op = TrailOperation {
        v: 1,
        ts: Utc::now().to_rfc3339(),
        ses: "ses-00000001".to_string(),
        op: TrailOp::Create,
        entity: EntityType::Finding,
        id: "fnd-00000001".to_string(),
        data: serde_json::json!({"id": "fnd-00000001"}),
    };

    writer.append_validated(&op, &schema).unwrap();

    let path = tmp.path().join("ses-00000001.jsonl");
    assert!(path.exists());
}
