//! # Spike 0.16: JSONL Trail Schema Versioning
//!
//! Validates the "Hybrid" versioning strategy (Approach D) for JSONL trail schema
//! evolution using only existing crates (`serde`, `schemars`, `jsonschema`, `serde-jsonlines`).
//!
//! **10 tests** across 4 sections:
//! - Part A: Envelope versioning (3 tests)
//! - Part B: Additive evolution (3 tests)
//! - Part C: Version-dispatch migration (2 tests)
//! - Part D: additionalProperties convention + JSONL roundtrip (2 tests)

#[cfg(test)]
mod tests {
    use schemars::{schema_for, JsonSchema};
    use serde::{Deserialize, Serialize};
    use serde_json::json;

    // =========================================================================
    // Types for this spike. These mirror the planned TrailOperation structure
    // with the proposed `v` version field added.
    // =========================================================================

    fn default_trail_version() -> u32 {
        1
    }

    #[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
    #[serde(rename_all = "snake_case")]
    enum TrailOp {
        Create,
        Update,
        Delete,
    }

    #[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
    #[serde(rename_all = "snake_case")]
    enum EntityType {
        Finding,
        Hypothesis,
        Issue,
        Task,
        Session,
        Research,
        Insight,
        ImplLog,
        Compat,
        Study,
    }

    /// The versioned trail operation envelope.
    /// `v` defaults to 1 when absent (backward-compat with pre-versioning trails).
    #[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
    struct TrailOperation {
        #[serde(default = "default_trail_version")]
        v: u32,
        ts: String,
        ses: String,
        op: TrailOp,
        entity: EntityType,
        id: String,
        data: serde_json::Value,
    }

    // =========================================================================
    // Part A: Envelope Versioning
    // =========================================================================

    /// H1: Old trail JSON without `v` field deserializes with v == 1
    #[test]
    fn spike_v_field_defaults_to_1_when_absent() {
        // This is what existing trail lines look like -- no `v` field
        let old_trail_json = json!({
            "ts": "2026-02-08T10:00:00Z",
            "ses": "ses-abc123",
            "op": "create",
            "entity": "finding",
            "id": "fnd-001",
            "data": {"content": "reqwest supports connection pooling"}
        });

        let op: TrailOperation = serde_json::from_value(old_trail_json).unwrap();
        assert_eq!(op.v, 1, "Missing v field should default to 1");
        assert_eq!(op.op, TrailOp::Create);
        assert_eq!(op.entity, EntityType::Finding);
    }

    /// v field is preserved when explicitly set
    #[test]
    fn spike_v_field_preserved_when_present() {
        let v2_trail_json = json!({
            "v": 2,
            "ts": "2026-02-08T10:00:00Z",
            "ses": "ses-abc123",
            "op": "update",
            "entity": "finding",
            "id": "fnd-001",
            "data": {"confidence": {"level": "high", "basis": "tested"}}
        });

        let op: TrailOperation = serde_json::from_value(v2_trail_json).unwrap();
        assert_eq!(op.v, 2);
    }

    /// H2: schemars-generated schema + jsonschema validation accepts JSON
    /// both with and without the `v` field.
    #[test]
    fn spike_schema_validates_with_and_without_v() {
        let schema = serde_json::to_value(schema_for!(TrailOperation)).unwrap();
        let validator = jsonschema::validator_for(&schema).unwrap();

        // With v
        let with_v = json!({
            "v": 1,
            "ts": "2026-02-08T10:00:00Z",
            "ses": "ses-abc123",
            "op": "create",
            "entity": "finding",
            "id": "fnd-001",
            "data": {"content": "test"}
        });
        assert!(
            validator.is_valid(&with_v),
            "JSON with v field should validate"
        );

        // Without v
        let without_v = json!({
            "ts": "2026-02-08T10:00:00Z",
            "ses": "ses-abc123",
            "op": "create",
            "entity": "finding",
            "id": "fnd-001",
            "data": {"content": "test"}
        });

        // schemars may or may not mark `v` as required.
        // If it's required (schemars doesn't know about serde defaults), we need to know.
        let is_valid = validator.is_valid(&without_v);

        // Document the behavior either way
        if is_valid {
            // Best case: schemars respects serde(default) and makes v optional
            println!(
                "FINDING: schemars makes `v` optional (not in required) -- \
                 schema validation accepts missing v. No workaround needed."
            );
        } else {
            // Acceptable case: schemars marks v as required.
            // We need to know this for the pre-commit hook: it should NOT
            // reject old trail files missing v.
            println!(
                "FINDING: schemars marks `v` as required despite #[serde(default)] -- \
                 pre-commit hook must use relaxed validation for the v field."
            );
            // Verify the error is specifically about `v` being missing
            let errors: Vec<_> = validator.iter_errors(&without_v).collect();
            let has_v_error = errors.iter().any(|e| {
                let msg = format!("{}", e);
                msg.contains("'v'") || msg.contains("\"v\"")
            });
            assert!(
                has_v_error,
                "If validation fails, it should be specifically about the `v` field. \
                 Errors: {:?}",
                errors.iter().map(|e| e.to_string()).collect::<Vec<_>>()
            );
        }

        // Either way, document whether `v` is in the schema's `required` array
        let required = schema
            .get("required")
            .and_then(|r| r.as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>())
            .unwrap_or_default();

        println!(
            "FINDING: schema required fields = {:?}, v_is_required = {}",
            required,
            required.contains(&"v")
        );
    }

    // =========================================================================
    // Part B: Additive Evolution
    // =========================================================================

    // --- V1.1 Finding: added optional field (no version bump needed) ---
    #[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
    struct FindingDataV1WithOptional {
        content: String,
        confidence: String,
        source_url: Option<String>,
    }

    // --- V1.2 Finding: added field with default (no version bump needed) ---
    #[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
    struct FindingDataV1WithDefault {
        content: String,
        confidence: String,
        #[serde(default)]
        methodology: String,
    }

    // --- V1.3 Finding: renamed field via alias (no version bump needed) ---
    #[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
    struct CompatDataWithAlias {
        #[serde(alias = "package_a")]
        pkg_a: String,
        #[serde(alias = "package_b")]
        pkg_b: String,
        compatible: bool,
    }

    /// H3: Adding Option<String> to entity is fully backward-compatible
    #[test]
    fn spike_new_optional_field_backward_compat() {
        // Old data without source_url
        let old_data = json!({
            "content": "reqwest supports pooling",
            "confidence": "high"
        });

        // Deserialize into new struct with Option<String> field
        let finding: FindingDataV1WithOptional = serde_json::from_value(old_data).unwrap();
        assert_eq!(finding.content, "reqwest supports pooling");
        assert_eq!(
            finding.source_url, None,
            "Missing optional field should be None"
        );

        // New data with source_url also works
        let new_data = json!({
            "content": "reqwest supports pooling",
            "confidence": "high",
            "source_url": "https://docs.rs/reqwest"
        });
        let finding2: FindingDataV1WithOptional = serde_json::from_value(new_data).unwrap();
        assert_eq!(
            finding2.source_url,
            Some("https://docs.rs/reqwest".to_string())
        );

        // Schema validation: old data (missing source_url) validates against new schema
        let schema = serde_json::to_value(schema_for!(FindingDataV1WithOptional)).unwrap();
        let validator = jsonschema::validator_for(&schema).unwrap();

        let old_data_again = json!({"content": "test", "confidence": "high"});
        assert!(
            validator.is_valid(&old_data_again),
            "Old data should validate against schema with new optional field. \
             Errors: {:?}",
            validator
                .iter_errors(&old_data_again)
                .map(|e| e.to_string())
                .collect::<Vec<_>>()
        );
    }

    /// H3: Adding field with #[serde(default)] is backward-compatible
    #[test]
    fn spike_new_default_field_backward_compat() {
        // Old data without methodology
        let old_data = json!({
            "content": "tokio spawn is async",
            "confidence": "medium"
        });

        let finding: FindingDataV1WithDefault = serde_json::from_value(old_data).unwrap();
        assert_eq!(finding.methodology, "", "Default String should be empty");

        // Schema validation: check if schemars treats default fields as optional
        let schema = serde_json::to_value(schema_for!(FindingDataV1WithDefault)).unwrap();
        let validator = jsonschema::validator_for(&schema).unwrap();

        let old_data_again = json!({"content": "test", "confidence": "high"});
        let is_valid = validator.is_valid(&old_data_again);

        println!(
            "FINDING: #[serde(default)] field `methodology` in schema required = {}",
            schema
                .get("required")
                .and_then(|r| r.as_array())
                .map(|arr| arr.iter().any(|v| v.as_str() == Some("methodology")))
                .unwrap_or(false)
        );

        if !is_valid {
            let errors: Vec<_> = validator
                .iter_errors(&old_data_again)
                .map(|e| e.to_string())
                .collect();
            println!(
                "FINDING: Schema validation rejects missing default field. Errors: {:?}. \
                 This means pre-commit/rebuild validation must NOT use schema for \
                 #[serde(default)] fields, OR we must make them Option<T> instead.",
                errors
            );
        } else {
            println!(
                "FINDING: Schema validation accepts missing #[serde(default)] field. \
                 Additive evolution with defaults works end-to-end."
            );
        }
    }

    /// H4: #[serde(alias)] allows old field names to deserialize
    #[test]
    fn spike_alias_field_rename_backward_compat() {
        // Old data with old field names
        let old_data = json!({
            "package_a": "tokio",
            "package_b": "async-std",
            "compatible": true
        });

        let compat: CompatDataWithAlias = serde_json::from_value(old_data).unwrap();
        assert_eq!(compat.pkg_a, "tokio");
        assert_eq!(compat.pkg_b, "async-std");

        // New data with new field names also works
        let new_data = json!({
            "pkg_a": "tokio",
            "pkg_b": "async-std",
            "compatible": true
        });

        let compat2: CompatDataWithAlias = serde_json::from_value(new_data).unwrap();
        assert_eq!(compat2.pkg_a, "tokio");

        // Schema validation: check what name appears in schema (new name or alias)
        let schema = serde_json::to_value(schema_for!(CompatDataWithAlias)).unwrap();
        let props = schema
            .get("properties")
            .and_then(|p| p.as_object())
            .unwrap();
        let field_names: Vec<&String> = props.keys().collect();
        println!(
            "FINDING: Schema property names after alias: {:?} \
             (expecting new names pkg_a/pkg_b, NOT old names package_a/package_b)",
            field_names
        );

        // Schema should use the new names
        assert!(
            props.contains_key("pkg_a"),
            "Schema should use the Rust field name (pkg_a), not the alias"
        );

        // Validation with old field names -- this may fail because schema uses new names
        let validator = jsonschema::validator_for(&schema).unwrap();
        let old_data_again = json!({
            "package_a": "tokio",
            "package_b": "async-std",
            "compatible": true
        });

        let old_valid = validator.is_valid(&old_data_again);
        println!(
            "FINDING: Old field names validate against new schema = {}. \
             If false, schema validation must be skipped or relaxed for renamed fields, \
             but serde deserialization still works via alias.",
            old_valid
        );
    }

    // =========================================================================
    // Part C: Version-Dispatch Migration
    // =========================================================================

    /// H5: Transform v1 data Value to v2 shape, validate against v2 schema
    #[test]
    fn spike_v1_to_v2_value_migration() {
        // V1: confidence is a string
        let v1_data = json!({
            "content": "reqwest supports pooling",
            "confidence": "high"
        });

        // V2: confidence is a structured object
        #[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
        struct FindingDataV2 {
            content: String,
            confidence: FindingConfidence,
        }

        #[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
        struct FindingConfidence {
            level: String,
            basis: String,
        }

        // Migration function: transform v1 Value -> v2 Value
        fn migrate_finding_v1_to_v2(mut data: serde_json::Value) -> serde_json::Value {
            if let Some(confidence) = data.get("confidence").and_then(|c| c.as_str()) {
                let level = confidence.to_string();
                data["confidence"] = json!({
                    "level": level,
                    "basis": "unknown"
                });
            }
            data
        }

        // Apply migration
        let v2_data = migrate_finding_v1_to_v2(v1_data);

        // Verify it deserializes as v2
        let finding: FindingDataV2 = serde_json::from_value(v2_data.clone()).unwrap();
        assert_eq!(finding.confidence.level, "high");
        assert_eq!(finding.confidence.basis, "unknown");

        // Validate migrated data against v2 schema
        let v2_schema = serde_json::to_value(schema_for!(FindingDataV2)).unwrap();
        let validator = jsonschema::validator_for(&v2_schema).unwrap();
        assert!(
            validator.is_valid(&v2_data),
            "Migrated v1->v2 data should validate against v2 schema"
        );
    }

    /// H5: Replay dispatch routes mixed v1+v2 operations correctly
    #[test]
    fn spike_replay_dispatch_routes_by_version() {
        // Migration function (same as above)
        fn migrate_finding_v1_to_v2(mut data: serde_json::Value) -> serde_json::Value {
            if let Some(confidence) = data.get("confidence").and_then(|c| c.as_str()) {
                let level = confidence.to_string();
                data["confidence"] = json!({
                    "level": level,
                    "basis": "unknown"
                });
            }
            data
        }

        // Dispatch function
        fn dispatch(op: &TrailOperation) -> Result<serde_json::Value, String> {
            match op.v {
                1 => {
                    if op.entity == EntityType::Finding {
                        Ok(migrate_finding_v1_to_v2(op.data.clone()))
                    } else {
                        Ok(op.data.clone())
                    }
                }
                2 => Ok(op.data.clone()), // Already v2 format
                v => Err(format!("Unsupported trail version: {}", v)),
            }
        }

        let ops = vec![
            // v1 finding (old format)
            TrailOperation {
                v: 1,
                ts: "2026-02-08T10:00:00Z".into(),
                ses: "ses-001".into(),
                op: TrailOp::Create,
                entity: EntityType::Finding,
                id: "fnd-001".into(),
                data: json!({"content": "test", "confidence": "high"}),
            },
            // v2 finding (new format)
            TrailOperation {
                v: 2,
                ts: "2026-02-08T11:00:00Z".into(),
                ses: "ses-001".into(),
                op: TrailOp::Create,
                entity: EntityType::Finding,
                id: "fnd-002".into(),
                data: json!({"content": "test2", "confidence": {"level": "low", "basis": "speculation"}}),
            },
            // v1 non-finding (no migration needed)
            TrailOperation {
                v: 1,
                ts: "2026-02-08T10:30:00Z".into(),
                ses: "ses-001".into(),
                op: TrailOp::Create,
                entity: EntityType::Task,
                id: "tsk-001".into(),
                data: json!({"title": "Do something"}),
            },
            // v99 (unsupported)
            TrailOperation {
                v: 99,
                ts: "2026-02-08T12:00:00Z".into(),
                ses: "ses-001".into(),
                op: TrailOp::Create,
                entity: EntityType::Finding,
                id: "fnd-003".into(),
                data: json!({"content": "future"}),
            },
        ];

        let results: Vec<Result<serde_json::Value, String>> =
            ops.iter().map(|op| dispatch(op)).collect();

        // v1 finding -> migrated
        let r0 = results[0].as_ref().unwrap();
        assert_eq!(
            r0["confidence"]["level"], "high",
            "v1 finding should be migrated"
        );
        assert_eq!(r0["confidence"]["basis"], "unknown");

        // v2 finding -> passed through
        let r1 = results[1].as_ref().unwrap();
        assert_eq!(r1["confidence"]["level"], "low");
        assert_eq!(r1["confidence"]["basis"], "speculation");

        // v1 task -> passed through (no finding migration)
        let r2 = results[2].as_ref().unwrap();
        assert_eq!(r2["title"], "Do something");

        // v99 -> error
        assert!(results[3].is_err());
        assert!(results[3]
            .as_ref()
            .unwrap_err()
            .contains("Unsupported trail version: 99"));
    }

    // =========================================================================
    // Part D: additionalProperties Convention + JSONL Roundtrip
    // =========================================================================

    /// H6: Trail schema (permissive) accepts unknown fields.
    /// Config schema with deny_unknown_fields rejects unknown fields.
    #[test]
    fn spike_additional_properties_convention() {
        // --- Trail: permissive (schemars default is additionalProperties: true) ---
        let trail_schema = serde_json::to_value(schema_for!(TrailOperation)).unwrap();
        let trail_validator = jsonschema::validator_for(&trail_schema).unwrap();

        // Trail with an extra unknown field
        let trail_with_extra = json!({
            "v": 1,
            "ts": "2026-02-08T10:00:00Z",
            "ses": "ses-001",
            "op": "create",
            "entity": "finding",
            "id": "fnd-001",
            "data": {"content": "test"},
            "extra_field": "should be accepted"
        });

        let trail_accepts_extra = trail_validator.is_valid(&trail_with_extra);
        println!(
            "FINDING: Trail schema accepts unknown fields = {}",
            trail_accepts_extra
        );

        // Check if additionalProperties is explicitly set in the schema
        let trail_addl = trail_schema.get("additionalProperties");
        println!(
            "FINDING: Trail schema additionalProperties = {:?}",
            trail_addl
        );

        // --- Config: strict (deny_unknown_fields) ---
        #[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
        #[serde(deny_unknown_fields)]
        struct StrictConfig {
            url: String,
            token: String,
        }

        let config_schema = serde_json::to_value(schema_for!(StrictConfig)).unwrap();
        let config_validator = jsonschema::validator_for(&config_schema).unwrap();

        // Config with an extra unknown field
        let config_with_extra = json!({
            "url": "https://db.turso.io",
            "token": "secret",
            "typo_field": "should be rejected"
        });

        let config_accepts_extra = config_validator.is_valid(&config_with_extra);
        println!(
            "FINDING: Config (deny_unknown_fields) schema accepts unknown fields = {}",
            config_accepts_extra
        );

        // Check if additionalProperties is set
        let config_addl = config_schema.get("additionalProperties");
        println!(
            "FINDING: Config schema additionalProperties = {:?}",
            config_addl
        );

        // The convention: trail should accept unknowns, config should reject
        // Document the actual behavior regardless of whether it matches expectations
        if trail_accepts_extra && !config_accepts_extra {
            println!(
                "FINDING: Convention works as designed -- \
                 trail is permissive, config is strict."
            );
        } else if trail_accepts_extra && config_accepts_extra {
            println!(
                "FINDING: deny_unknown_fields does NOT generate additionalProperties: false \
                 in schemars. Config strictness must be enforced via serde deserialization only, \
                 not schema validation."
            );
        } else {
            println!(
                "FINDING: Unexpected behavior -- trail_accepts={}, config_accepts={}. \
                 Needs investigation.",
                trail_accepts_extra, config_accepts_extra
            );
        }
    }

    /// H7: serde-jsonlines roundtrip preserves version field and all data.
    /// Also: old-format lines (no v) read back with v defaulting to 1.
    #[test]
    fn spike_jsonlines_roundtrip_preserves_version() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("trail.jsonl");

        // Write versioned operations
        let ops = vec![
            TrailOperation {
                v: 1,
                ts: "2026-02-08T10:00:00Z".into(),
                ses: "ses-001".into(),
                op: TrailOp::Create,
                entity: EntityType::Finding,
                id: "fnd-001".into(),
                data: json!({"content": "test"}),
            },
            TrailOperation {
                v: 2,
                ts: "2026-02-08T11:00:00Z".into(),
                ses: "ses-001".into(),
                op: TrailOp::Update,
                entity: EntityType::Finding,
                id: "fnd-001".into(),
                data: json!({"confidence": {"level": "high", "basis": "tested"}}),
            },
        ];

        serde_jsonlines::write_json_lines(&path, &ops).unwrap();

        // Read back
        let read_ops: Vec<TrailOperation> = serde_jsonlines::json_lines(&path)
            .unwrap()
            .collect::<std::io::Result<Vec<_>>>()
            .unwrap();

        assert_eq!(read_ops.len(), 2);
        assert_eq!(read_ops[0].v, 1);
        assert_eq!(read_ops[1].v, 2);
        assert_eq!(read_ops[0].data["content"], "test");
        assert_eq!(read_ops[1].data["confidence"]["level"], "high");

        // Now test old-format: write raw JSON without `v` field, read back as TrailOperation
        let old_path = dir.path().join("old_trail.jsonl");
        let old_line_1 = r#"{"ts":"2026-02-08T09:00:00Z","ses":"ses-000","op":"create","entity":"finding","id":"fnd-000","data":{"content":"old"}}"#;
        let old_line_2 = r#"{"ts":"2026-02-08T09:30:00Z","ses":"ses-000","op":"update","entity":"task","id":"tsk-000","data":{"status":"done"}}"#;
        std::fs::write(&old_path, format!("{}\n{}\n", old_line_1, old_line_2)).unwrap();

        let old_ops: Vec<TrailOperation> = serde_jsonlines::json_lines(&old_path)
            .unwrap()
            .collect::<std::io::Result<Vec<_>>>()
            .unwrap();

        assert_eq!(old_ops.len(), 2);
        assert_eq!(old_ops[0].v, 1, "Old trail without v should default to 1");
        assert_eq!(old_ops[1].v, 1, "Old trail without v should default to 1");
        assert_eq!(old_ops[0].entity, EntityType::Finding);
        assert_eq!(old_ops[1].entity, EntityType::Task);
        assert_eq!(old_ops[0].data["content"], "old");

        // Append a v2 operation to the old file
        let new_op = TrailOperation {
            v: 2,
            ts: "2026-02-08T10:00:00Z".into(),
            ses: "ses-001".into(),
            op: TrailOp::Create,
            entity: EntityType::Finding,
            id: "fnd-new".into(),
            data: json!({"content": "new", "confidence": {"level": "high", "basis": "tested"}}),
        };
        serde_jsonlines::append_json_lines(&old_path, [&new_op]).unwrap();

        // Read mixed old+new file
        let mixed_ops: Vec<TrailOperation> = serde_jsonlines::json_lines(&old_path)
            .unwrap()
            .collect::<std::io::Result<Vec<_>>>()
            .unwrap();

        assert_eq!(mixed_ops.len(), 3);
        assert_eq!(mixed_ops[0].v, 1); // old, no v
        assert_eq!(mixed_ops[1].v, 1); // old, no v
        assert_eq!(mixed_ops[2].v, 2); // new, explicit v
    }
}
