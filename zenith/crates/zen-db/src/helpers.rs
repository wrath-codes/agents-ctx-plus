//! Row-to-entity parsing helpers.
//!
//! Every repo needs to convert `libsql::Row` (column-indexed) into typed entity
//! structs. These helpers isolate the parsing logic and handle the dual datetime
//! format issue (`SQLite`'s `datetime('now')` vs Rust's `to_rfc3339()`).

use chrono::{DateTime, Utc};

use crate::error::DatabaseError;

/// Parse a required TEXT column as `DateTime<Utc>`.
///
/// Handles both RFC 3339 (`"2026-02-09T14:30:00+00:00"`) and `SQLite`'s default
/// format (`"2026-02-09 14:30:00"`).
///
/// # Errors
///
/// Returns `DatabaseError::Query` if the string cannot be parsed as either format.
pub fn parse_datetime(s: &str) -> Result<DateTime<Utc>, DatabaseError> {
    if let Ok(dt) = DateTime::parse_from_rfc3339(s) {
        return Ok(dt.with_timezone(&Utc));
    }
    chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S")
        .map(|naive| naive.and_utc())
        .map_err(|e| DatabaseError::Query(format!("Failed to parse datetime '{s}': {e}")))
}

/// Parse an optional TEXT column as `Option<DateTime<Utc>>`.
///
/// # Errors
///
/// Returns `DatabaseError::Query` if a non-empty string cannot be parsed.
pub fn parse_optional_datetime(s: Option<&str>) -> Result<Option<DateTime<Utc>>, DatabaseError> {
    match s {
        Some(s) if !s.is_empty() => Ok(Some(parse_datetime(s)?)),
        _ => Ok(None),
    }
}

/// Parse a TEXT column into a serde-deserializable enum.
///
/// Works with all zen-core enums that use `#[serde(rename_all = "snake_case")]`.
///
/// # Errors
///
/// Returns `DatabaseError::Query` if the string does not match any enum variant.
pub fn parse_enum<T: serde::de::DeserializeOwned>(s: &str) -> Result<T, DatabaseError> {
    serde_json::from_value(serde_json::Value::String(s.to_string()))
        .map_err(|e| DatabaseError::Query(format!("Failed to parse enum from '{s}': {e}")))
}

/// Read a nullable TEXT column. Returns `None` for both SQL NULL and empty string.
///
/// `row.get::<String>(idx)` on a NULL column returns an error, not `""`.
/// You must use `get::<Option<String>>()` for nullable columns.
///
/// # Errors
///
/// Returns `DatabaseError` if the column read fails.
pub fn get_opt_string(row: &libsql::Row, idx: i32) -> Result<Option<String>, DatabaseError> {
    match row.get::<Option<String>>(idx)? {
        Some(s) if s.is_empty() => Ok(None),
        other => Ok(other),
    }
}

/// Extract an optional JSON value from a TEXT column.
///
/// # Errors
///
/// Returns `DatabaseError::Query` if a non-empty string contains invalid JSON.
pub fn parse_optional_json(s: Option<&str>) -> Result<Option<serde_json::Value>, DatabaseError> {
    match s {
        Some(s) if !s.is_empty() => {
            let val = serde_json::from_str(s)
                .map_err(|e| DatabaseError::Query(format!("Invalid JSON in column: {e}")))?;
            Ok(Some(val))
        }
        _ => Ok(None),
    }
}

/// Map `EntityType` to the corresponding SQL table name.
///
/// Uses exhaustive match — adding a new `EntityType` variant forces updating this.
#[must_use]
pub const fn entity_type_to_table(entity: &zen_core::enums::EntityType) -> &'static str {
    use zen_core::enums::EntityType;
    match entity {
        EntityType::Session => "sessions",
        EntityType::Research => "research_items",
        EntityType::Finding => "findings",
        EntityType::Hypothesis => "hypotheses",
        EntityType::Insight => "insights",
        EntityType::Issue => "issues",
        EntityType::Task => "tasks",
        EntityType::ImplLog => "implementation_log",
        EntityType::Compat => "compatibility_checks",
        EntityType::Study => "studies",
        EntityType::Decision => "decisions",
        EntityType::EntityLink => "entity_links",
        EntityType::Audit => "audit_trail",
    }
}
