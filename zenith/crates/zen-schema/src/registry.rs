//! Central schema registry for all Zenith types.
//!
//! The `SchemaRegistry` builds JSON Schemas from zen-core types at construction
//! time using [`schemars::schema_for!`] and provides validation via `jsonschema`.

use std::collections::HashMap;

use schemars::schema_for;

use crate::error::SchemaError;

/// Central store of all JSON Schemas in the Zenith system.
///
/// Built from zen-core types via [`schemars::schema_for!`]. Provides lookup
/// by name and validation of arbitrary JSON values against registered schemas.
pub struct SchemaRegistry {
    schemas: HashMap<&'static str, serde_json::Value>,
}

/// Insert a schema into the map, converting the `schemars` output to a
/// `serde_json::Value`. Panics if `serde_json::to_value` fails (should be
/// infallible for valid `schemars` output).
macro_rules! register {
    ($map:expr, $name:expr, $ty:ty) => {
        $map.insert($name, serde_json::to_value(schema_for!($ty)).unwrap());
    };
}

impl SchemaRegistry {
    /// Build a new registry containing all entity, trail, response, and
    /// audit-detail schemas from zen-core.
    ///
    /// # Panics
    ///
    /// Panics if `serde_json::to_value` fails on any `schemars`-generated
    /// schema. This is not expected in practice because `schemars` always
    /// produces valid JSON-serialisable output.
    #[must_use]
    pub fn new() -> Self {
        let mut schemas = HashMap::new();

        // --- Entity types (15) ---
        register!(schemas, "session", zen_core::entities::Session);
        register!(
            schemas,
            "session_snapshot",
            zen_core::entities::SessionSnapshot
        );
        register!(schemas, "research_item", zen_core::entities::ResearchItem);
        register!(schemas, "finding", zen_core::entities::Finding);
        register!(schemas, "hypothesis", zen_core::entities::Hypothesis);
        register!(schemas, "insight", zen_core::entities::Insight);
        register!(schemas, "issue", zen_core::entities::Issue);
        register!(schemas, "task", zen_core::entities::Task);
        register!(schemas, "impl_log", zen_core::entities::ImplLog);
        register!(schemas, "compat_check", zen_core::entities::CompatCheck);
        register!(schemas, "study", zen_core::entities::Study);
        register!(schemas, "entity_link", zen_core::entities::EntityLink);
        register!(schemas, "audit_entry", zen_core::entities::AuditEntry);
        register!(schemas, "project_meta", zen_core::entities::ProjectMeta);
        register!(
            schemas,
            "project_dependency",
            zen_core::entities::ProjectDependency
        );

        // --- Trail envelope (1) ---
        register!(schemas, "trail_operation", zen_core::trail::TrailOperation);

        // --- CLI response types (6) ---
        register!(
            schemas,
            "finding_create_response",
            zen_core::responses::FindingCreateResponse
        );
        register!(
            schemas,
            "session_start_response",
            zen_core::responses::SessionStartResponse
        );
        register!(
            schemas,
            "whats_next_response",
            zen_core::responses::WhatsNextResponse
        );
        register!(schemas, "search_result", zen_core::responses::SearchResult);
        register!(
            schemas,
            "search_results_response",
            zen_core::responses::SearchResultsResponse
        );
        register!(
            schemas,
            "rebuild_response",
            zen_core::responses::RebuildResponse
        );

        // --- Audit detail types (4) ---
        register!(
            schemas,
            "status_changed_detail",
            zen_core::audit_detail::StatusChangedDetail
        );
        register!(
            schemas,
            "linked_detail",
            zen_core::audit_detail::LinkedDetail
        );
        register!(
            schemas,
            "tagged_detail",
            zen_core::audit_detail::TaggedDetail
        );
        register!(
            schemas,
            "indexed_detail",
            zen_core::audit_detail::IndexedDetail
        );

        Self { schemas }
    }

    /// Get a schema by name. Returns `None` if not found.
    #[must_use]
    pub fn get(&self, name: &str) -> Option<&serde_json::Value> {
        self.schemas.get(name)
    }

    /// Validate a JSON value against a named schema.
    ///
    /// # Errors
    ///
    /// Returns `SchemaError::NotFound` if the schema name is unknown, or
    /// `SchemaError::ValidationFailed` if validation produces errors.
    pub fn validate(&self, name: &str, instance: &serde_json::Value) -> Result<(), SchemaError> {
        let schema = self
            .get(name)
            .ok_or_else(|| SchemaError::NotFound(name.to_string()))?;

        let validator = jsonschema::validator_for(schema)
            .map_err(|e| SchemaError::Generation(format!("{e}")))?;

        let errors: Vec<String> = validator
            .iter_errors(instance)
            .map(|e| format!("{e}"))
            .collect();

        if errors.is_empty() {
            Ok(())
        } else {
            Err(SchemaError::ValidationFailed { errors })
        }
    }

    /// List all registered schema names.
    #[must_use]
    pub fn list(&self) -> Vec<&'static str> {
        let mut names: Vec<&'static str> = self.schemas.keys().copied().collect();
        names.sort_unstable();
        names
    }

    /// Number of registered schemas.
    #[must_use]
    pub fn schema_count(&self) -> usize {
        self.schemas.len()
    }
}

impl Default for SchemaRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use zen_core::entities::Finding;
    use zen_core::enums::{Confidence, EntityType, TrailOp};
    use zen_core::trail::TrailOperation;

    fn registry() -> SchemaRegistry {
        SchemaRegistry::new()
    }

    #[test]
    fn registry_has_expected_count() {
        let reg = registry();
        // 15 entities + 1 trail + 6 responses + 4 audit details = 26
        assert_eq!(reg.schema_count(), 26);
    }

    #[test]
    fn registry_list_is_sorted() {
        let reg = registry();
        let names = reg.list();
        let mut sorted = names.clone();
        sorted.sort_unstable();
        assert_eq!(names, sorted);
    }

    #[test]
    fn get_existing_schema() {
        let reg = registry();
        assert!(reg.get("finding").is_some());
        assert!(reg.get("trail_operation").is_some());
        assert!(reg.get("status_changed_detail").is_some());
    }

    #[test]
    fn get_nonexistent_schema() {
        let reg = registry();
        assert!(reg.get("nonexistent").is_none());
    }

    #[test]
    fn validate_valid_finding() {
        let reg = registry();
        let finding = Finding {
            id: "fnd-test1234".into(),
            research_id: None,
            session_id: None,
            content: "Test finding".into(),
            source: None,
            confidence: Confidence::High,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        let json = serde_json::to_value(&finding).unwrap();
        assert!(reg.validate("finding", &json).is_ok());
    }

    #[test]
    fn validate_rejects_missing_required_field() {
        let reg = registry();
        let invalid = serde_json::json!({
            "id": "fnd-test",
            // "content" is missing
            "confidence": "high",
            "created_at": "2026-02-08T12:00:00Z",
            "updated_at": "2026-02-08T12:00:00Z"
        });
        let result = reg.validate("finding", &invalid);
        assert!(result.is_err());
        if let Err(SchemaError::ValidationFailed { errors }) = result {
            assert!(!errors.is_empty());
        } else {
            panic!("Expected ValidationFailed");
        }
    }

    #[test]
    fn validate_rejects_invalid_enum() {
        let reg = registry();
        let invalid = serde_json::json!({
            "id": "fnd-test",
            "content": "test",
            "confidence": "super_high",
            "created_at": "2026-02-08T12:00:00Z",
            "updated_at": "2026-02-08T12:00:00Z"
        });
        assert!(reg.validate("finding", &invalid).is_err());
    }

    #[test]
    fn validate_valid_trail_operation() {
        let reg = registry();
        let op = TrailOperation {
            v: 1,
            ts: "2026-02-08T12:00:00Z".into(),
            ses: "ses-00000000".into(),
            op: TrailOp::Create,
            entity: EntityType::Finding,
            id: "fnd-test1234".into(),
            data: serde_json::json!({"content": "test"}),
        };
        let json = serde_json::to_value(&op).unwrap();
        assert!(reg.validate("trail_operation", &json).is_ok());
    }

    #[test]
    fn validate_nonexistent_schema_returns_not_found() {
        let reg = registry();
        let result = reg.validate("bogus", &serde_json::json!({}));
        assert!(matches!(result, Err(SchemaError::NotFound(_))));
    }

    #[test]
    fn all_expected_schemas_present() {
        let reg = registry();
        let expected = [
            "session",
            "session_snapshot",
            "research_item",
            "finding",
            "hypothesis",
            "insight",
            "issue",
            "task",
            "impl_log",
            "compat_check",
            "study",
            "entity_link",
            "audit_entry",
            "project_meta",
            "project_dependency",
            "trail_operation",
            "finding_create_response",
            "session_start_response",
            "whats_next_response",
            "search_result",
            "search_results_response",
            "rebuild_response",
            "status_changed_detail",
            "linked_detail",
            "tagged_detail",
            "indexed_detail",
        ];
        for name in &expected {
            assert!(reg.get(name).is_some(), "Missing expected schema: {name}");
        }
    }
}
