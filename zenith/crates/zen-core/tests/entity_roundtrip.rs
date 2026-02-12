//! Serde roundtrip and JsonSchema validation tests for all entity types.

use chrono::Utc;
use schemars::schema_for;
use zen_core::audit_detail::{IndexedDetail, LinkedDetail, StatusChangedDetail, TaggedDetail};
use zen_core::entities::*;
use zen_core::enums::*;
use zen_core::responses::*;
use zen_core::trail::TrailOperation;

/// Validate a JSON value against a schemars-generated schema.
fn validate_against_schema(
    schema: &serde_json::Value,
    instance: &serde_json::Value,
) -> Vec<String> {
    let validator = jsonschema::validator_for(schema).expect("schema should be valid");
    validator
        .iter_errors(instance)
        .map(|e| format!("{e}"))
        .collect()
}

macro_rules! roundtrip_and_validate {
    ($name:ident, $ty:ty, $instance:expr) => {
        #[test]
        fn $name() {
            let val: $ty = $instance;

            // Serde roundtrip
            let json_str = serde_json::to_string_pretty(&val).unwrap();
            let recovered: $ty = serde_json::from_str(&json_str).unwrap();
            assert_eq!(
                recovered,
                val,
                "serde roundtrip failed for {}",
                stringify!($ty)
            );

            // Schema validation
            let schema = serde_json::to_value(schema_for!($ty)).unwrap();
            let instance = serde_json::to_value(&val).unwrap();
            let errors = validate_against_schema(&schema, &instance);
            assert!(
                errors.is_empty(),
                "Schema validation failed for {}: {:?}",
                stringify!($ty),
                errors
            );
        }
    };
}

roundtrip_and_validate!(
    session_roundtrip,
    Session,
    Session {
        id: "ses-a3f8b2c1".into(),
        started_at: Utc::now(),
        ended_at: None,
        status: SessionStatus::Active,
        summary: None,
    }
);

roundtrip_and_validate!(
    session_snapshot_roundtrip,
    SessionSnapshot,
    SessionSnapshot {
        session_id: "ses-a3f8b2c1".into(),
        open_tasks: 5,
        in_progress_tasks: 2,
        pending_hypotheses: 3,
        unverified_hypotheses: 1,
        recent_findings: 8,
        open_research: 2,
        summary: "Progress made on HTTP client evaluation.".into(),
        created_at: Utc::now(),
    }
);

roundtrip_and_validate!(
    research_roundtrip,
    ResearchItem,
    ResearchItem {
        id: "res-c4e2d1f0".into(),
        session_id: Some("ses-a3f8b2c1".into()),
        title: "Evaluate HTTP clients".into(),
        description: Some("Compare reqwest, hyper, and ureq".into()),
        status: ResearchStatus::Open,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    }
);

roundtrip_and_validate!(
    finding_roundtrip,
    Finding,
    Finding {
        id: "fnd-b7a3f9e2".into(),
        research_id: Some("res-c4e2d1f0".into()),
        session_id: Some("ses-a3f8b2c1".into()),
        content: "reqwest supports connection pooling by default".into(),
        source: Some("package:reqwest:0.12".into()),
        confidence: Confidence::High,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    }
);

roundtrip_and_validate!(
    hypothesis_roundtrip,
    Hypothesis,
    Hypothesis {
        id: "hyp-e1c4b2d3".into(),
        research_id: None,
        finding_id: Some("fnd-b7a3f9e2".into()),
        session_id: Some("ses-a3f8b2c1".into()),
        content: "reqwest works with tower middleware".into(),
        status: HypothesisStatus::Unverified,
        reason: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    }
);

roundtrip_and_validate!(
    insight_roundtrip,
    Insight,
    Insight {
        id: "ins-d2f5a8c1".into(),
        research_id: Some("res-c4e2d1f0".into()),
        session_id: Some("ses-a3f8b2c1".into()),
        content: "reqwest is the best HTTP client for our tower stack".into(),
        confidence: Confidence::Medium,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    }
);

roundtrip_and_validate!(
    issue_roundtrip,
    Issue,
    Issue {
        id: "iss-f3b7c1e4".into(),
        issue_type: IssueType::Feature,
        parent_id: None,
        title: "Add HTTP client layer".into(),
        description: Some("Implement retry logic with reqwest".into()),
        status: IssueStatus::Open,
        priority: 2,
        session_id: Some("ses-a3f8b2c1".into()),
        created_at: Utc::now(),
        updated_at: Utc::now(),
    }
);

roundtrip_and_validate!(
    task_roundtrip,
    Task,
    Task {
        id: "tsk-a8d3e2b5".into(),
        research_id: None,
        issue_id: Some("iss-f3b7c1e4".into()),
        session_id: Some("ses-a3f8b2c1".into()),
        title: "Implement retry logic".into(),
        description: None,
        status: TaskStatus::Open,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    }
);

roundtrip_and_validate!(
    impl_log_roundtrip,
    ImplLog,
    ImplLog {
        id: "imp-c1f4b7a9".into(),
        task_id: "tsk-a8d3e2b5".into(),
        session_id: Some("ses-a3f8b2c1".into()),
        file_path: "src/http/retry.rs".into(),
        start_line: Some(45),
        end_line: Some(82),
        description: Some("Added exponential backoff retry".into()),
        created_at: Utc::now(),
    }
);

roundtrip_and_validate!(
    compat_roundtrip,
    CompatCheck,
    CompatCheck {
        id: "cmp-e5a2d9f3".into(),
        package_a: "rust:tokio:1.40.0".into(),
        package_b: "rust:axum:0.8.0".into(),
        status: CompatStatus::Compatible,
        conditions: None,
        finding_id: Some("fnd-b7a3f9e2".into()),
        session_id: Some("ses-a3f8b2c1".into()),
        created_at: Utc::now(),
        updated_at: Utc::now(),
    }
);

roundtrip_and_validate!(
    study_roundtrip,
    Study,
    Study {
        id: "stu-a1b2c3d4".into(),
        session_id: Some("ses-a3f8b2c1".into()),
        research_id: Some("res-c4e2d1f0".into()),
        topic: "How tokio::spawn works".into(),
        library: Some("tokio".into()),
        methodology: StudyMethodology::Explore,
        status: StudyStatus::Active,
        summary: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    }
);

roundtrip_and_validate!(
    entity_link_roundtrip,
    EntityLink,
    EntityLink {
        id: "lnk-b3c8f1d6".into(),
        source_type: EntityType::Finding,
        source_id: "fnd-b7a3f9e2".into(),
        target_type: EntityType::Hypothesis,
        target_id: "hyp-e1c4b2d3".into(),
        relation: Relation::Validates,
        created_at: Utc::now(),
    }
);

roundtrip_and_validate!(
    audit_entry_roundtrip,
    AuditEntry,
    AuditEntry {
        id: "aud-d7e2a4c8".into(),
        session_id: Some("ses-a3f8b2c1".into()),
        entity_type: EntityType::Finding,
        entity_id: "fnd-b7a3f9e2".into(),
        action: AuditAction::Created,
        detail: Some(serde_json::json!({"content": "reqwest supports connection pooling"})),
        created_at: Utc::now(),
    }
);

roundtrip_and_validate!(
    project_meta_roundtrip,
    ProjectMeta,
    ProjectMeta {
        key: "language".into(),
        value: "rust".into(),
        updated_at: Utc::now(),
    }
);

roundtrip_and_validate!(
    project_dep_roundtrip,
    ProjectDependency,
    ProjectDependency {
        ecosystem: "rust".into(),
        name: "tokio".into(),
        version: Some("1.49".into()),
        source: "cargo.toml".into(),
        indexed: true,
        indexed_at: Some(Utc::now()),
    }
);

// --- Trail envelope ---

roundtrip_and_validate!(
    trail_operation_roundtrip,
    TrailOperation,
    TrailOperation {
        v: 1,
        ts: "2026-02-08T12:00:00Z".into(),
        ses: "ses-a3f8b2c1".into(),
        op: TrailOp::Create,
        entity: EntityType::Finding,
        id: "fnd-deadbeef".into(),
        data: serde_json::json!({"content": "test finding", "confidence": "high"}),
    }
);

// --- Response types ---

roundtrip_and_validate!(
    finding_create_response_roundtrip,
    FindingCreateResponse,
    FindingCreateResponse {
        finding: Finding {
            id: "fnd-b7a3f9e2".into(),
            research_id: None,
            session_id: None,
            content: "test".into(),
            source: None,
            confidence: Confidence::Low,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        },
    }
);

roundtrip_and_validate!(
    rebuild_response_roundtrip,
    RebuildResponse,
    RebuildResponse {
        rebuilt: true,
        trail_files: 3,
        operations_replayed: 150,
        entities_created: 45,
        duration_ms: 230,
    }
);

roundtrip_and_validate!(
    search_result_roundtrip,
    SearchResult,
    SearchResult {
        package: "tokio".into(),
        ecosystem: "rust".into(),
        kind: "function".into(),
        name: "spawn".into(),
        signature: Some("pub fn spawn<F>(future: F) -> JoinHandle<F::Output>".into()),
        doc_comment: Some("Spawns a new asynchronous task.".into()),
        file_path: Some("src/task/spawn.rs".into()),
        line_start: Some(42),
        score: 0.95,
    }
);

roundtrip_and_validate!(
    search_result_member_kind_roundtrip,
    SearchResult,
    SearchResult {
        package: "react".into(),
        ecosystem: "npm".into(),
        kind: "property".into(),
        name: "Component::props".into(),
        signature: Some("readonly props: Props".into()),
        doc_comment: Some("Component properties.".into()),
        file_path: Some("src/component.tsx".into()),
        line_start: Some(12),
        score: 0.84,
    }
);

// --- Audit detail types ---

roundtrip_and_validate!(
    status_changed_detail_roundtrip,
    StatusChangedDetail,
    StatusChangedDetail {
        from: "unverified".into(),
        to: "confirmed".into(),
        reason: Some("Spike validated compatibility".into()),
    }
);

roundtrip_and_validate!(
    linked_detail_roundtrip,
    LinkedDetail,
    LinkedDetail {
        source_type: "finding".into(),
        source_id: "fnd-b7a3f9e2".into(),
        target_type: "hypothesis".into(),
        target_id: "hyp-e1c4b2d3".into(),
        relation: "validates".into(),
    }
);

roundtrip_and_validate!(
    tagged_detail_roundtrip,
    TaggedDetail,
    TaggedDetail {
        tag: "verified".into(),
    }
);

roundtrip_and_validate!(
    indexed_detail_roundtrip,
    IndexedDetail,
    IndexedDetail {
        package: "tokio".into(),
        ecosystem: "rust".into(),
        symbols: 1250,
        duration_ms: 3400,
    }
);

// --- Schema rejection test ---

#[test]
fn schema_rejects_invalid_finding() {
    let schema = serde_json::to_value(schema_for!(Finding)).unwrap();
    // Missing required "content" field
    let invalid = serde_json::json!({
        "id": "fnd-test",
        "confidence": "high",
        "created_at": "2026-02-08T12:00:00Z",
        "updated_at": "2026-02-08T12:00:00Z"
    });
    let errors = validate_against_schema(&schema, &invalid);
    assert!(
        !errors.is_empty(),
        "Should reject finding without 'content'"
    );
}

#[test]
fn schema_rejects_invalid_enum_value() {
    let schema = serde_json::to_value(schema_for!(Finding)).unwrap();
    let invalid = serde_json::json!({
        "id": "fnd-test",
        "content": "test",
        "confidence": "super_high",  // invalid enum variant
        "created_at": "2026-02-08T12:00:00Z",
        "updated_at": "2026-02-08T12:00:00Z"
    });
    let errors = validate_against_schema(&schema, &invalid);
    assert!(!errors.is_empty(), "Should reject invalid confidence value");
}
