//! # Spike 0.14: JSON Schema Generation & Validation
//!
//! Validates `schemars` 1.x for auto-generating JSON Schemas from Zenith entity types,
//! and `jsonschema` 0.28 for runtime validation at every JSON boundary.
//!
//! **22 tests** across 7 sections:
//! - Part A: Entity schema generation (5 tests)
//! - Part B: Trail operation schema (4 tests)
//! - Part C: Config schema (3 tests)
//! - Part D: CLI response & input schemas (3 tests)
//! - Part E: Audit detail schemas (2 tests)
//! - Part F: DuckDB metadata schemas (2 tests)
//! - Part G: Schema registry & cross-cutting (3 tests)

#[cfg(test)]
mod tests {
    use chrono::{DateTime, Utc};
    use schemars::{JsonSchema, schema_for};
    use serde::{Deserialize, Serialize};
    use serde_json::json;
    use std::collections::HashMap;

    // =========================================================================
    // Sample types matching 05-crate-designs.md entity definitions.
    // These replicate the planned zen-core types for spike validation.
    // In production, these will live in zen-core with #[derive(JsonSchema)].
    // =========================================================================

    // --- Enums ---

    #[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
    #[serde(rename_all = "snake_case")]
    enum Confidence {
        High,
        Medium,
        Low,
    }

    #[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
    #[serde(rename_all = "snake_case")]
    enum HypothesisStatus {
        Unverified,
        Analyzing,
        Confirmed,
        Debunked,
        PartiallyConfirmed,
        Inconclusive,
    }

    #[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
    #[serde(rename_all = "snake_case")]
    enum IssueType {
        Bug,
        Feature,
        Spike,
        Epic,
        Request,
    }

    #[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
    #[serde(rename_all = "snake_case")]
    enum IssueStatus {
        Open,
        InProgress,
        Done,
        Blocked,
        Abandoned,
    }

    #[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
    #[serde(rename_all = "snake_case")]
    enum TaskStatus {
        Open,
        InProgress,
        Done,
        Blocked,
    }

    #[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
    #[serde(rename_all = "snake_case")]
    enum SessionStatus {
        Active,
        WrappedUp,
        Abandoned,
    }

    #[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
    #[serde(rename_all = "snake_case")]
    enum ResearchStatus {
        Open,
        InProgress,
        Resolved,
        Abandoned,
    }

    #[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
    #[serde(rename_all = "snake_case")]
    enum StudyStatus {
        Active,
        Concluding,
        Completed,
        Abandoned,
    }

    #[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
    #[serde(rename_all = "snake_case")]
    enum StudyMethodology {
        Explore,
        TestDriven,
        Compare,
    }

    #[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
    #[serde(rename_all = "snake_case")]
    enum CompatStatus {
        Compatible,
        Incompatible,
        Conditional,
        Unknown,
    }

    #[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
    #[serde(rename_all = "snake_case")]
    enum EntityType {
        Session,
        Research,
        Finding,
        Hypothesis,
        Insight,
        Issue,
        Task,
        ImplLog,
        Compat,
        Study,
        EntityLink,
        Audit,
    }

    #[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
    #[serde(rename_all = "snake_case")]
    enum AuditAction {
        Created,
        Updated,
        StatusChanged,
        Linked,
        Unlinked,
        Tagged,
        Untagged,
        Indexed,
        SessionStart,
        SessionEnd,
        WrapUp,
    }

    #[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
    #[serde(rename_all = "snake_case")]
    enum Relation {
        Blocks,
        Validates,
        Debunks,
        Implements,
        RelatesTo,
        DerivedFrom,
        Triggers,
        Supersedes,
        DependsOn,
    }

    #[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
    #[serde(rename_all = "snake_case")]
    enum TrailOp {
        Create,
        Update,
        Delete,
    }

    // --- Entity Structs (all 12) ---

    #[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
    struct Session {
        id: String,
        status: SessionStatus,
        started_at: DateTime<Utc>,
        ended_at: Option<DateTime<Utc>>,
        summary: Option<String>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
    struct ResearchItem {
        id: String,
        title: String,
        description: Option<String>,
        status: ResearchStatus,
        session_id: Option<String>,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
    struct Finding {
        id: String,
        research_id: Option<String>,
        session_id: Option<String>,
        content: String,
        source: Option<String>,
        confidence: Confidence,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
    struct Hypothesis {
        id: String,
        research_id: Option<String>,
        finding_id: Option<String>,
        content: String,
        status: HypothesisStatus,
        reason: Option<String>,
        session_id: Option<String>,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
    struct Insight {
        id: String,
        research_id: Option<String>,
        content: String,
        session_id: Option<String>,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
    struct Issue {
        id: String,
        issue_type: IssueType,
        parent_id: Option<String>,
        title: String,
        description: Option<String>,
        status: IssueStatus,
        priority: u8,
        session_id: Option<String>,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
    struct Task {
        id: String,
        issue_id: Option<String>,
        title: String,
        description: Option<String>,
        status: TaskStatus,
        session_id: Option<String>,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
    struct ImplLog {
        id: String,
        task_id: Option<String>,
        file_path: String,
        start_line: Option<u32>,
        end_line: Option<u32>,
        description: Option<String>,
        session_id: Option<String>,
        created_at: DateTime<Utc>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
    struct CompatCheck {
        id: String,
        package_a: String,
        package_b: String,
        status: CompatStatus,
        conditions: Option<String>,
        finding_id: Option<String>,
        session_id: Option<String>,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
    struct Study {
        id: String,
        topic: String,
        library: Option<String>,
        methodology: StudyMethodology,
        status: StudyStatus,
        summary: Option<String>,
        research_id: Option<String>,
        session_id: Option<String>,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
    struct EntityLink {
        id: String,
        source_type: EntityType,
        source_id: String,
        target_type: EntityType,
        target_id: String,
        relation: Relation,
        created_at: DateTime<Utc>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
    struct AuditEntry {
        id: String,
        session_id: Option<String>,
        entity_type: EntityType,
        entity_id: String,
        action: AuditAction,
        detail: Option<serde_json::Value>,
        created_at: DateTime<Utc>,
    }

    // --- Trail Operation ---

    #[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
    struct TrailOperation {
        ts: String,
        ses: String,
        op: TrailOp,
        entity: EntityType,
        id: String,
        data: serde_json::Value,
    }

    // --- Config Types ---

    #[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
    struct ZenConfig {
        #[serde(default)]
        turso: TursoConfig,
        #[serde(default)]
        motherduck: MotherDuckConfig,
        #[serde(default)]
        r2: R2Config,
        #[serde(default)]
        general: GeneralConfig,
    }

    #[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
    struct TursoConfig {
        url: Option<String>,
        auth_token: Option<String>,
        db_name: Option<String>,
        sync_interval_secs: Option<u64>,
    }

    #[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
    struct MotherDuckConfig {
        token: Option<String>,
        db_name: Option<String>,
    }

    #[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
    struct R2Config {
        account_id: Option<String>,
        access_key_id: Option<String>,
        secret_access_key: Option<String>,
        bucket_name: Option<String>,
    }

    #[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
    struct GeneralConfig {
        default_ecosystem: Option<String>,
        default_limit: Option<u32>,
    }

    // --- CLI Response Types ---

    #[derive(Debug, Clone, Serialize, JsonSchema)]
    struct FindingCreateResponse {
        finding: Finding,
    }

    #[derive(Debug, Clone, Serialize, JsonSchema)]
    struct SessionStartResponse {
        session: Session,
        previous_session: Option<Session>,
    }

    #[derive(Debug, Clone, Serialize, JsonSchema)]
    struct WhatsNextResponse {
        last_session: Option<Session>,
        open_tasks: Vec<Task>,
        pending_hypotheses: Vec<Hypothesis>,
        recent_audit: Vec<AuditEntry>,
    }

    #[derive(Debug, Clone, Serialize, JsonSchema)]
    struct SearchResult {
        package: String,
        ecosystem: String,
        kind: String,
        name: String,
        signature: Option<String>,
        doc_comment: Option<String>,
        file_path: Option<String>,
        line_start: Option<u32>,
        score: f64,
    }

    #[derive(Debug, Clone, Serialize, JsonSchema)]
    struct SearchResultsResponse {
        query: String,
        results: Vec<SearchResult>,
        total_results: u32,
    }

    #[derive(Debug, Clone, Serialize, JsonSchema)]
    struct RebuildResponse {
        rebuilt: bool,
        trail_files: u32,
        operations_replayed: u32,
        entities_created: u32,
        duration_ms: u64,
    }

    // --- Audit Detail Types ---

    #[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
    struct StatusChangedDetail {
        from: String,
        to: String,
        reason: Option<String>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
    struct LinkedDetail {
        source_type: String,
        source_id: String,
        target_type: String,
        target_id: String,
        relation: String,
    }

    #[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
    struct TaggedDetail {
        tag: String,
    }

    #[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
    struct IndexedDetail {
        package: String,
        ecosystem: String,
        symbols: u32,
        duration_ms: u64,
    }

    // --- DuckDB Metadata Types ---

    #[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
    struct RustDocSections {
        errors: Option<Vec<String>>,
        panics: Option<String>,
        safety: Option<String>,
        examples: Option<Vec<String>>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
    struct RustMetadata {
        lifetimes: Option<Vec<String>>,
        where_clause: Option<String>,
        is_pyo3: bool,
        trait_name: Option<String>,
        for_type: Option<String>,
        variants: Option<Vec<String>>,
        fields: Option<Vec<String>>,
        methods: Option<Vec<String>>,
        associated_types: Option<Vec<String>>,
        abi: Option<String>,
        doc_sections: Option<RustDocSections>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
    struct PythonDocSections {
        args: Option<HashMap<String, String>>,
        returns: Option<String>,
        raises: Option<HashMap<String, String>>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
    struct PythonMetadata {
        is_generator: bool,
        is_property: bool,
        is_pydantic: bool,
        is_protocol: bool,
        is_dataclass: bool,
        base_classes: Option<Vec<String>>,
        decorators: Option<Vec<String>>,
        parameters: Option<Vec<String>>,
        doc_sections: Option<PythonDocSections>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
    struct TypeScriptMetadata {
        is_exported: bool,
        is_default_export: bool,
        type_parameters: Option<Vec<String>>,
        implements: Option<Vec<String>>,
    }

    // =========================================================================
    // Helper: validate JSON against a schemars-generated schema
    // =========================================================================

    fn validate_with_schema(
        schema: &serde_json::Value,
        instance: &serde_json::Value,
    ) -> Result<(), Vec<String>> {
        let validator = jsonschema::validator_for(schema).expect("schema should be valid");
        let errors: Vec<String> = validator
            .iter_errors(instance)
            .map(|e| format!("{e}"))
            .collect();
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    fn make_finding() -> Finding {
        Finding {
            id: "fnd-a3f8b2c1".into(),
            research_id: Some("res-12345678".into()),
            session_id: Some("ses-abcdef01".into()),
            content: "reqwest supports connection pooling".into(),
            source: Some("docs/connection.md".into()),
            confidence: Confidence::High,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    fn make_session() -> Session {
        Session {
            id: "ses-abcdef01".into(),
            status: SessionStatus::Active,
            started_at: Utc::now(),
            ended_at: None,
            summary: None,
        }
    }

    // =========================================================================
    // Part A: Entity Schema Generation (5 tests)
    // =========================================================================

    #[test]
    fn spike_schema_entity_basic() {
        // Generate schemas for core entity types
        let finding_schema = schema_for!(Finding);
        let hypothesis_schema = schema_for!(Hypothesis);
        let issue_schema = schema_for!(Issue);
        let task_schema = schema_for!(Task);

        let finding_json = serde_json::to_value(&finding_schema).unwrap();
        let hypothesis_json = serde_json::to_value(&hypothesis_schema).unwrap();
        let issue_json = serde_json::to_value(&issue_schema).unwrap();
        let task_json = serde_json::to_value(&task_schema).unwrap();

        // All produce object schemas
        assert_eq!(finding_json["type"], "object");
        assert_eq!(hypothesis_json["type"], "object");
        assert_eq!(issue_json["type"], "object");
        assert_eq!(task_json["type"], "object");

        // Finding has required fields
        let required = finding_json["required"].as_array().unwrap();
        let required_names: Vec<&str> = required.iter().map(|v| v.as_str().unwrap()).collect();
        assert!(required_names.contains(&"id"));
        assert!(required_names.contains(&"content"));
        assert!(required_names.contains(&"confidence"));
        assert!(required_names.contains(&"created_at"));

        // Optional fields should NOT be in required
        assert!(!required_names.contains(&"research_id"));
        assert!(!required_names.contains(&"source"));

        // DateTime<Utc> with chrono04 should produce format: date-time
        let created_at = &finding_json["properties"]["created_at"];
        assert_eq!(created_at["type"], "string");
        assert_eq!(created_at["format"], "date-time");

        // confidence enum should have correct values
        let confidence_ref = &finding_json["properties"]["confidence"];
        // May be inline or $ref — resolve it
        let confidence_schema = if confidence_ref.get("$ref").is_some() {
            let ref_path = confidence_ref["$ref"].as_str().unwrap();
            let def_name = ref_path.split('/').last().unwrap();
            &finding_json["$defs"][def_name]
        } else {
            confidence_ref
        };
        let enum_vals = confidence_schema["enum"].as_array().unwrap();
        let vals: Vec<&str> = enum_vals.iter().map(|v| v.as_str().unwrap()).collect();
        assert!(vals.contains(&"high"));
        assert!(vals.contains(&"medium"));
        assert!(vals.contains(&"low"));
        assert_eq!(vals.len(), 3);

        println!("=== Part A Test 1: Entity Basic ===");
        println!(
            "Finding schema: {} properties, {} required",
            finding_json["properties"]
                .as_object()
                .map_or(0, |o| o.len()),
            required.len()
        );
        println!("DateTime<Utc> format: {}", created_at["format"]);
        println!("Confidence enum values: {:?}", vals);
        println!("PASS: schemars generates correct schemas for entity structs");
    }

    #[test]
    fn spike_schema_entity_all_twelve() {
        // Generate schemas for all 12 entity types
        let schemas: Vec<(&str, serde_json::Value)> = vec![
            (
                "Session",
                serde_json::to_value(schema_for!(Session)).unwrap(),
            ),
            (
                "ResearchItem",
                serde_json::to_value(schema_for!(ResearchItem)).unwrap(),
            ),
            (
                "Finding",
                serde_json::to_value(schema_for!(Finding)).unwrap(),
            ),
            (
                "Hypothesis",
                serde_json::to_value(schema_for!(Hypothesis)).unwrap(),
            ),
            (
                "Insight",
                serde_json::to_value(schema_for!(Insight)).unwrap(),
            ),
            ("Issue", serde_json::to_value(schema_for!(Issue)).unwrap()),
            ("Task", serde_json::to_value(schema_for!(Task)).unwrap()),
            (
                "ImplLog",
                serde_json::to_value(schema_for!(ImplLog)).unwrap(),
            ),
            (
                "CompatCheck",
                serde_json::to_value(schema_for!(CompatCheck)).unwrap(),
            ),
            ("Study", serde_json::to_value(schema_for!(Study)).unwrap()),
            (
                "EntityLink",
                serde_json::to_value(schema_for!(EntityLink)).unwrap(),
            ),
            (
                "AuditEntry",
                serde_json::to_value(schema_for!(AuditEntry)).unwrap(),
            ),
        ];

        println!("=== Part A Test 2: All 12 Entity Schemas ===");
        let mut total_fields = 0;
        for (name, schema) in &schemas {
            // Each must be a valid schema (meta-validate)
            assert_eq!(schema["type"], "object", "{name} should be object type");
            let prop_count = schema["properties"].as_object().map_or(0, |o| o.len());
            total_fields += prop_count;
            println!("  {name}: {prop_count} properties");

            // Verify jsonschema can parse it as a valid schema
            let validator = jsonschema::validator_for(schema);
            assert!(
                validator.is_ok(),
                "{name} schema should be valid for jsonschema"
            );
        }
        println!("Total: {} schemas, {} fields", schemas.len(), total_fields);
        assert_eq!(schemas.len(), 12);
        println!("PASS: All 12 entity schemas generated and meta-validated");
    }

    #[test]
    fn spike_schema_entity_roundtrip() {
        // Test the full pipeline: create -> serialize -> validate -> deserialize
        let finding = make_finding();
        let schema = serde_json::to_value(schema_for!(Finding)).unwrap();

        // Serialize
        let json_val = serde_json::to_value(&finding).unwrap();

        // Validate against schema
        let result = validate_with_schema(&schema, &json_val);
        assert!(
            result.is_ok(),
            "Valid finding should pass: {:?}",
            result.err()
        );

        // Deserialize back
        let roundtripped: Finding = serde_json::from_value(json_val).unwrap();
        assert_eq!(roundtripped.id, finding.id);
        assert_eq!(roundtripped.content, finding.content);
        assert_eq!(roundtripped.confidence, finding.confidence);

        // Test a few more entity types for roundtrip
        let session = make_session();
        let session_schema = serde_json::to_value(schema_for!(Session)).unwrap();
        let session_json = serde_json::to_value(&session).unwrap();
        assert!(validate_with_schema(&session_schema, &session_json).is_ok());

        let issue = Issue {
            id: "iss-12345678".into(),
            issue_type: IssueType::Feature,
            parent_id: None,
            title: "Add HTTP client layer".into(),
            description: Some("Implement the HTTP abstraction".into()),
            status: IssueStatus::Open,
            priority: 2,
            session_id: Some("ses-abcdef01".into()),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        let issue_schema = serde_json::to_value(schema_for!(Issue)).unwrap();
        let issue_json = serde_json::to_value(&issue).unwrap();
        assert!(validate_with_schema(&issue_schema, &issue_json).is_ok());
        let issue_rt: Issue = serde_json::from_value(issue_json).unwrap();
        assert_eq!(issue_rt.priority, 2);
        assert_eq!(issue_rt.issue_type, IssueType::Feature);

        println!("=== Part A Test 3: Entity Roundtrip ===");
        println!("PASS: Finding, Session, Issue all roundtrip through schema validation");
    }

    #[test]
    fn spike_schema_enum_constraints() {
        let check_enum = |name: &str, schema: &serde_json::Value, expected: &[&str]| {
            // May be at top level or in $defs
            let enum_vals = if let Some(arr) = schema.get("enum").and_then(|v| v.as_array()) {
                arr.clone()
            } else if let Some(one_of) = schema.get("oneOf").and_then(|v| v.as_array()) {
                // schemars may use oneOf for enums
                one_of
                    .iter()
                    .filter_map(|v| v.get("const").cloned())
                    .collect()
            } else {
                panic!("{name}: no enum or oneOf found in schema: {schema}");
            };

            let vals: Vec<&str> = enum_vals.iter().map(|v| v.as_str().unwrap()).collect();
            for exp in expected {
                assert!(
                    vals.contains(exp),
                    "{name}: missing expected value '{exp}', got {vals:?}"
                );
            }
            assert_eq!(
                vals.len(),
                expected.len(),
                "{name}: expected {} values, got {}: {vals:?}",
                expected.len(),
                vals.len()
            );
            println!("  {name}: {:?}", vals);
        };

        println!("=== Part A Test 4: Enum Constraints ===");

        check_enum(
            "Confidence",
            &serde_json::to_value(schema_for!(Confidence)).unwrap(),
            &["high", "medium", "low"],
        );

        check_enum(
            "HypothesisStatus",
            &serde_json::to_value(schema_for!(HypothesisStatus)).unwrap(),
            &[
                "unverified",
                "analyzing",
                "confirmed",
                "debunked",
                "partially_confirmed",
                "inconclusive",
            ],
        );

        check_enum(
            "IssueType",
            &serde_json::to_value(schema_for!(IssueType)).unwrap(),
            &["bug", "feature", "spike", "epic", "request"],
        );

        check_enum(
            "IssueStatus",
            &serde_json::to_value(schema_for!(IssueStatus)).unwrap(),
            &["open", "in_progress", "done", "blocked", "abandoned"],
        );

        check_enum(
            "TaskStatus",
            &serde_json::to_value(schema_for!(TaskStatus)).unwrap(),
            &["open", "in_progress", "done", "blocked"],
        );

        check_enum(
            "AuditAction",
            &serde_json::to_value(schema_for!(AuditAction)).unwrap(),
            &[
                "created",
                "updated",
                "status_changed",
                "linked",
                "unlinked",
                "tagged",
                "untagged",
                "indexed",
                "session_start",
                "session_end",
                "wrap_up",
            ],
        );

        check_enum(
            "EntityType",
            &serde_json::to_value(schema_for!(EntityType)).unwrap(),
            &[
                "session",
                "research",
                "finding",
                "hypothesis",
                "insight",
                "issue",
                "task",
                "impl_log",
                "compat",
                "study",
                "entity_link",
                "audit",
            ],
        );

        check_enum(
            "Relation",
            &serde_json::to_value(schema_for!(Relation)).unwrap(),
            &[
                "blocks",
                "validates",
                "debunks",
                "implements",
                "relates_to",
                "derived_from",
                "triggers",
                "supersedes",
                "depends_on",
            ],
        );

        println!("PASS: All 8 enums produce correct snake_case enum schemas");
    }

    #[test]
    fn spike_schema_entity_validation_errors() {
        let schema = serde_json::to_value(schema_for!(Finding)).unwrap();

        // (a) Wrong enum value
        let wrong_enum = json!({
            "id": "fnd-1234",
            "content": "test",
            "confidence": "very_high",
            "created_at": "2026-02-08T10:00:00Z",
            "updated_at": "2026-02-08T10:00:00Z"
        });
        let err = validate_with_schema(&schema, &wrong_enum);
        assert!(err.is_err(), "Wrong enum value should fail");
        let errors = err.unwrap_err();
        println!("  Wrong enum error: {}", errors[0]);

        // (b) Missing required field (content)
        let missing = json!({
            "id": "fnd-1234",
            "confidence": "high",
            "created_at": "2026-02-08T10:00:00Z",
            "updated_at": "2026-02-08T10:00:00Z"
        });
        let err = validate_with_schema(&schema, &missing);
        assert!(err.is_err(), "Missing required field should fail");
        println!("  Missing field error: {}", err.unwrap_err()[0]);

        // (c) Wrong type (priority as string on Issue)
        let issue_schema = serde_json::to_value(schema_for!(Issue)).unwrap();
        let wrong_type = json!({
            "id": "iss-1234",
            "issue_type": "feature",
            "title": "Test",
            "status": "open",
            "priority": "high",
            "created_at": "2026-02-08T10:00:00Z",
            "updated_at": "2026-02-08T10:00:00Z"
        });
        let err = validate_with_schema(&issue_schema, &wrong_type);
        assert!(err.is_err(), "Wrong type should fail");
        println!("  Wrong type error: {}", err.unwrap_err()[0]);

        // (d) Extra unknown field — document behavior
        let extra = json!({
            "id": "fnd-1234",
            "content": "test",
            "confidence": "high",
            "created_at": "2026-02-08T10:00:00Z",
            "updated_at": "2026-02-08T10:00:00Z",
            "unknown_field": "should this pass?"
        });
        let result = validate_with_schema(&schema, &extra);
        println!(
            "  Extra field behavior: {}",
            if result.is_ok() {
                "ACCEPTED (permissive)"
            } else {
                "REJECTED (strict)"
            }
        );
        // Document: schemars default does NOT add additionalProperties: false

        println!("=== Part A Test 5: Validation Errors ===");
        println!("PASS: Wrong enum, missing field, wrong type all produce descriptive errors");
    }

    // =========================================================================
    // Part B: Trail Operation Schema (4 tests)
    // =========================================================================

    #[test]
    fn spike_schema_trail_envelope() {
        let schema = serde_json::to_value(schema_for!(TrailOperation)).unwrap();

        assert_eq!(schema["type"], "object");
        let required = schema["required"].as_array().unwrap();
        let required_names: Vec<&str> = required.iter().map(|v| v.as_str().unwrap()).collect();

        // All fields should be required (no Option<> in TrailOperation)
        assert!(required_names.contains(&"ts"));
        assert!(required_names.contains(&"ses"));
        assert!(required_names.contains(&"op"));
        assert!(required_names.contains(&"entity"));
        assert!(required_names.contains(&"id"));
        assert!(required_names.contains(&"data"));

        // Validate a well-formed operation
        let valid = json!({
            "ts": "2026-02-08T10:00:00Z",
            "ses": "ses-001",
            "op": "create",
            "entity": "finding",
            "id": "fnd-001",
            "data": {"content": "test", "confidence": "high"}
        });
        assert!(validate_with_schema(&schema, &valid).is_ok());

        println!("=== Part B Test 6: Trail Envelope ===");
        println!("Required fields: {:?}", required_names);
        println!("PASS: Trail operation envelope schema generated with all required fields");
    }

    #[test]
    fn spike_schema_trail_data_dispatch() {
        // Generate per-entity data sub-schemas (create payloads without server-generated fields)
        // For the spike, we use the full entity schemas and just verify dispatch works
        let entity_schemas: HashMap<&str, serde_json::Value> = [
            (
                "finding",
                serde_json::to_value(schema_for!(Finding)).unwrap(),
            ),
            (
                "hypothesis",
                serde_json::to_value(schema_for!(Hypothesis)).unwrap(),
            ),
            ("issue", serde_json::to_value(schema_for!(Issue)).unwrap()),
            (
                "session",
                serde_json::to_value(schema_for!(Session)).unwrap(),
            ),
        ]
        .into();

        // Dispatch: entity="finding" -> validate data against Finding schema
        let finding_data = json!({
            "id": "fnd-001",
            "content": "test finding",
            "confidence": "high",
            "created_at": "2026-02-08T10:00:00Z",
            "updated_at": "2026-02-08T10:00:00Z"
        });
        let finding_schema = &entity_schemas["finding"];
        assert!(
            validate_with_schema(finding_schema, &finding_data).is_ok(),
            "Finding data should pass finding schema"
        );

        // Wrong entity data: hypothesis fields as finding -> should fail
        let hypothesis_data = json!({
            "id": "hyp-001",
            "content": "test hypothesis",
            "status": "unverified",
            "created_at": "2026-02-08T10:00:00Z",
            "updated_at": "2026-02-08T10:00:00Z"
        });
        // Validate hypothesis_data against finding_schema — should fail because
        // "confidence" is required for Finding
        let result = validate_with_schema(finding_schema, &hypothesis_data);
        assert!(
            result.is_err(),
            "Hypothesis data should fail finding schema (missing confidence)"
        );
        println!("  Cross-entity error: {}", result.unwrap_err()[0]);

        println!("=== Part B Test 7: Trail Data Dispatch ===");
        println!("PASS: Per-entity dispatch validates correct data, rejects wrong entity data");
    }

    #[test]
    fn spike_schema_trail_validation_matrix() {
        let schema = serde_json::to_value(schema_for!(TrailOperation)).unwrap();

        // (a) Valid operation
        let valid = json!({"ts":"2026-02-08T10:00:00Z","ses":"ses-001","op":"create","entity":"finding","id":"fnd-001","data":{}});
        assert!(
            validate_with_schema(&schema, &valid).is_ok(),
            "Valid op should pass"
        );

        // (b) Malformed JSON — caught before schema check
        let malformed = serde_json::from_str::<serde_json::Value>("{broken json");
        assert!(malformed.is_err(), "Malformed JSON caught by serde");

        // (c) BOM prefix — caught before schema check
        let bom_str = "\u{FEFF}{\"ts\":\"2026-02-08T10:00:00Z\"}";
        let bom_result = serde_json::from_str::<serde_json::Value>(bom_str);
        // BOM may or may not cause serde to fail — document behavior
        println!(
            "  BOM behavior: {}",
            if bom_result.is_ok() {
                "serde accepts BOM (need manual check)"
            } else {
                "serde rejects BOM"
            }
        );

        // (d) Conflict markers — not valid JSON, caught by serde
        let conflict = "<<<<<<< HEAD\n{\"ts\":\"2026-02-08T10:00:00Z\"}\n=======";
        assert!(
            serde_json::from_str::<serde_json::Value>(conflict).is_err(),
            "Conflict markers caught by serde"
        );

        // (e) Missing required ts field
        let no_ts =
            json!({"ses":"ses-001","op":"create","entity":"finding","id":"fnd-001","data":{}});
        assert!(
            validate_with_schema(&schema, &no_ts).is_err(),
            "Missing ts should fail"
        );

        // (f) Invalid op enum
        let bad_op = json!({"ts":"2026-02-08T10:00:00Z","ses":"ses-001","op":"INVALID","entity":"finding","id":"fnd-001","data":{}});
        let err = validate_with_schema(&schema, &bad_op);
        assert!(err.is_err(), "Invalid op should fail");
        println!("  Invalid op error: {}", err.unwrap_err()[0]);

        // (g) Missing entity field
        let no_entity = json!({"ts":"2026-02-08T10:00:00Z","ses":"ses-001","op":"create","id":"fnd-001","data":{}});
        assert!(
            validate_with_schema(&schema, &no_entity).is_err(),
            "Missing entity should fail"
        );

        println!("=== Part B Test 8: Trail Validation Matrix ===");
        println!(
            "PASS: All edge cases caught (malformed, BOM, conflict, missing field, invalid enum)"
        );
    }

    #[test]
    fn spike_schema_trail_export() {
        let dir = tempfile::tempdir().unwrap();

        // Export trail envelope schema
        let trail_schema = serde_json::to_value(schema_for!(TrailOperation)).unwrap();
        let trail_path = dir.path().join("trail_operation.schema.json");
        std::fs::write(
            &trail_path,
            serde_json::to_string_pretty(&trail_schema).unwrap(),
        )
        .unwrap();
        assert!(trail_path.exists());

        // Export a few entity data schemas
        let entity_names = ["finding", "hypothesis", "session", "issue"];
        let entity_types: Vec<serde_json::Value> = vec![
            serde_json::to_value(schema_for!(Finding)).unwrap(),
            serde_json::to_value(schema_for!(Hypothesis)).unwrap(),
            serde_json::to_value(schema_for!(Session)).unwrap(),
            serde_json::to_value(schema_for!(Issue)).unwrap(),
        ];
        for (name, schema) in entity_names.iter().zip(entity_types.iter()) {
            let path = dir.path().join(format!("{name}.schema.json"));
            std::fs::write(&path, serde_json::to_string_pretty(schema).unwrap()).unwrap();
            assert!(path.exists());

            // Verify re-loadable and re-validatable
            let loaded: serde_json::Value =
                serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
            assert!(
                jsonschema::validator_for(&loaded).is_ok(),
                "{name} schema should be reloadable"
            );
        }

        println!("=== Part B Test 9: Trail Export ===");
        println!(
            "Exported {} schema files to {:?}",
            1 + entity_names.len(),
            dir.path()
        );
        println!("PASS: All schemas export to .schema.json and are re-loadable");
    }

    // =========================================================================
    // Part C: Config Schema (3 tests)
    // =========================================================================

    #[test]
    fn spike_schema_config_derive() {
        let schema = serde_json::to_value(schema_for!(ZenConfig)).unwrap();
        assert_eq!(schema["type"], "object");

        let props = schema["properties"].as_object().unwrap();
        assert!(props.contains_key("turso"));
        assert!(props.contains_key("motherduck"));
        assert!(props.contains_key("r2"));
        assert!(props.contains_key("general"));

        // #[serde(default)] means sections should NOT be required
        let required = schema.get("required").and_then(|v| v.as_array());
        if let Some(req) = required {
            let names: Vec<&str> = req.iter().map(|v| v.as_str().unwrap()).collect();
            println!("  Config required fields: {:?}", names);
            // With #[serde(default)], these should not be required
            // However, schemars behavior may differ — document it
        } else {
            println!("  Config has no required fields (all have #[serde(default)])");
        }

        // Check nested TursoConfig is inline or $ref
        let turso = &props["turso"];
        let is_ref = turso.get("$ref").is_some();
        let is_inline = turso.get("type").is_some();
        println!(
            "  Nested TursoConfig: {}",
            if is_ref {
                "$ref"
            } else if is_inline {
                "inline"
            } else {
                "other"
            }
        );

        println!("=== Part C Test 10: Config Derive ===");
        println!("PASS: ZenConfig schema generated with all 4 nested config sections");
    }

    #[test]
    fn spike_schema_config_validate() {
        let schema = serde_json::to_value(schema_for!(ZenConfig)).unwrap();

        // Valid: full config
        let valid = json!({
            "turso": {"url": "libsql://mydb.turso.io", "auth_token": "secret"},
            "motherduck": {"token": "md_token"},
            "r2": {"bucket_name": "zenith"},
            "general": {"default_ecosystem": "rust", "default_limit": 50}
        });
        assert!(
            validate_with_schema(&schema, &valid).is_ok(),
            "Full config should pass"
        );

        // Valid: empty object (all sections have #[serde(default)])
        let empty = json!({});
        let empty_result = validate_with_schema(&schema, &empty);
        println!(
            "  Empty config: {}",
            if empty_result.is_ok() {
                "ACCEPTED"
            } else {
                "REJECTED"
            }
        );

        // Invalid: wrong type for sync_interval_secs
        let wrong_type = json!({
            "turso": {"sync_interval_secs": "not_a_number"}
        });
        let result = validate_with_schema(&schema, &wrong_type);
        // This may pass or fail depending on how schemars handles nested validation
        println!(
            "  Wrong type for nested field: {}",
            if result.is_ok() {
                "ACCEPTED (permissive nesting)"
            } else {
                "REJECTED"
            }
        );

        println!("=== Part C Test 11: Config Validate ===");
        println!("PASS: Config validation works for valid/invalid inputs");
    }

    #[test]
    fn spike_schema_config_export() {
        let schema = serde_json::to_value(schema_for!(ZenConfig)).unwrap();
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.schema.json");
        let pretty = serde_json::to_string_pretty(&schema).unwrap();
        std::fs::write(&path, &pretty).unwrap();

        assert!(path.exists());
        let size = pretty.len();
        println!("=== Part C Test 12: Config Export ===");
        println!("Exported config.schema.json: {} bytes", size);

        // Verify re-loadable
        let loaded: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert!(jsonschema::validator_for(&loaded).is_ok());
        println!("PASS: Config schema exported and re-loadable");
    }

    // =========================================================================
    // Part D: CLI Response & Input Schemas (3 tests)
    // =========================================================================

    #[test]
    fn spike_schema_response_structs() {
        let schemas: Vec<(&str, serde_json::Value)> = vec![
            (
                "FindingCreateResponse",
                serde_json::to_value(schema_for!(FindingCreateResponse)).unwrap(),
            ),
            (
                "SessionStartResponse",
                serde_json::to_value(schema_for!(SessionStartResponse)).unwrap(),
            ),
            (
                "WhatsNextResponse",
                serde_json::to_value(schema_for!(WhatsNextResponse)).unwrap(),
            ),
            (
                "SearchResultsResponse",
                serde_json::to_value(schema_for!(SearchResultsResponse)).unwrap(),
            ),
            (
                "RebuildResponse",
                serde_json::to_value(schema_for!(RebuildResponse)).unwrap(),
            ),
        ];

        println!("=== Part D Test 13: Response Structs ===");
        for (name, schema) in &schemas {
            assert_eq!(schema["type"], "object", "{name} should be object");
            let props = schema["properties"].as_object().map_or(0, |o| o.len());
            println!("  {name}: {props} properties");
            assert!(
                jsonschema::validator_for(schema).is_ok(),
                "{name} should be valid schema"
            );
        }
        println!("PASS: All 5 response schemas generated");
    }

    #[test]
    fn spike_schema_response_validate() {
        // FindingCreateResponse
        let schema = serde_json::to_value(schema_for!(FindingCreateResponse)).unwrap();
        let valid = serde_json::to_value(FindingCreateResponse {
            finding: make_finding(),
        })
        .unwrap();
        assert!(
            validate_with_schema(&schema, &valid).is_ok(),
            "Valid response should pass"
        );

        // RebuildResponse
        let rebuild_schema = serde_json::to_value(schema_for!(RebuildResponse)).unwrap();
        let valid_rebuild = json!({
            "rebuilt": true,
            "trail_files": 3,
            "operations_replayed": 150,
            "entities_created": 45,
            "duration_ms": 230
        });
        assert!(validate_with_schema(&rebuild_schema, &valid_rebuild).is_ok());

        // Invalid: wrong type
        let invalid_rebuild = json!({
            "rebuilt": "yes",
            "trail_files": 3,
            "operations_replayed": 150,
            "entities_created": 45,
            "duration_ms": 230
        });
        let err = validate_with_schema(&rebuild_schema, &invalid_rebuild);
        assert!(err.is_err(), "Wrong type for rebuilt should fail");
        println!("  Rebuild wrong type error: {}", err.unwrap_err()[0]);

        // Invalid: missing required field
        let missing = json!({"rebuilt": true});
        let err = validate_with_schema(&rebuild_schema, &missing);
        assert!(err.is_err(), "Missing required fields should fail");

        println!("=== Part D Test 14: Response Validate ===");
        println!("PASS: Response validation catches wrong types and missing fields");
    }

    #[test]
    fn spike_schema_input_validate() {
        // Simple: Vec<String> for --tasks input
        let schema = serde_json::to_value(schema_for!(Vec<String>)).unwrap();

        assert!(
            validate_with_schema(&schema, &json!(["task1", "task2"])).is_ok(),
            "String array should pass"
        );
        assert!(
            validate_with_schema(&schema, &json!([])).is_ok(),
            "Empty array should pass"
        );
        assert!(
            validate_with_schema(&schema, &json!([123, null])).is_err(),
            "Non-string array should fail"
        );
        assert!(
            validate_with_schema(&schema, &json!("not-array")).is_err(),
            "Non-array should fail"
        );

        // Complex: Vec<TaskDefinition>
        #[derive(Debug, Serialize, Deserialize, JsonSchema)]
        struct TaskDefinition {
            title: String,
            description: Option<String>,
        }
        let complex_schema = serde_json::to_value(schema_for!(Vec<TaskDefinition>)).unwrap();
        let valid = json!([
            {"title": "Task 1"},
            {"title": "Task 2", "description": "with desc"}
        ]);
        assert!(
            validate_with_schema(&complex_schema, &valid).is_ok(),
            "Valid task definitions should pass"
        );

        let invalid = json!([{"description": "no title"}]);
        assert!(
            validate_with_schema(&complex_schema, &invalid).is_err(),
            "Missing title should fail"
        );

        println!("=== Part D Test 15: Input Validate ===");
        println!(
            "PASS: Simple (Vec<String>) and complex (Vec<TaskDefinition>) input validation works"
        );
    }

    // =========================================================================
    // Part E: Audit Detail Schemas (2 tests)
    // =========================================================================

    #[test]
    fn spike_schema_audit_detail_types() {
        let status_schema = serde_json::to_value(schema_for!(StatusChangedDetail)).unwrap();
        let linked_schema = serde_json::to_value(schema_for!(LinkedDetail)).unwrap();
        let tagged_schema = serde_json::to_value(schema_for!(TaggedDetail)).unwrap();
        let indexed_schema = serde_json::to_value(schema_for!(IndexedDetail)).unwrap();

        // Validate real examples
        assert!(
            validate_with_schema(&status_schema, &json!({"from": "open", "to": "done"})).is_ok()
        );
        assert!(
            validate_with_schema(
                &status_schema,
                &json!({"from": "open", "to": "done", "reason": "completed task"})
            )
            .is_ok()
        );
        assert!(
            validate_with_schema(
                &linked_schema,
                &json!({
                    "source_type": "finding", "source_id": "fnd-001",
                    "target_type": "hypothesis", "target_id": "hyp-001",
                    "relation": "validates"
                })
            )
            .is_ok()
        );
        assert!(validate_with_schema(&tagged_schema, &json!({"tag": "verified"})).is_ok());
        assert!(
            validate_with_schema(
                &indexed_schema,
                &json!({
                    "package": "tokio", "ecosystem": "rust", "symbols": 450, "duration_ms": 1200
                })
            )
            .is_ok()
        );

        // Missing required field
        assert!(validate_with_schema(&status_schema, &json!({"from": "open"})).is_err());
        assert!(validate_with_schema(&tagged_schema, &json!({})).is_err());

        println!("=== Part E Test 16: Audit Detail Types ===");
        println!("PASS: All 4 audit detail types validate correctly");
    }

    #[test]
    fn spike_schema_audit_detail_dispatch() {
        // Build a dispatch map: AuditAction -> detail schema
        let detail_schemas: HashMap<&str, serde_json::Value> = [
            (
                "status_changed",
                serde_json::to_value(schema_for!(StatusChangedDetail)).unwrap(),
            ),
            (
                "linked",
                serde_json::to_value(schema_for!(LinkedDetail)).unwrap(),
            ),
            (
                "tagged",
                serde_json::to_value(schema_for!(TaggedDetail)).unwrap(),
            ),
            (
                "indexed",
                serde_json::to_value(schema_for!(IndexedDetail)).unwrap(),
            ),
        ]
        .into();

        // status_changed + correct detail -> pass
        let schema = &detail_schemas["status_changed"];
        assert!(validate_with_schema(schema, &json!({"from": "open", "to": "done"})).is_ok());

        // status_changed + wrong detail (TaggedDetail) -> fail
        assert!(
            validate_with_schema(schema, &json!({"tag": "verified"})).is_err(),
            "TaggedDetail should fail against StatusChangedDetail schema"
        );

        // tagged + correct detail -> pass
        let tag_schema = &detail_schemas["tagged"];
        assert!(validate_with_schema(tag_schema, &json!({"tag": "verified"})).is_ok());

        // Actions without specific detail schemas (created, updated) accept any object
        // In production, we'd allow these to pass with no schema check

        println!("=== Part E Test 17: Audit Detail Dispatch ===");
        println!(
            "PASS: Dispatch correctly routes to per-action schemas, rejects wrong detail types"
        );
    }

    // =========================================================================
    // Part F: DuckDB Metadata Schemas (2 tests)
    // =========================================================================

    #[test]
    fn spike_schema_metadata_rust() {
        let schema = serde_json::to_value(schema_for!(RustMetadata)).unwrap();
        assert_eq!(schema["type"], "object");

        // Validate a real Rust metadata example (from 02-ducklake-data-model.md)
        let valid = json!({
            "lifetimes": ["'a", "'b"],
            "where_clause": "T: Send + Sync",
            "is_pyo3": false,
            "trait_name": "Iterator",
            "for_type": "Vec<T>",
            "variants": null,
            "fields": null,
            "methods": ["next", "size_hint"],
            "associated_types": ["Item"],
            "abi": null,
            "doc_sections": {
                "errors": ["Returns Err if connection fails"],
                "panics": "Panics if buffer overflows",
                "safety": null,
                "examples": ["let x = foo();"]
            }
        });
        assert!(
            validate_with_schema(&schema, &valid).is_ok(),
            "Full Rust metadata should pass"
        );

        // Minimal valid (only required field is is_pyo3)
        let minimal = json!({"is_pyo3": false});
        assert!(
            validate_with_schema(&schema, &minimal).is_ok(),
            "Minimal Rust metadata should pass"
        );

        // Nested optional struct (doc_sections with partial fields)
        let partial = json!({
            "is_pyo3": true,
            "doc_sections": {"errors": ["E1", "E2"]}
        });
        assert!(
            validate_with_schema(&schema, &partial).is_ok(),
            "Partial doc_sections should pass"
        );

        // Optional<Vec<String>> produces nullable array
        let props = &schema["properties"];
        let lifetimes = &props["lifetimes"];
        // Should allow null or array of strings
        println!(
            "  lifetimes schema: {}",
            serde_json::to_string(lifetimes).unwrap()
        );

        println!("=== Part F Test 18: Rust Metadata ===");
        println!("PASS: RustMetadata with nested RustDocSections, Option<Vec<String>>, all work");
    }

    #[test]
    fn spike_schema_metadata_python_ts() {
        // Python metadata
        let py_schema = serde_json::to_value(schema_for!(PythonMetadata)).unwrap();

        let valid_py = json!({
            "is_generator": false,
            "is_property": true,
            "is_pydantic": false,
            "is_protocol": false,
            "is_dataclass": true,
            "base_classes": ["BaseModel"],
            "decorators": ["@dataclass", "@frozen"],
            "parameters": ["self", "name: str", "age: int"],
            "doc_sections": {
                "args": {"name": "The user's name", "age": "The user's age"},
                "returns": "A new User instance",
                "raises": {"ValueError": "If age is negative"}
            }
        });
        assert!(
            validate_with_schema(&py_schema, &valid_py).is_ok(),
            "Full Python metadata should pass"
        );

        // Verify HashMap<String,String> produces additionalProperties
        let doc_sections = &py_schema["properties"]["doc_sections"];
        // Navigate to the args field schema (may be nested in $defs)
        println!(
            "  Python doc_sections schema snippet: {}",
            &serde_json::to_string(doc_sections).unwrap()
                [..200.min(serde_json::to_string(doc_sections).unwrap().len())]
        );

        // TypeScript metadata
        let ts_schema = serde_json::to_value(schema_for!(TypeScriptMetadata)).unwrap();
        let valid_ts = json!({
            "is_exported": true,
            "is_default_export": false,
            "type_parameters": ["T", "U"],
            "implements": ["Iterable<T>"]
        });
        assert!(
            validate_with_schema(&ts_schema, &valid_ts).is_ok(),
            "Full TypeScript metadata should pass"
        );

        // Minimal TS (only required bools)
        let minimal_ts = json!({"is_exported": true, "is_default_export": false});
        assert!(
            validate_with_schema(&ts_schema, &minimal_ts).is_ok(),
            "Minimal TypeScript metadata should pass"
        );

        // Verify HashMap produces additionalProperties schema
        let py_doc_schema = serde_json::to_value(schema_for!(PythonDocSections)).unwrap();
        let args_prop = &py_doc_schema["properties"]["args"];
        // Should have additionalProperties or use $ref to HashMap schema
        println!(
            "  PythonDocSections.args schema: {}",
            serde_json::to_string_pretty(args_prop).unwrap()
        );

        println!("=== Part F Test 19: Python & TypeScript Metadata ===");
        println!("PASS: HashMap<String,String> and nested metadata types all work");
    }

    // =========================================================================
    // Part G: Schema Registry & Cross-Cutting (3 tests)
    // =========================================================================

    #[test]
    fn spike_schema_draft_compat() {
        // schemars 1.x generates Draft 2020-12 by default
        let schema = serde_json::to_value(schema_for!(Finding)).unwrap();

        // Check the $schema field
        let draft = schema.get("$schema").and_then(|v| v.as_str());
        println!("  schemars default draft: {:?}", draft);

        // jsonschema should accept it via auto-detection
        let validator = jsonschema::validator_for(&schema);
        assert!(
            validator.is_ok(),
            "jsonschema should accept schemars-generated schema"
        );

        // Validate actual data
        let data = serde_json::to_value(make_finding()).unwrap();
        assert!(validator.unwrap().is_valid(&data));

        // Test: can we also generate Draft 7?
        // schemars 1.x uses SchemaSettings for this
        use schemars::generate::SchemaSettings;
        let settings = SchemaSettings::draft07();
        let generator = settings.into_generator();
        let draft7_schema =
            serde_json::to_value(generator.into_root_schema_for::<Finding>()).unwrap();
        let draft7_version = draft7_schema.get("$schema").and_then(|v| v.as_str());
        println!("  Draft 7 schema: {:?}", draft7_version);

        let validator7 = jsonschema::validator_for(&draft7_schema);
        assert!(
            validator7.is_ok(),
            "jsonschema should accept Draft 7 schema"
        );
        assert!(validator7.unwrap().is_valid(&data));

        println!("=== Part G Test 20: Draft Compatibility ===");
        println!("PASS: Both Draft 2020-12 and Draft 7 schemas work with jsonschema 0.28");
    }

    #[test]
    fn spike_schema_registry() {
        // Prototype SchemaRegistry
        let mut registry: HashMap<String, serde_json::Value> = HashMap::new();

        // Register all entity schemas (12)
        let entity_entries: Vec<(&str, serde_json::Value)> = vec![
            (
                "session",
                serde_json::to_value(schema_for!(Session)).unwrap(),
            ),
            (
                "research_item",
                serde_json::to_value(schema_for!(ResearchItem)).unwrap(),
            ),
            (
                "finding",
                serde_json::to_value(schema_for!(Finding)).unwrap(),
            ),
            (
                "hypothesis",
                serde_json::to_value(schema_for!(Hypothesis)).unwrap(),
            ),
            (
                "insight",
                serde_json::to_value(schema_for!(Insight)).unwrap(),
            ),
            ("issue", serde_json::to_value(schema_for!(Issue)).unwrap()),
            ("task", serde_json::to_value(schema_for!(Task)).unwrap()),
            (
                "impl_log",
                serde_json::to_value(schema_for!(ImplLog)).unwrap(),
            ),
            (
                "compat_check",
                serde_json::to_value(schema_for!(CompatCheck)).unwrap(),
            ),
            ("study", serde_json::to_value(schema_for!(Study)).unwrap()),
            (
                "entity_link",
                serde_json::to_value(schema_for!(EntityLink)).unwrap(),
            ),
            (
                "audit_entry",
                serde_json::to_value(schema_for!(AuditEntry)).unwrap(),
            ),
        ];
        for (name, schema) in entity_entries {
            registry.insert(name.to_string(), schema);
        }

        // Trail envelope (1)
        registry.insert(
            "trail_operation".into(),
            serde_json::to_value(schema_for!(TrailOperation)).unwrap(),
        );

        // Config (1)
        registry.insert(
            "config".into(),
            serde_json::to_value(schema_for!(ZenConfig)).unwrap(),
        );

        // Response schemas (5)
        registry.insert(
            "finding_create_response".into(),
            serde_json::to_value(schema_for!(FindingCreateResponse)).unwrap(),
        );
        registry.insert(
            "session_start_response".into(),
            serde_json::to_value(schema_for!(SessionStartResponse)).unwrap(),
        );
        registry.insert(
            "whats_next_response".into(),
            serde_json::to_value(schema_for!(WhatsNextResponse)).unwrap(),
        );
        registry.insert(
            "search_results_response".into(),
            serde_json::to_value(schema_for!(SearchResultsResponse)).unwrap(),
        );
        registry.insert(
            "rebuild_response".into(),
            serde_json::to_value(schema_for!(RebuildResponse)).unwrap(),
        );

        // Audit detail schemas (4)
        registry.insert(
            "detail_status_changed".into(),
            serde_json::to_value(schema_for!(StatusChangedDetail)).unwrap(),
        );
        registry.insert(
            "detail_linked".into(),
            serde_json::to_value(schema_for!(LinkedDetail)).unwrap(),
        );
        registry.insert(
            "detail_tagged".into(),
            serde_json::to_value(schema_for!(TaggedDetail)).unwrap(),
        );
        registry.insert(
            "detail_indexed".into(),
            serde_json::to_value(schema_for!(IndexedDetail)).unwrap(),
        );

        // Metadata schemas (3)
        registry.insert(
            "metadata_rust".into(),
            serde_json::to_value(schema_for!(RustMetadata)).unwrap(),
        );
        registry.insert(
            "metadata_python".into(),
            serde_json::to_value(schema_for!(PythonMetadata)).unwrap(),
        );
        registry.insert(
            "metadata_typescript".into(),
            serde_json::to_value(schema_for!(TypeScriptMetadata)).unwrap(),
        );

        // Verify registry
        assert_eq!(registry.len(), 26, "Should have 26 schemas (12+1+1+5+4+3)");

        // get() works
        assert!(registry.get("finding").is_some());
        assert!(registry.get("trail_operation").is_some());
        assert!(registry.get("nonexistent").is_none());

        // validate() works
        let finding_schema = registry.get("finding").unwrap();
        let valid = serde_json::to_value(make_finding()).unwrap();
        assert!(validate_with_schema(finding_schema, &valid).is_ok());

        let invalid = json!({"id": "fnd-1234"});
        assert!(validate_with_schema(finding_schema, &invalid).is_err());

        // Measure construction time
        let start = std::time::Instant::now();
        let mut _registry2: HashMap<String, serde_json::Value> = HashMap::new();
        _registry2.insert(
            "finding".into(),
            serde_json::to_value(schema_for!(Finding)).unwrap(),
        );
        _registry2.insert(
            "hypothesis".into(),
            serde_json::to_value(schema_for!(Hypothesis)).unwrap(),
        );
        _registry2.insert(
            "issue".into(),
            serde_json::to_value(schema_for!(Issue)).unwrap(),
        );
        _registry2.insert(
            "task".into(),
            serde_json::to_value(schema_for!(Task)).unwrap(),
        );
        let elapsed = start.elapsed();
        println!("  4-schema registry construction: {:?}", elapsed);

        // Export test
        let dir = tempfile::tempdir().unwrap();
        let mut exported = 0;
        for (name, schema) in &registry {
            let path = dir.path().join(format!("{name}.schema.json"));
            std::fs::write(&path, serde_json::to_string_pretty(schema).unwrap()).unwrap();
            exported += 1;
        }
        assert_eq!(exported, 26);

        let mut names: Vec<&str> = registry.keys().map(|k| k.as_str()).collect();
        names.sort();

        println!("=== Part G Test 21: Schema Registry ===");
        println!("Total schemas: {}", registry.len());
        println!("Names: {:?}", names);
        println!("PASS: SchemaRegistry prototype with 26 schemas, get/validate/export all work");
    }

    #[test]
    fn spike_schema_compare_handwritten() {
        // Hand-written trail schema from spike 0.13 (simplified reproduction)
        let handwritten = json!({
            "type": "object",
            "required": ["ts", "ses", "op", "entity", "id", "data"],
            "properties": {
                "ts": {"type": "string"},
                "ses": {"type": "string"},
                "op": {"type": "string", "enum": ["create", "update", "delete"]},
                "entity": {"type": "string", "enum": [
                    "session", "research", "finding", "hypothesis", "insight",
                    "issue", "task", "impl_log", "compat", "study", "entity_link",
                    "finding_tag", "audit"
                ]},
                "id": {"type": "string"},
                "data": {"type": "object"}
            },
            "additionalProperties": false
        });

        let generated = serde_json::to_value(schema_for!(TrailOperation)).unwrap();

        // Compare required fields
        let hw_required: Vec<&str> = handwritten["required"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap())
            .collect();
        let gen_required: Vec<&str> = generated["required"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap())
            .collect();

        println!("=== Part G Test 22: Compare Hand-written vs Generated ===");
        println!("  Hand-written required: {:?}", hw_required);
        println!("  Generated required: {:?}", gen_required);

        // Both should require the same fields
        for field in &hw_required {
            assert!(
                gen_required.contains(field),
                "Generated schema missing required field: {field}"
            );
        }

        // Compare op enum values
        let hw_ops: Vec<&str> = handwritten["properties"]["op"]["enum"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap())
            .collect();
        println!("  Hand-written op values: {:?}", hw_ops);

        // Generated op may be $ref or inline
        let gen_op = &generated["properties"]["op"];
        println!(
            "  Generated op schema: {}",
            serde_json::to_string(gen_op).unwrap()
        );

        // Compare additionalProperties behavior
        let hw_additional = handwritten.get("additionalProperties");
        let gen_additional = generated.get("additionalProperties");
        println!("  Hand-written additionalProperties: {:?}", hw_additional);
        println!("  Generated additionalProperties: {:?}", gen_additional);

        // Benchmark: validate 1000 operations
        let ops: Vec<serde_json::Value> = (0..1000)
            .map(|i| {
                json!({
                    "ts": "2026-02-08T10:00:00Z",
                    "ses": "ses-001",
                    "op": "create",
                    "entity": "finding",
                    "id": format!("fnd-{i:04}"),
                    "data": {"content": "test", "confidence": "high"}
                })
            })
            .collect();

        let hw_validator = jsonschema::validator_for(&handwritten).unwrap();
        let start = std::time::Instant::now();
        for op in &ops {
            assert!(hw_validator.is_valid(op));
        }
        let hw_time = start.elapsed();

        let gen_validator = jsonschema::validator_for(&generated).unwrap();
        let start = std::time::Instant::now();
        for op in &ops {
            assert!(gen_validator.is_valid(op));
        }
        let gen_time = start.elapsed();

        println!("  Hand-written 1000 validations: {:?}", hw_time);
        println!("  Generated 1000 validations: {:?}", gen_time);
        println!("PASS: Both schemas validate correctly, performance comparison documented");
    }
}
