# Phase 2: Storage Layer — Implementation Plan

**Version**: 2026-02-10 (rev 2 — spike 0.2g Option<T> params)
**Status**: Ready to Execute (12 review issues resolved, 21 spike tests added)
**Depends on**: Phase 1 (all tasks DONE — 127 tests pass across zen-core, zen-config, zen-db, zen-schema)
**Produces**: Milestone 2 — `cargo test -p zen-db` passes, 13 repo modules + JSONL trail + replayer all working

---

## Table of Contents

1. [Overview](#1-overview)
2. [Current State](#2-current-state)
3. [Key Decisions](#3-key-decisions)
4. [Architecture: ZenService Layer](#4-architecture-zenservice-layer)
5. [PR 1 — Stream A: Infrastructure](#5-pr-1--stream-a-infrastructure)
6. [PR 2 — Stream B: Entity Repos](#6-pr-2--stream-b-entity-repos)
7. [PR 3 — Stream C: Cross-Cutting + Replayer](#7-pr-3--stream-c-cross-cutting--replayer)
8. [Execution Order](#8-execution-order)
9. [Gotchas & Warnings](#9-gotchas--warnings)
10. [Milestone 2 Validation](#10-milestone-2-validation)
11. [Post-Review Amendments](#11-post-review-amendments)

---

## 1. Overview

**Goal**: CRUD operations for every entity, FTS5 search, audit trail, JSONL trail writer/replayer, session management, `whats_next()` aggregate query.

**Crate touched**: `zen-db` (heavy — all new code lives here)

**Dependency change**: `zen-db` gains `zen-schema` as a production dependency (for trail + audit validation).

**Estimated deliverables**: ~35 new files, ~5000–7000 LOC production code, ~3000 LOC tests

**PR strategy**: 3 PRs by stream. Each PR compiles and tests pass before merging.

| PR | Stream | Contents |
|----|--------|----------|
| PR 1 | A: Infrastructure | helpers, audit repo, trail writer, session repo |
| PR 2 | B: Entity Repos | 10 entity repos + update builders |
| PR 3 | C: Cross-Cutting | link repo, `whats_next()`, trail replayer, versioning |

---

## 2. Current State

| Component | Status | What Exists |
|-----------|--------|-------------|
| **zen-core** | DONE | 15 entity structs, 14 enums, `TrailOperation` envelope, error hierarchy, ID helpers, responses, audit details. 73 tests. |
| **zen-schema** | DONE | `SchemaRegistry` with 26 schemas, validation via `jsonschema` 0.28. 42 tests. |
| **zen-db** | Phase 1 DONE | `ZenDb` struct (`open_local`, `generate_id`, `conn()`), `001_initial.sql` (16 tables, 8 FTS5, 31 indexes, 22 triggers), `DatabaseError`. 12 production tests + 6 spike modules. |
| **Repos** | NOT STARTED | No `repos/` directory. No CRUD methods. Spike code in `spike_libsql.rs` has reference patterns. |
| **Trail I/O** | NOT STARTED | `TrailOperation` type exists in zen-core. `serde-jsonlines` is in Cargo.toml. No production writer/reader. |
| **Schema audit** | CLEAN | All 15 entity structs match SQL columns exactly (3 mismatches fixed: `issues.type` default, `implementation_log.task_id NOT NULL`, `project_dependencies.indexed NOT NULL`). |

---

## 3. Key Decisions

All decisions are backed by validated spike results.

### 3.1 Service Layer Wrapper (`ZenService`)

**Decision**: Create `ZenService` in `zen-db` that wraps `ZenDb` + `TrailWriter` + `SchemaRegistry`. All repo methods live as `impl ZenService`. `ZenDb` stays lean (raw DB access only).

**Rationale**: `ZenDb` currently holds `db: Database` + `conn: Connection`. Adding trail dir and schema registry would conflate DB access with file I/O and validation concerns. The service layer keeps `ZenDb` as a pure database handle while `ZenService` orchestrates the mutation protocol (SQL + audit + trail).

```rust
// src/service.rs
pub struct ZenService {
    db: ZenDb,
    trail: TrailWriter,
    schema: SchemaRegistry,
}

impl ZenService {
    pub fn db(&self) -> &ZenDb { &self.db }
    pub fn trail(&self) -> &TrailWriter { &self.trail }
    pub fn schema(&self) -> &SchemaRegistry { &self.schema }
}
```

### 3.2 Builder Pattern for Updates

**Decision**: Per-entity builder structs (`FindingUpdateBuilder`, `HypothesisUpdateBuilder`, etc.) that produce typed update structs. Only `Some` fields generate `SET` clauses in SQL. Builder output is serialized as the trail `data` payload (only changed fields).

**Rationale**: Type-safe, IDE-friendly, and the serialized builder output naturally matches the trail's "changed fields only" convention from spike 0.12. A builder is more ergonomic than raw `Option`-field structs for the CLI layer.

```rust
// src/updates/finding.rs
#[derive(Debug, Clone, Default, Serialize)]
pub struct FindingUpdate {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<Option<String>>,  // Some(None) = set to NULL
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confidence: Option<Confidence>,
}

pub struct FindingUpdateBuilder(FindingUpdate);

impl FindingUpdateBuilder {
    pub fn new() -> Self { Self(FindingUpdate::default()) }
    pub fn content(mut self, val: impl Into<String>) -> Self {
        self.0.content = Some(val.into()); self
    }
    pub fn source(mut self, val: Option<String>) -> Self {
        self.0.source = Some(val); self
    }
    pub fn confidence(mut self, val: Confidence) -> Self {
        self.0.confidence = Some(val); self
    }
    pub fn build(self) -> FindingUpdate { self.0 }
}
```

**`Option<Option<T>>` pattern**: For nullable columns, the outer `Option` means "was this field specified?" and the inner `Option` means "set to value or NULL?". `None` = don't change. `Some(Some("val"))` = set to "val". `Some(None)` = set to NULL.

### 3.3 Serialize Builder Output for Trail Data

**Decision**: When a mutation uses a builder, the `FindingUpdate` struct (with `#[serde(skip_serializing_if = "Option::is_none")]`) is serialized as the trail operation's `data` field. Only changed fields appear in the JSONL.

**Rationale**: Validated in spike 0.12 — trail `data` for updates contains only changed fields, not full entity snapshots. The builder's `skip_serializing_if` annotation produces exactly this shape. No extra DB read needed.

```jsonl
{"v":1,"ts":"2026-02-09T14:30:00Z","ses":"ses-001","op":"update","entity":"finding","id":"fnd-abc123","data":{"confidence":"high"}}
```

### 3.4 Helper Functions for Row Mapping

**Decision**: Standalone helper functions (`parse_datetime`, `parse_enum`, `get_opt_string`) rather than macros or a `FromRow` trait.

**Rationale**: 13 entities means significant row-mapping boilerplate. Helper functions are verbose but debuggable — `libsql::Row` returns `Result<T>` by column index, and debugging positional mismatches in a macro is painful. The helpers isolate the parsing logic while keeping each entity's mapping explicit.

### 3.5 Trail Writer Disabled During Rebuild

**Decision**: `TrailWriter` has an `enabled: bool` flag. During `TrailReplayer::rebuild()`, the writer is disabled to avoid re-writing operations that are being replayed. Audit entries are also skipped during rebuild.

**Rationale**: Validated in spike 0.12 — rebuild reads JSONL and replays SQL operations. If the writer were active, every replayed operation would append duplicate JSONL lines. FTS5 triggers fire automatically on INSERT during replay, so no manual FTS work is needed.

### 3.6 zen-schema as Production Dependency of zen-db

**Decision**: Add `zen-schema.workspace = true` to `zen-db/Cargo.toml` `[dependencies]`.

**Rationale**: Tasks 2.12 (audit detail validation) and 2.15 (trail data validation) require `SchemaRegistry` at runtime. The dependency direction is safe: `zen-schema → zen-core` and `zen-db → zen-schema → zen-core`. No circular dependency.

### 3.7 Validation Traceability Matrix

This matrix makes validation provenance explicit for the highest-risk Phase 2 behaviors.

**Legend**:
- **Validated**: behavior is directly proven by spike tests
- **Design-only**: chosen implementation pattern, not yet proven end-to-end in Phase 2 integration tests

| Area | Claim | Status | Spike/Test Evidence | Source |
|------|-------|--------|---------------------|--------|
| JSONL replay | DB can be rebuilt from trail operations | Validated | `spike_jsonl_replay_rebuild` | `zen-db/src/spike_jsonl.rs:648` |
| Replay + FTS | FTS indexes survive rebuild via triggers | Validated | `spike_jsonl_rebuild_fts` | `zen-db/src/spike_jsonl.rs:710` |
| Concurrent trail writes | Per-session files append without corruption | Validated | `spike_jsonl_concurrent_append` | `zen-db/src/spike_jsonl.rs:908` |
| Study full state | Study graph hydration works from entity links | Validated | `spike_b_query_full_state` | `zen-db/src/spike_studies.rs:1172` |
| Study progress | Progress aggregation over assumptions/tests works | Validated | `spike_b_progress_tracking` | `zen-db/src/spike_studies.rs:1321` |
| libsql nullable reads | NULL columns require nullable extraction path | Validated | `spike_null_handling` | `zen-db/src/spike_libsql.rs:537` |
| Trail envelope versioning | Missing `v` defaults to `1` | Validated | `spike_v_field_defaults_to_1_when_absent` | `zen-schema/src/spike_trail_versioning.rs:70` |
| Replay version dispatch | `match op.v` routes known versions and rejects unknown | Validated | `spike_replay_dispatch_routes_by_version` | `zen-schema/src/spike_trail_versioning.rs:423` |
| Audit detail dispatch | Per-action detail schema validation works | Validated | `spike_schema_audit_detail_dispatch` | `zen-schema/src/spike_schema_gen.rs:1430` |
| NULL binding for FK columns | `unwrap_or("")` violates FK constraints; `Value::Null` is correct | Validated | `spike_empty_string_violates_fk_constraint`, `spike_replay_unwrap_or_empty_breaks_fk` | `zen-db/src/spike_libsql.rs:798` |
| Dynamic update params | `Vec<libsql::Value>` + `params_from_iter()` works for dynamic SET clauses | Validated | `spike_dynamic_update_with_params_from_iter`, `spike_vec_value_directly_as_params` | `zen-db/src/spike_libsql.rs:1000` |
| Update set NULL | Dynamic UPDATE can set nullable column to SQL NULL | Validated | `spike_dynamic_update_set_null_with_params_from_iter` | `zen-db/src/spike_libsql.rs:1100` |
| Transaction + trail atomicity | Trail failure → DB rollback, no orphaned state | Validated | `spike_transaction_rollback_on_trail_failure`, `spike_transaction_implicit_rollback_on_drop` | `zen-db/src/spike_libsql.rs:1210` |
| Full mutation protocol | BEGIN → SQL → audit → trail → COMMIT end-to-end | Validated | `spike_full_mutation_protocol_with_file_trail` | `zen-db/src/spike_libsql.rs:1400` |
| Replay null vs absent | JSON null = set NULL, absent key = skip (no ambiguity) | Validated | `spike_replay_null_vs_absent_vs_value`, `spike_option_option_serde_roundtrip_for_replay` | `zen-db/src/spike_libsql.rs:1560` |
| Concurrent same-session writes | 8 tasks × 50 ops to same JSONL file, 0 corruption | Validated | `spike_concurrent_same_session_file_append` | `zen-db/src/spike_libsql.rs:1820` |
| SQL injection surface | EntityType enum → `&'static str` table mapping is exhaustive and safe | Validated | `spike_entity_type_table_mapping_is_exhaustive`, `spike_count_by_status_with_enum_is_safe` | `zen-db/src/spike_libsql.rs:1900` |
| Mutation orchestration | BEGIN → SQL + audit → trail → COMMIT in `ZenService` | Validated | `spike_full_mutation_protocol_with_file_trail` | `zen-db/src/spike_libsql.rs:1400` |
| Option<T> in params! | `Option<&str>` and `Option<String>` work natively in `params!` macro | Validated | `spike_option_works_natively_in_params_macro`, `spike_option_string_owned_works_in_params_macro` | `zen-db/src/spike_libsql.rs:1997` |
| Option<T> with .as_deref() | `Option<String>` fields use `.as_deref()` in params! for repo methods | Validated | `spike_option_as_deref_pattern_for_repos` | `zen-db/src/spike_libsql.rs:2097` |
| named_params! with Option | `named_params!` macro handles Option<T> same as `params!` | Validated | `spike_named_params_with_option` | `zen-db/src/spike_libsql.rs:2195` |
| Vec<Value> for dynamic updates | `Vec<Value>` + `.into()` still needed for dynamic SET clause count | Validated | `spike_vec_value_needed_for_dynamic_update_builders` | `zen-db/src/spike_libsql.rs:2233` |

### 3.8 Native `Option<T>` in `params!` Macro

**Decision**: Use `libsql::params!` with `Option<T>` for all fixed-param queries (INSERT, SELECT with params). Use `Vec<libsql::Value>` only for dynamic UPDATE builders where param count varies at runtime.

**Rationale**: libsql 0.9.29 has `impl<T: Into<Value>> From<Option<T>> for Value` which maps `None → Value::Null` and `Some(v) → v.into()`. The `IntoValue` trait has a blanket impl for anything `TryInto<Value>`, so `Option<String>` and `Option<&str>` work directly in the `params!` and `named_params!` macros. This was missed in the original plan because GitHub issue #278 (filed against v0.1.6) showed `Option` wasn't supported — but it was fixed in later versions.

**Write-side pattern** (repo INSERT methods):
```rust
// Use .as_deref() to convert Option<String> → Option<&str>
libsql::params![
    finding.id.as_str(),
    finding.research_id.as_deref(),  // None → NULL, Some("res-001") → "res-001"
    session_id,
    finding.content.as_str(),
    finding.source.as_deref(),       // None → NULL
    finding.confidence.as_str()
]
```

**Update-side pattern** (dynamic SET clauses — still requires `Vec<Value>`):
```rust
let mut params: Vec<libsql::Value> = Vec::new();
if let Some(ref content) = update.content {
    clauses.push(format!("content = ?{idx}"));
    params.push(content.as_str().into());
    idx += 1;
}
if let Some(ref source) = update.source {
    clauses.push(format!("source = ?{idx}"));
    params.push(source.as_deref().into());  // Option<&str>.into() → Value::Null or Value::Text
    idx += 1;
}
```

**Eliminates**: `opt_to_value()` helper for the write side. The helper was only needed because the plan assumed `params!` couldn't handle `Option`. For `Vec<Value>` construction (dynamic updates), `.into()` on `Option<&str>` replaces the manual match arm.

**Still needed**: `json_to_value()` and `json_to_update_value()` helpers for the replay side (converting `serde_json::Value` fields from JSONL trail data — `Option<T>` native support doesn't help here because the source is JSON, not Rust types).

Validated in spike 0.2g: 6 tests (`spike_option_works_natively_in_params_macro`, `spike_option_string_owned_works_in_params_macro`, `spike_option_as_deref_pattern_for_repos`, `spike_named_params_with_option`, `spike_vec_value_needed_for_dynamic_update_builders`).

---

## 4. Architecture: ZenService Layer

### Module Structure (Final)

```
zen-db/src/
├── lib.rs              # ZenDb struct (unchanged from Phase 1), mod declarations
├── error.rs            # DatabaseError (unchanged from Phase 1)
├── migrations.rs       # run_migrations (unchanged from Phase 1)
├── service.rs          # ZenService = ZenDb + TrailWriter + SchemaRegistry
├── helpers.rs          # Row-to-entity parsing helpers
├── repos/
│   ├── mod.rs          # mod declarations for all repo modules
│   ├── audit.rs        # append_audit(), query_audit(), search_audit()
│   ├── session.rs      # start_session(), end_session(), list_sessions(), etc.
│   ├── research.rs     # CRUD + FTS
│   ├── finding.rs      # CRUD + tag/untag + FTS
│   ├── hypothesis.rs   # CRUD + status transitions
│   ├── insight.rs      # CRUD + FTS
│   ├── issue.rs        # CRUD + FTS + parent-child
│   ├── task.rs         # CRUD + FTS + issue linkage
│   ├── impl_log.rs     # CRUD + file path queries
│   ├── compat.rs       # CRUD + package pair queries
│   ├── project.rs      # meta CRUD, dependency CRUD
│   ├── study.rs        # CRUD + FTS + progress + conclude + full state
│   ├── link.rs         # create, delete, query by source/target
│   └── whats_next.rs   # Aggregate query
├── trail/
│   ├── mod.rs          # pub mod writer, replayer
│   ├── writer.rs       # TrailWriter: append + validate
│   └── replayer.rs     # TrailReplayer: rebuild from JSONL
└── updates/
    ├── mod.rs          # Re-exports all update types + builders
    ├── finding.rs      # FindingUpdate + FindingUpdateBuilder
    ├── hypothesis.rs   # HypothesisUpdate + HypothesisUpdateBuilder
    ├── research.rs     # ResearchUpdate + ResearchUpdateBuilder
    ├── insight.rs      # InsightUpdate + InsightUpdateBuilder
    ├── issue.rs        # IssueUpdate + IssueUpdateBuilder
    ├── task.rs         # TaskUpdate + TaskUpdateBuilder
    ├── compat.rs       # CompatUpdate + CompatUpdateBuilder
    └── study.rs        # StudyUpdate + StudyUpdateBuilder
```

### Mutation Protocol

Every mutation method on `ZenService` follows this exact sequence:

```rust
impl ZenService {
    pub async fn create_finding(
        &self,
        session_id: &str,
        finding: &Finding,
    ) -> Result<(), DatabaseError> {
        let tx = self.db.conn().transaction().await?;

        // 1. Execute SQL (Option<T> natively maps to NULL via params! macro)
        tx.execute(
            "INSERT INTO findings (id, research_id, session_id, content, source, confidence, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            libsql::params![
                finding.id.as_str(),
                finding.research_id.as_deref(),
                session_id,
                finding.content.as_str(),
                finding.source.as_deref(),
                finding.confidence.as_str(),
                finding.created_at.to_rfc3339(),
                finding.updated_at.to_rfc3339()
            ],
        ).await?;

        // 2. Append audit entry (inside same transaction)
        tx.execute(
            "INSERT INTO audit_trail (id, session_id, entity_type, entity_id, action, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            libsql::params![
                self.db.generate_id(PREFIX_AUDIT).await?,
                session_id,
                EntityType::Finding.as_str(),
                finding.id.as_str(),
                AuditAction::Created.as_str(),
                Utc::now().to_rfc3339(),
            ],
        ).await?;

        // 3. Append trail operation (file I/O — before DB commit)
        self.trail.append(&TrailOperation {
            v: 1,
            ts: Utc::now().to_rfc3339(),
            ses: session_id.to_string(),
            op: TrailOp::Create,
            entity: EntityType::Finding,
            id: finding.id.clone(),
            data: serde_json::to_value(finding)?,
        })?;

        // 4. Commit DB transaction
        tx.commit().await?;

        Ok(())
    }
}
```

---

## 5. PR 1 — Stream A: Infrastructure

Must be implemented first. All entity repos depend on these components.

Validation status:
- **Validated components**: helper parsing edge cases (`spike_null_handling`), JSONL append/concurrency behavior (`spike_jsonl_concurrent_append`), schema dispatch for audit/trail (`spike_schema_audit_detail_dispatch`)
- **Design-only components**: full `ZenService` orchestration across all mutation paths (to be validated by PR 1 + PR 2 integration tests)

### A1. Cargo.toml Update

Add `zen-schema` as a production dependency:

```toml
[dependencies]
zen-schema.workspace = true   # NEW — for trail + audit validation
```

### A2. `src/helpers.rs` — Row-to-Entity Parsing Helpers

Every repo needs to convert `libsql::Row` (column-indexed) into typed entity structs. These helpers isolate the parsing logic.

**Validated in**: Phase 1 test code (`insert_and_select_finding`, etc.) uses raw `row.get::<String>(0)`. These helpers formalize the pattern.

```rust
use chrono::{DateTime, Utc};
use crate::error::DatabaseError;

/// Parse a required TEXT column as DateTime<Utc>.
///
/// libSQL stores datetimes as TEXT in RFC 3339 / SQLite datetime('now') format.
/// SQLite's `datetime('now')` produces `"2026-02-09 14:30:00"` (space-separated,
/// no timezone), while Rust's `to_rfc3339()` produces `"2026-02-09T14:30:00+00:00"`.
/// We handle both formats.
pub fn parse_datetime(s: &str) -> Result<DateTime<Utc>, DatabaseError> {
    // Try RFC 3339 first (what our Rust code writes)
    if let Ok(dt) = DateTime::parse_from_rfc3339(s) {
        return Ok(dt.with_timezone(&Utc));
    }
    // Fall back to SQLite's default format: "2026-02-09 14:30:00"
    chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S")
        .map(|naive| naive.and_utc())
        .map_err(|e| DatabaseError::Query(format!("Failed to parse datetime '{s}': {e}")))
}

/// Parse an optional TEXT column as Option<DateTime<Utc>>.
pub fn parse_optional_datetime(s: Option<String>) -> Result<Option<DateTime<Utc>>, DatabaseError> {
    match s {
        Some(ref s) if !s.is_empty() => Ok(Some(parse_datetime(s)?)),
        _ => Ok(None),
    }
}

/// Parse a TEXT column into a serde-deserializable enum.
///
/// Works with all zen-core enums that use `#[serde(rename_all = "snake_case")]`.
/// E.g., `parse_enum::<Confidence>("high")` returns `Confidence::High`.
pub fn parse_enum<T: serde::de::DeserializeOwned>(s: &str) -> Result<T, DatabaseError> {
    serde_json::from_value(serde_json::Value::String(s.to_string()))
        .map_err(|e| DatabaseError::Query(format!("Failed to parse enum from '{s}': {e}")))
}

/// Read a nullable TEXT column. Returns None for both SQL NULL and empty string.
///
/// Two cases produce None:
/// 1. Column is SQL NULL → `row.get::<Option<String>>()` returns `Ok(None)`
/// 2. Column is "" (empty string stored by `unwrap_or("")` pattern) → normalized to None
///
/// IMPORTANT: `row.get::<String>(idx)` on a NULL column returns an ERROR,
/// not "". You MUST use `get::<Option<String>>()` for nullable columns.
/// This was confirmed in spike 0.2 (`spike_null_handling` test, line 577).
///
/// This normalization is safe because no Zenith entity field is meaningfully "".
/// See gotcha §9.4 for full explanation.
pub fn get_opt_string(row: &libsql::Row, idx: i32) -> Result<Option<String>, DatabaseError> {
    match row.get::<Option<String>>(idx)? {
        Some(s) if s.is_empty() => Ok(None),
        other => Ok(other),
    }
}

/// Extract an optional JSON value from a TEXT column.
///
/// Used for `audit_trail.detail` which stores JSON as TEXT.
pub fn parse_optional_json(s: Option<String>) -> Result<Option<serde_json::Value>, DatabaseError> {
    match s {
        Some(ref s) if !s.is_empty() => {
            let val = serde_json::from_str(s)
                .map_err(|e| DatabaseError::Query(format!("Invalid JSON in column: {e}")))?;
            Ok(Some(val))
        }
        _ => Ok(None),
    }
}
```

**Gotcha — SQLite datetime format**: SQLite's `datetime('now')` produces `"2026-02-09 14:30:00"` (no `T`, no timezone), but Rust's `Utc::now().to_rfc3339()` produces `"2026-02-09T14:30:00+00:00"`. The helper must handle both because:
- Rows created by SQL DEFAULTs use SQLite format
- Rows created by Rust code use RFC 3339 format
- Rows replayed from JSONL use RFC 3339 format

**Gotcha — libsql NULL handling**: `row.get::<String>(idx)` on a NULL column **returns an error**, not `""`. You must use `row.get::<Option<String>>(idx)`. The write side now uses `params!` with `Option<T>` which sends proper SQL NULL, so empty-string normalization in `get_opt_string` is only a safety net for legacy data.

**Proof snippet (`spike_null_handling`)**:

```rust
let row1 = rows.next().await.unwrap().unwrap();
assert_eq!(row1.get::<String>(0).unwrap(), "tsk-001");
let issue_val = row1.get_value(1).unwrap();
assert!(matches!(issue_val, libsql::Value::Null));
```

### A3. `src/repos/audit.rs` — AuditRepo

**Implements task 2.12**: append (every repo method calls this), query with filters.

**Validated in**: spike 0.2 (`spike_libsql.rs`) tested raw INSERT/SELECT for `audit_trail`. Spike 0.15 validated audit detail schema validation.

**Source**: `01-turso-data-model.md` §8 (audit trail table + actions), `05-crate-designs.md` §5 (repo pattern).

```rust
use zen_core::entities::AuditEntry;
use zen_core::enums::{AuditAction, EntityType};
use zen_core::ids::PREFIX_AUDIT;

impl ZenService {
    /// Append an audit entry. Called by every mutation method.
    ///
    /// Generates the audit ID internally. Optionally validates the `detail`
    /// payload against the per-action schema from SchemaRegistry.
    pub async fn append_audit(&self, entry: &AuditEntry) -> Result<(), DatabaseError> {
        self.db.conn().execute(
            "INSERT INTO audit_trail (id, session_id, entity_type, entity_id, action, detail, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            libsql::params![
                entry.id.as_str(),
                entry.session_id.as_deref(),
                entry.entity_type.as_str(),
                entry.entity_id.as_str(),
                entry.action.as_str(),
                entry.detail.as_ref().map(|d| d.to_string()).as_deref(),
                entry.created_at.to_rfc3339()
            ],
        ).await?;
        Ok(())
    }

    /// Query audit entries with optional filters.
    pub async fn query_audit(&self, filter: &AuditFilter) -> Result<Vec<AuditEntry>, DatabaseError> {
        // Build WHERE clauses dynamically based on which filters are set
        let mut conditions = Vec::new();
        let mut params: Vec<libsql::Value> = Vec::new();

        if let Some(ref et) = filter.entity_type {
            params.push(libsql::Value::Text(et.as_str().to_string()));
            conditions.push(format!("entity_type = ?{}", params.len()));
        }
        if let Some(ref eid) = filter.entity_id {
            params.push(libsql::Value::Text(eid.clone()));
            conditions.push(format!("entity_id = ?{}", params.len()));
        }
        if let Some(ref action) = filter.action {
            params.push(libsql::Value::Text(action.as_str().to_string()));
            conditions.push(format!("action = ?{}", params.len()));
        }
        if let Some(ref sid) = filter.session_id {
            params.push(libsql::Value::Text(sid.clone()));
            conditions.push(format!("session_id = ?{}", params.len()));
        }

        let where_clause = if conditions.is_empty() {
            String::new()
        } else {
            format!("WHERE {}", conditions.join(" AND "))
        };

        let limit = filter.limit.unwrap_or(100);
        let sql = format!(
            "SELECT id, session_id, entity_type, entity_id, action, detail, created_at
             FROM audit_trail {where_clause}
             ORDER BY created_at DESC LIMIT {limit}"
        );

        // Execute and map rows to AuditEntry structs using helpers
        // ...
    }

    /// FTS5 search across audit entries.
    ///
    /// SQL: `audit_fts MATCH ?1 JOIN audit_trail ON rowid`
    /// Source: 01-turso-data-model.md §9 (FTS5 query pattern)
    pub async fn search_audit(
        &self,
        query: &str,
        limit: u32,
    ) -> Result<Vec<AuditEntry>, DatabaseError> {
        let mut rows = self.db.conn().query(
            "SELECT a.id, a.session_id, a.entity_type, a.entity_id, a.action, a.detail, a.created_at
             FROM audit_fts
             JOIN audit_trail a ON a.rowid = audit_fts.rowid
             WHERE audit_fts MATCH ?1
             ORDER BY rank LIMIT ?2",
            libsql::params![query, limit.to_string().as_str()],
        ).await?;
        // Map rows...
    }
}

/// Filter criteria for audit queries.
#[derive(Default)]
pub struct AuditFilter {
    pub entity_type: Option<EntityType>,
    pub entity_id: Option<String>,
    pub action: Option<AuditAction>,
    pub session_id: Option<String>,
    pub limit: Option<u32>,
}
```

### A4. `src/trail/writer.rs` — Trail Writer

**Implements task 2.15**: Append operations to per-session `.zenith/trail/ses-xxx.jsonl` on every mutation. Validate `Operation.data` against per-entity schema from zen-schema before writing.

**Validated in**: Spike 0.12 (15/15 tests) confirmed `serde-jsonlines` API, per-session file isolation, and concurrent safety. Spike 0.15 (22/22) confirmed per-entity schema dispatch. Spike 0.16 (10/10) confirmed versioning envelope.

**Source**: `10-git-jsonl-strategy.md` (Approach B — JSONL as source of truth), `14-trail-versioning-spike-plan.md` (Approach D — `v` field).

```rust
use std::path::{Path, PathBuf};
use zen_core::trail::TrailOperation;
use zen_core::enums::EntityType;
use zen_schema::SchemaRegistry;

pub struct TrailWriter {
    trail_dir: PathBuf,
    enabled: bool,
}

impl TrailWriter {
    /// Create a new TrailWriter pointing at the given directory.
    ///
    /// The directory is typically `.zenith/trail/`.
    /// Creates the directory if it doesn't exist.
    pub fn new(trail_dir: PathBuf) -> Result<Self, DatabaseError> {
        std::fs::create_dir_all(&trail_dir)
            .map_err(|e| DatabaseError::Other(e.into()))?;
        Ok(Self { trail_dir, enabled: true })
    }

    /// Create a disabled writer (for testing or when trail is not configured).
    pub fn disabled() -> Self {
        Self {
            trail_dir: PathBuf::new(),
            enabled: false,
        }
    }

    /// Disable writing (used during rebuild to avoid re-writing replayed ops).
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    /// Append a trail operation to the session's JSONL file.
    ///
    /// File path: `{trail_dir}/{op.ses}.jsonl`
    ///
    /// Uses `serde_jsonlines::append_json_lines` which:
    /// - Creates the file if it doesn't exist
    /// - Appends a single JSON line (no multi-line JSON)
    /// - Is atomic per line (concurrent-safe for separate session files)
    ///
    /// Validated in spike 0.12: 4 agents, 100 ops, zero corruption.
    pub fn append(&self, op: &TrailOperation) -> Result<(), DatabaseError> {
        if !self.enabled {
            return Ok(());
        }

        let path = self.trail_dir.join(format!("{}.jsonl", op.ses));
        serde_jsonlines::append_json_lines(&path, [op])
            .map_err(|e| DatabaseError::Other(e.into()))?;
        Ok(())
    }

    /// Append with schema validation of the `data` field.
    ///
    /// Dispatches validation by `op.entity`:
    /// - EntityType::Finding  → validate against "finding" schema
    /// - EntityType::Session  → validate against "session" schema
    /// - etc.
    ///
    /// Only validates for `Create` ops (full entity data).
    /// `Update` ops contain partial data and are not validated against
    /// the full entity schema (they'd fail required field checks).
    ///
    /// Validated in spike 0.15: per-entity data dispatch works correctly.
    /// Wrong entity data submitted as the wrong type fails with descriptive errors.
    pub fn append_validated(
        &self,
        op: &TrailOperation,
        schema: &SchemaRegistry,
    ) -> Result<(), DatabaseError> {
        if !self.enabled {
            return Ok(());
        }

        // Only validate full entity data on Create ops
        if op.op == zen_core::enums::TrailOp::Create {
            let schema_name = entity_type_to_schema_name(&op.entity);
            if let Err(e) = schema.validate(schema_name, &op.data) {
                tracing::warn!(
                    "Trail validation failed for {} {}: {:?}",
                    op.entity, op.id, e
                );
                // Warn but don't block — permissive for forward-compat
                // (spike 0.16 decision: trails are permissive)
            }
        }

        self.append(op)
    }

    pub fn trail_dir(&self) -> &Path {
        &self.trail_dir
    }
}

/// Map EntityType to schema registry name.
///
/// EntityType::Finding → "finding"
/// EntityType::ImplLog → "impl_log"
/// etc.
fn entity_type_to_schema_name(entity: &EntityType) -> &'static str {
    match entity {
        EntityType::Session => "session",
        EntityType::Research => "research_item",
        EntityType::Finding => "finding",
        EntityType::Hypothesis => "hypothesis",
        EntityType::Insight => "insight",
        EntityType::Issue => "issue",
        EntityType::Task => "task",
        EntityType::ImplLog => "impl_log",
        EntityType::Compat => "compat_check",
        EntityType::Study => "study",
        EntityType::EntityLink => "entity_link",
        EntityType::Audit => "audit_entry",
    }
}
```

**Key design choice — warn-only validation**: Trail writes use permissive validation (warn, don't block). This follows the spike 0.16 decision: trails must not have `additionalProperties: false`, and forward-compat requires accepting unknown fields. The `--strict` flag on `znt rebuild` is the enforcement point.

**Proof snippet (`spike_jsonl_concurrent_append`)**:

```rust
let handles: Vec<_> = (0..4).map(|agent| {
    tokio::spawn(async move { /* append ops to ses-{agent}.jsonl */ })
}).collect();

for h in handles { h.await.unwrap(); }
// Expect zero corruption and per-session file isolation.
```

### A5. `src/service.rs` — ZenService Struct

```rust
use crate::ZenDb;
use crate::error::DatabaseError;
use crate::trail::writer::TrailWriter;
use zen_schema::SchemaRegistry;
use std::path::PathBuf;

/// Orchestrates database mutations with audit trail and JSONL trail.
///
/// Every mutation method:
/// 1. Executes SQL via `self.db.conn()`
/// 2. Appends to audit trail via `self.append_audit()`
/// 3. Appends to JSONL trail via `self.trail.append()`
pub struct ZenService {
    db: ZenDb,
    trail: TrailWriter,
    schema: SchemaRegistry,
}

impl ZenService {
    /// Create a new service wrapping a local database.
    ///
    /// `trail_dir` is typically `.zenith/trail/`. Pass `None` to disable
    /// trail writing (for tests that don't need trail files).
    pub async fn new_local(
        db_path: &str,
        trail_dir: Option<PathBuf>,
    ) -> Result<Self, DatabaseError> {
        let db = ZenDb::open_local(db_path).await?;
        let trail = match trail_dir {
            Some(dir) => TrailWriter::new(dir)?,
            None => TrailWriter::disabled(),
        };
        let schema = SchemaRegistry::new();
        Ok(Self { db, trail, schema })
    }

    /// Create from an existing ZenDb (for testing).
    pub fn from_db(db: ZenDb, trail: TrailWriter) -> Self {
        Self {
            db,
            trail,
            schema: SchemaRegistry::new(),
        }
    }

    /// Access the underlying database handle.
    pub fn db(&self) -> &ZenDb { &self.db }

    /// Access the trail writer (e.g., to disable during rebuild).
    pub fn trail_mut(&mut self) -> &mut TrailWriter { &mut self.trail }

    /// Access the schema registry.
    pub fn schema(&self) -> &SchemaRegistry { &self.schema }
}
```

### A6. `src/repos/session.rs` — SessionRepo

**Implements task 2.1**: start, end, list, snapshot, orphan detection.

**Validated in**: Spike 0.2 tested basic session INSERT/SELECT. Session lifecycle in `01-turso-data-model.md` §2 and §13 (sync strategy — orphan detection on startup).

```rust
use zen_core::entities::{Session, SessionSnapshot};
use zen_core::enums::{SessionStatus, AuditAction, EntityType};
use zen_core::ids::PREFIX_SESSION;
use chrono::Utc;

impl ZenService {
    /// Start a new session. Detects orphaned active sessions first.
    ///
    /// 1. Check for active sessions (orphan detection)
    /// 2. Mark orphans as abandoned
    /// 3. Create new session with status='active'
    /// 4. Write audit entry (SessionStart)
    /// 5. Write trail operation (Create)
    ///
    /// Returns the new session and any previous active session that was abandoned.
    ///
    /// Source: 01-turso-data-model.md §13 — "next `znt session start` detects
    /// the orphaned active session, marks it as `abandoned`"
    pub async fn start_session(&self) -> Result<(Session, Option<Session>), DatabaseError> {
        let now = Utc::now();
        let id = self.db.generate_id(PREFIX_SESSION).await?;

        // Orphan detection: find any sessions still in 'active' status
        let orphaned = self.detect_orphan_sessions().await?;
        for orphan in &orphaned {
            self.abandon_session(&orphan.id).await?;
        }

        // Create new session
        self.db.conn().execute(
            "INSERT INTO sessions (id, started_at, status) VALUES (?1, ?2, 'active')",
            libsql::params![id.as_str(), now.to_rfc3339().as_str()],
        ).await?;

        let session = Session {
            id: id.clone(),
            started_at: now,
            ended_at: None,
            status: SessionStatus::Active,
            summary: None,
        };

        // Audit + trail
        self.append_audit(/* ... SessionStart ... */).await?;
        self.trail.append(/* ... Create session ... */)?;

        Ok((session, orphaned.into_iter().next()))
    }

    /// End a session (wrap-up).
    ///
    /// Validates transition: Active → WrappedUp.
    /// Sets ended_at and summary.
    ///
    /// Source: enums.rs — SessionStatus::Active.can_transition_to(WrappedUp)
    pub async fn end_session(
        &self,
        session_id: &str,
        summary: &str,
    ) -> Result<Session, DatabaseError> {
        let current = self.get_session(session_id).await?;

        if !current.status.can_transition_to(SessionStatus::WrappedUp) {
            return Err(DatabaseError::InvalidState(format!(
                "Cannot transition session {} from {} to wrapped_up",
                session_id, current.status
            )));
        }

        let now = Utc::now();
        self.db.conn().execute(
            "UPDATE sessions SET ended_at = ?1, status = 'wrapped_up', summary = ?2 WHERE id = ?3",
            libsql::params![now.to_rfc3339().as_str(), summary, session_id],
        ).await?;

        // Audit (SessionEnd) + trail (Transition)
        // ...

        Ok(Session {
            ended_at: Some(now),
            status: SessionStatus::WrappedUp,
            summary: Some(summary.to_string()),
            ..current
        })
    }

    /// Get a session by ID.
    pub async fn get_session(&self, id: &str) -> Result<Session, DatabaseError> {
        let mut rows = self.db.conn().query(
            "SELECT id, started_at, ended_at, status, summary FROM sessions WHERE id = ?1",
            [id],
        ).await?;
        let row = rows.next().await?.ok_or(DatabaseError::NoResult)?;

        Ok(Session {
            id: row.get::<String>(0)?,
            started_at: parse_datetime(&row.get::<String>(1)?)?,
            ended_at: parse_optional_datetime(get_opt_string(&row, 2)?)?,
            status: parse_enum(&row.get::<String>(3)?)?,
            summary: get_opt_string(&row, 4)?,
        })
    }

    /// List sessions, optionally filtered by status.
    pub async fn list_sessions(
        &self,
        status: Option<SessionStatus>,
        limit: u32,
    ) -> Result<Vec<Session>, DatabaseError> {
        let sql = match status {
            Some(s) => format!(
                "SELECT id, started_at, ended_at, status, summary FROM sessions
                 WHERE status = '{}' ORDER BY started_at DESC LIMIT {}",
                s.as_str(), limit
            ),
            None => format!(
                "SELECT id, started_at, ended_at, status, summary FROM sessions
                 ORDER BY started_at DESC LIMIT {}", limit
            ),
        };
        // Execute and map rows...
    }

    /// Create a session snapshot (called during wrap-up).
    ///
    /// Aggregates counts from tasks, hypotheses, findings, research_items.
    ///
    /// Source: 01-turso-data-model.md §2 — session_snapshots table.
    pub async fn create_snapshot(
        &self,
        session_id: &str,
        summary: &str,
    ) -> Result<SessionSnapshot, DatabaseError> {
        // Count aggregates
        let open_tasks = self.count_by_status(EntityType::Task, "open").await?;
        let in_progress_tasks = self.count_by_status(EntityType::Task, "in_progress").await?;
        let pending_hyps = self.count_by_status(EntityType::Hypothesis, "unverified").await?;
        let unverified_hyps = self.count_by_status(EntityType::Hypothesis, "analyzing").await?;
        let recent_findings = self.count_recent("findings", 24).await?; // last 24h
        let open_research = self.count_by_status(EntityType::Research, "open").await?;

        self.db.conn().execute(
            "INSERT INTO session_snapshots
             (session_id, open_tasks, in_progress_tasks, pending_hypotheses,
              unverified_hypotheses, recent_findings, open_research, summary)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            libsql::params![
                session_id, open_tasks, in_progress_tasks, pending_hyps,
                unverified_hyps, recent_findings, open_research, summary
            ],
        ).await?;

        // Return SessionSnapshot...
    }

    /// Detect sessions in 'active' status (orphans from crashed sessions).
    async fn detect_orphan_sessions(&self) -> Result<Vec<Session>, DatabaseError> {
        self.list_sessions(Some(SessionStatus::Active), 10).await
    }

    /// Mark a session as abandoned.
    async fn abandon_session(&self, session_id: &str) -> Result<(), DatabaseError> {
        self.db.conn().execute(
            "UPDATE sessions SET status = 'abandoned', ended_at = datetime('now') WHERE id = ?1",
            [session_id],
        ).await?;
        // Audit + trail...
        Ok(())
    }

    /// Helper: count rows matching a status in a table.
    async fn count_by_status(&self, entity: EntityType, status: &str) -> Result<i64, DatabaseError> {
        let table = entity_type_to_table(&entity);
        let sql = format!("SELECT COUNT(*) FROM {table} WHERE status = ?1");
        let mut rows = self.db.conn().query(&sql, [status]).await?;
        let row = rows.next().await?.ok_or(DatabaseError::NoResult)?;
        Ok(row.get::<i64>(0)?)
    }

    /// Helper: count rows created in the last N hours.
    async fn count_recent(&self, table: &str, hours: u32) -> Result<i64, DatabaseError> {
        let sql = format!(
            "SELECT COUNT(*) FROM {table} WHERE created_at >= datetime('now', '-{hours} hours')"
        );
        let mut rows = self.db.conn().query(&sql, ()).await?;
        let row = rows.next().await?.ok_or(DatabaseError::NoResult)?;
        Ok(row.get::<i64>(0)?)
    }
}
```

### A — Tests (PR 1)

| Test | What it validates |
|------|-------------------|
| `audit_append_and_query` | Append 3 audit entries, query back with no filter, verify all 3 returned |
| `audit_filter_by_entity` | Filter by `entity_type = Finding`, verify only finding audits returned |
| `audit_filter_by_action` | Filter by `action = Created`, verify only creation audits returned |
| `audit_filter_by_session` | Filter by session_id, verify scoping |
| `audit_search_fts` | Insert audit with detail "tokio runtime", FTS search "runtime" returns it |
| `session_start_creates_active` | `start_session()` returns session with status `Active` |
| `session_end_transitions` | Start → end, verify status is `WrappedUp`, `ended_at` is set |
| `session_end_invalid_transition` | End an already-ended session → `InvalidState` error |
| `session_orphan_detection` | Start session A, start session B → A is marked `Abandoned` |
| `session_list_by_status` | List with status filter works |
| `session_snapshot_aggregates` | Create tasks + hypotheses, snapshot captures correct counts |
| `trail_writer_creates_file` | Append an op, verify `{session_id}.jsonl` file exists |
| `trail_writer_appends_valid_json` | Append op, read back with `serde_jsonlines`, verify roundtrip |
| `trail_writer_per_session_files` | Two sessions write to separate files |
| `trail_writer_disabled_noop` | Disabled writer doesn't create files |
| `trail_validation_warns_on_invalid` | Invalid data logs warning but still writes (permissive) |
| `service_new_local` | `ZenService::new_local(":memory:", None)` succeeds |

---

## 6. PR 2 — Stream B: Entity Repos

10 entity repos + 8 update builders. Each repo follows the same template. I'll specify the full pattern for `FindingRepo` (the most complete example), then document the entity-specific variations.

Validation status:
- **Validated components**: CRUD/FTS SQL patterns from `spike_libsql.rs`, study lifecycle/query patterns from `spike_studies.rs`
- **Design-only components**: repo-by-repo typed method surface and builder wiring in production modules (validated by Phase 2 test matrix)

### B — Template: The FindingRepo Pattern

**Implements task 2.4**: CRUD + tag/untag + FTS search.

**Source**: `05-crate-designs.md` §5 lines 667–715 (code sample for `create_finding`, `tag_finding`, `search_findings`).

#### Create

```rust
impl ZenService {
    pub async fn create_finding(
        &self,
        session_id: &str,
        content: &str,
        source: Option<&str>,
        confidence: Confidence,
        research_id: Option<&str>,
    ) -> Result<Finding, DatabaseError> {
        let id = self.db.generate_id(PREFIX_FINDING).await?;
        let now = Utc::now();

        self.db.conn().execute(
            "INSERT INTO findings (id, research_id, session_id, content, source, confidence, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            libsql::params![
                id.as_str(),
                research_id.unwrap_or(""),
                session_id,
                content,
                source.unwrap_or(""),
                confidence.as_str(),
                now.to_rfc3339().as_str(),
                now.to_rfc3339().as_str(),
            ],
        ).await?;

        let finding = Finding {
            id: id.clone(),
            research_id: research_id.map(String::from),
            session_id: Some(session_id.to_string()),
            content: content.to_string(),
            source: source.map(String::from),
            confidence,
            created_at: now,
            updated_at: now,
        };

        // Audit
        self.append_audit(&AuditEntry {
            id: self.db.generate_id(PREFIX_AUDIT).await?,
            session_id: Some(session_id.to_string()),
            entity_type: EntityType::Finding,
            entity_id: id.clone(),
            action: AuditAction::Created,
            detail: None,
            created_at: now,
        }).await?;

        // Trail
        self.trail.append(&TrailOperation {
            v: 1,
            ts: now.to_rfc3339(),
            ses: session_id.to_string(),
            op: TrailOp::Create,
            entity: EntityType::Finding,
            id,
            data: serde_json::to_value(&finding)
                .map_err(|e| DatabaseError::Other(e.into()))?,
        })?;

        Ok(finding)
    }
}
```

#### Get

```rust
    pub async fn get_finding(&self, id: &str) -> Result<Finding, DatabaseError> {
        let mut rows = self.db.conn().query(
            "SELECT id, research_id, session_id, content, source, confidence, created_at, updated_at
             FROM findings WHERE id = ?1",
            [id],
        ).await?;
        let row = rows.next().await?.ok_or(DatabaseError::NoResult)?;

        Ok(Finding {
            id: row.get(0)?,
            research_id: get_opt_string(&row, 1)?,
            session_id: get_opt_string(&row, 2)?,
            content: row.get(3)?,
            source: get_opt_string(&row, 4)?,
            confidence: parse_enum(&row.get::<String>(5)?)?,
            created_at: parse_datetime(&row.get::<String>(6)?)?,
            updated_at: parse_datetime(&row.get::<String>(7)?)?,
        })
    }
```

#### Update (with builder)

```rust
    pub async fn update_finding(
        &self,
        session_id: &str,
        finding_id: &str,
        update: FindingUpdate,
    ) -> Result<Finding, DatabaseError> {
        // Build SET clause dynamically from builder output
        let mut sets = Vec::new();
        let mut params: Vec<Box<dyn libsql::params::IntoValue>> = Vec::new();

        if let Some(ref content) = update.content {
            sets.push(format!("content = ?{}", params.len() + 1));
            params.push(Box::new(content.clone()));
        }
        if let Some(ref source) = update.source {
            sets.push(format!("source = ?{}", params.len() + 1));
            params.push(Box::new(source.clone().unwrap_or_default()));
        }
        if let Some(ref confidence) = update.confidence {
            sets.push(format!("confidence = ?{}", params.len() + 1));
            params.push(Box::new(confidence.as_str().to_string()));
        }

        if sets.is_empty() {
            return self.get_finding(finding_id).await;
        }

        // Always update updated_at
        sets.push(format!("updated_at = ?{}", params.len() + 1));
        params.push(Box::new(Utc::now().to_rfc3339()));

        // WHERE id = ?N
        let id_param = params.len() + 1;
        let sql = format!(
            "UPDATE findings SET {} WHERE id = ?{id_param}",
            sets.join(", ")
        );
        // Execute with params...

        // Audit (Updated) + trail (Update with builder data)
        self.trail.append(&TrailOperation {
            v: 1,
            ts: Utc::now().to_rfc3339(),
            ses: session_id.to_string(),
            op: TrailOp::Update,
            entity: EntityType::Finding,
            id: finding_id.to_string(),
            data: serde_json::to_value(&update)
                .map_err(|e| DatabaseError::Other(e.into()))?,
        })?;

        self.get_finding(finding_id).await
    }
```

#### Delete

```rust
    pub async fn delete_finding(
        &self,
        session_id: &str,
        finding_id: &str,
    ) -> Result<(), DatabaseError> {
        // Delete tags first (FK constraint)
        self.db.conn().execute(
            "DELETE FROM finding_tags WHERE finding_id = ?1",
            [finding_id],
        ).await?;

        self.db.conn().execute(
            "DELETE FROM findings WHERE id = ?1",
            [finding_id],
        ).await?;

        // Audit + trail (Delete)
        // ...

        Ok(())
    }
```

#### Tag / Untag

```rust
    /// Tag a finding. Uses INSERT OR IGNORE for idempotent tagging.
    ///
    /// Source: 05-crate-designs.md §5 line 689 — `tag_finding` code sample.
    pub async fn tag_finding(
        &self,
        session_id: &str,
        finding_id: &str,
        tag: &str,
    ) -> Result<(), DatabaseError> {
        self.db.conn().execute(
            "INSERT OR IGNORE INTO finding_tags (finding_id, tag) VALUES (?1, ?2)",
            libsql::params![finding_id, tag],
        ).await?;

        // Audit with TaggedDetail
        self.append_audit(&AuditEntry {
            id: self.db.generate_id(PREFIX_AUDIT).await?,
            session_id: Some(session_id.to_string()),
            entity_type: EntityType::Finding,
            entity_id: finding_id.to_string(),
            action: AuditAction::Tagged,
            detail: Some(serde_json::to_value(TaggedDetail { tag: tag.to_string() }).unwrap()),
            created_at: Utc::now(),
        }).await?;

        // Trail (Tag)
        self.trail.append(&TrailOperation {
            v: 1,
            ts: Utc::now().to_rfc3339(),
            ses: session_id.to_string(),
            op: TrailOp::Tag,
            entity: EntityType::Finding,
            id: finding_id.to_string(),
            data: serde_json::json!({"tag": tag}),
        })?;

        Ok(())
    }

    pub async fn untag_finding(
        &self,
        session_id: &str,
        finding_id: &str,
        tag: &str,
    ) -> Result<(), DatabaseError> {
        self.db.conn().execute(
            "DELETE FROM finding_tags WHERE finding_id = ?1 AND tag = ?2",
            libsql::params![finding_id, tag],
        ).await?;

        // Audit (Untagged) + trail (Untag)
        // ...

        Ok(())
    }

    /// Get all tags for a finding.
    pub async fn get_finding_tags(&self, finding_id: &str) -> Result<Vec<String>, DatabaseError> {
        let mut rows = self.db.conn().query(
            "SELECT tag FROM finding_tags WHERE finding_id = ?1 ORDER BY tag",
            [finding_id],
        ).await?;
        let mut tags = Vec::new();
        while let Some(row) = rows.next().await? {
            tags.push(row.get::<String>(0)?);
        }
        Ok(tags)
    }
```

#### FTS Search

```rust
    /// Full-text search across findings.
    ///
    /// Uses porter stemming: "spawning" matches "spawn".
    /// Source: 01-turso-data-model.md §9 — FTS5 query pattern.
    /// Validated in: Phase 1 test `fts5_search_works`.
    pub async fn search_findings(
        &self,
        query: &str,
        limit: u32,
    ) -> Result<Vec<Finding>, DatabaseError> {
        let mut rows = self.db.conn().query(
            "SELECT f.id, f.research_id, f.session_id, f.content, f.source,
                    f.confidence, f.created_at, f.updated_at
             FROM findings_fts
             JOIN findings f ON f.rowid = findings_fts.rowid
             WHERE findings_fts MATCH ?1
             ORDER BY rank LIMIT ?2",
            libsql::params![query, limit.to_string().as_str()],
        ).await?;
        // Map rows to Finding structs using helpers...
    }
```

### B — Entity-Specific Variations

Each entity repo follows the same Create/Get/Update/Delete/List + FTS pattern. Here are the variations per entity:

#### ResearchRepo (task 2.3)

- **Table**: `research_items`
- **FTS**: `research_fts` (indexed: `title, description`)
- **Status transitions**: `ResearchStatus` — `open → in_progress → resolved|abandoned`
- **Update builder**: `ResearchUpdateBuilder` — fields: `title`, `description`, `status`

#### HypothesisRepo (task 2.5)

- **Table**: `hypotheses`
- **FTS**: `hypotheses_fts` (indexed: `content, reason`)
- **Status transitions**: `HypothesisStatus` — validated via `can_transition_to()`
- **Special**: Status change writes `StatusChangedDetail` to audit detail
- **Update builder**: `HypothesisUpdateBuilder` — fields: `content`, `status`, `reason`

Status transition method:

```rust
    /// Transition hypothesis status with validation.
    ///
    /// Source: enums.rs — HypothesisStatus state machine.
    /// Uses Transition trail op (not Update) for status changes.
    pub async fn transition_hypothesis(
        &self,
        session_id: &str,
        hyp_id: &str,
        new_status: HypothesisStatus,
        reason: Option<&str>,
    ) -> Result<Hypothesis, DatabaseError> {
        let current = self.get_hypothesis(hyp_id).await?;

        if !current.status.can_transition_to(new_status) {
            return Err(DatabaseError::InvalidState(format!(
                "Cannot transition hypothesis {} from {} to {}",
                hyp_id, current.status, new_status
            )));
        }

        self.db.conn().execute(
            "UPDATE hypotheses SET status = ?1, reason = ?2, updated_at = datetime('now') WHERE id = ?3",
            libsql::params![new_status.as_str(), reason.unwrap_or(""), hyp_id],
        ).await?;

        // Audit with StatusChangedDetail
        let detail = StatusChangedDetail {
            from: current.status.as_str().to_string(),
            to: new_status.as_str().to_string(),
            reason: reason.map(String::from),
        };
        self.append_audit(&AuditEntry {
            // ...
            action: AuditAction::StatusChanged,
            detail: Some(serde_json::to_value(&detail).unwrap()),
            // ...
        }).await?;

        // Trail (Transition op)
        self.trail.append(&TrailOperation {
            op: TrailOp::Transition,
            data: serde_json::to_value(&detail).unwrap(),
            // ...
        })?;

        self.get_hypothesis(hyp_id).await
    }
```

#### InsightRepo (task 2.6)

- **Table**: `insights`
- **FTS**: `insights_fts` (indexed: `content`)
- **No status transitions** (no status column)
- **Update builder**: `InsightUpdateBuilder` — fields: `content`, `confidence`

#### IssueRepo (task 2.7)

- **Table**: `issues`
- **FTS**: `issues_fts` (indexed: `title, description`)
- **Status transitions**: `IssueStatus` — `open → in_progress → done|blocked|abandoned`
- **Special: parent-child queries**
- **Column mapping note**: SQL `type` → Rust `issue_type` (keyword collision)
- **Update builder**: `IssueUpdateBuilder` — fields: `title`, `description`, `status`, `priority`, `parent_id`, `issue_type`

Parent-child queries:

```rust
    /// Get child issues for a parent.
    pub async fn get_child_issues(&self, parent_id: &str) -> Result<Vec<Issue>, DatabaseError> {
        // SELECT * FROM issues WHERE parent_id = ?1 ORDER BY priority, created_at
    }

    /// Get the parent issue.
    pub async fn get_parent_issue(&self, issue_id: &str) -> Result<Option<Issue>, DatabaseError> {
        let issue = self.get_issue(issue_id).await?;
        match issue.parent_id {
            Some(ref pid) => Ok(Some(self.get_issue(pid).await?)),
            None => Ok(None),
        }
    }
```

#### TaskRepo (task 2.8)

- **Table**: `tasks`
- **FTS**: `tasks_fts` (indexed: `title, description`)
- **Status transitions**: `TaskStatus` — `open → in_progress → done|blocked`
- **Special**: issue linkage queries
- **Update builder**: `TaskUpdateBuilder` — fields: `title`, `description`, `status`, `issue_id`, `research_id`

```rust
    /// Get tasks for an issue.
    pub async fn get_tasks_for_issue(&self, issue_id: &str) -> Result<Vec<Task>, DatabaseError> {
        // SELECT * FROM tasks WHERE issue_id = ?1 ORDER BY status, created_at
    }
```

#### ImplLogRepo (task 2.9)

- **Table**: `implementation_log`
- **No FTS** (no FTS virtual table for impl_log)
- **No status transitions**
- **Append-only**: No update builder (impl logs record facts, they're not edited)
- **Special**: file path queries

```rust
    /// Get impl logs for a file path (prefix match).
    pub async fn get_logs_by_file(
        &self,
        file_path: &str,
    ) -> Result<Vec<ImplLog>, DatabaseError> {
        // SELECT * FROM implementation_log WHERE file_path LIKE ?1 || '%'
    }

    /// Get impl logs for a task.
    pub async fn get_logs_for_task(&self, task_id: &str) -> Result<Vec<ImplLog>, DatabaseError> {
        // SELECT * FROM implementation_log WHERE task_id = ?1 ORDER BY created_at
    }
```

#### CompatRepo (task 2.10)

- **Table**: `compatibility_checks`
- **No FTS** (no FTS virtual table for compat)
- **No status transitions** (CompatStatus is not a state machine)
- **Special**: package pair queries
- **Update builder**: `CompatUpdateBuilder` — fields: `status`, `conditions`, `finding_id`

```rust
    /// Query compatibility between two packages (order-independent).
    pub async fn get_compat(
        &self,
        package_a: &str,
        package_b: &str,
    ) -> Result<Option<CompatCheck>, DatabaseError> {
        // Check both orderings: (a,b) and (b,a)
        // SELECT * FROM compatibility_checks
        //  WHERE (package_a = ?1 AND package_b = ?2)
        //     OR (package_a = ?2 AND package_b = ?1)
    }
```

#### ProjectRepo (task 2.2)

- **Table**: `project_meta` (KV pairs) + `project_dependencies`
- **No FTS, no status transitions**
- **No update builder** (meta is upsert, deps are upsert)

```rust
    /// Upsert a project metadata key-value pair.
    pub async fn set_meta(&self, key: &str, value: &str) -> Result<(), DatabaseError> {
        self.db.conn().execute(
            "INSERT INTO project_meta (key, value, updated_at)
             VALUES (?1, ?2, datetime('now'))
             ON CONFLICT(key) DO UPDATE SET value = ?2, updated_at = datetime('now')",
            libsql::params![key, value],
        ).await?;
        Ok(())
    }

    /// Upsert a project dependency.
    pub async fn upsert_dependency(&self, dep: &ProjectDependency) -> Result<(), DatabaseError> {
        self.db.conn().execute(
            "INSERT INTO project_dependencies (ecosystem, name, version, source, indexed, indexed_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT(ecosystem, name) DO UPDATE SET
                version = ?3, source = ?4, indexed = ?5, indexed_at = ?6",
            libsql::params![
                dep.ecosystem.as_str(), dep.name.as_str(),
                dep.version.as_deref(),
                dep.source.as_str(),
                dep.indexed,
                dep.indexed_at.map(|dt| dt.to_rfc3339()).as_deref()
            ],
        ).await?;
        Ok(())
    }
```

#### StudyRepo (task 2.14)

The most complex repo. Uses `entity_links` extensively.

**Validated in**: Spike 0.11 (15/15 tests) — Approach B (hybrid), full state query, progress tracking.

**Source**: `08-studies-spike-plan.md` (entity_links join pattern, progress aggregate).

- **Table**: `studies`
- **FTS**: `studies_fts` (indexed: `topic, summary`)
- **Status transitions**: `StudyStatus` — `active → concluding → completed|abandoned`
- **Update builder**: `StudyUpdateBuilder` — fields: `topic`, `library`, `methodology`, `status`, `summary`

```rust
    /// Add a hypothesis as a study assumption.
    ///
    /// Creates the hypothesis AND links it to the study via entity_links.
    /// Source: spike 0.11 — hypotheses linked to studies via entity_links, not FK.
    pub async fn add_assumption(
        &self,
        session_id: &str,
        study_id: &str,
        content: &str,
    ) -> Result<Hypothesis, DatabaseError> {
        // 1. Get the study (to find its research_id)
        let study = self.get_study(study_id).await?;

        // 2. Create hypothesis (uses research_id FK for direct queries)
        let hyp = self.create_hypothesis(
            session_id, content,
            study.research_id.as_deref(), None,
        ).await?;

        // 3. Link hypothesis to study via entity_links
        self.create_link(
            session_id,
            EntityType::Study, study_id,
            EntityType::Hypothesis, &hyp.id,
            Relation::RelatesTo,
        ).await?;

        Ok(hyp)
    }

    /// Record a test result (finding) for a study hypothesis.
    ///
    /// Creates finding + links it to both study and hypothesis.
    pub async fn record_test_result(
        &self,
        session_id: &str,
        study_id: &str,
        hypothesis_id: &str,
        content: &str,
        confidence: Confidence,
    ) -> Result<Finding, DatabaseError> {
        let study = self.get_study(study_id).await?;

        let finding = self.create_finding(
            session_id, content, None, confidence,
            study.research_id.as_deref(),
        ).await?;

        // Link finding → study
        self.create_link(
            session_id,
            EntityType::Study, study_id,
            EntityType::Finding, &finding.id,
            Relation::RelatesTo,
        ).await?;

        // Link finding → hypothesis (validates)
        self.create_link(
            session_id,
            EntityType::Finding, &finding.id,
            EntityType::Hypothesis, hypothesis_id,
            Relation::Validates,
        ).await?;

        Ok(finding)
    }

    /// Conclude a study: set status, summary, and create an insight.
    pub async fn conclude_study(
        &self,
        session_id: &str,
        study_id: &str,
        summary: &str,
    ) -> Result<(Study, Insight), DatabaseError> {
        // 1. Transition through Concluding if currently Active
        let study = self.get_study(study_id).await?;
        if study.status == StudyStatus::Active {
            self.transition_study(session_id, study_id, StudyStatus::Concluding).await?;
        }
        self.transition_study(session_id, study_id, StudyStatus::Completed).await?;

        // 2. Update summary
        let update = StudyUpdateBuilder::new().summary(summary.to_string()).build();
        self.update_study(session_id, study_id, update).await?;

        // 3. Create insight from summary
        let study = self.get_study(study_id).await?;
        let insight = self.create_insight(
            session_id, summary, Confidence::High,
            study.research_id.as_deref(),
        ).await?;

        // 4. Link insight to study
        self.create_link(
            session_id,
            EntityType::Study, study_id,
            EntityType::Insight, &insight.id,
            Relation::DerivedFrom,
        ).await?;

        Ok((self.get_study(study_id).await?, insight))
    }

    /// Get full study state including linked hypotheses, findings, insights.
    ///
    /// Uses the correlated subquery pattern from spike 0.11.
    /// Source: spike_studies.rs — full_state_query test.
    pub async fn get_study_full_state(&self, study_id: &str) -> Result<StudyFullState, DatabaseError> {
        let study = self.get_study(study_id).await?;

        // Explicit per-type methods (no generic FromRow trait needed)
        let assumptions = self.get_linked_hypotheses(
            EntityType::Study, study_id,
        ).await?;

        let findings = self.get_linked_findings(
            EntityType::Study, study_id,
        ).await?;

        let conclusions = self.get_linked_insights(
            EntityType::Study, study_id,
        ).await?;

        Ok(StudyFullState { study, assumptions, findings, conclusions })
    }

    /// Progress tracking: count hypotheses by status.
    ///
    /// SQL from spike 0.11:
    /// ```sql
    /// SELECT COUNT(*) as total,
    ///     SUM(CASE WHEN h.status = 'confirmed' THEN 1 ELSE 0 END) as confirmed,
    ///     SUM(CASE WHEN h.status = 'debunked' THEN 1 ELSE 0 END) as debunked,
    ///     SUM(CASE WHEN h.status = 'unverified' THEN 1 ELSE 0 END) as untested
    /// FROM entity_links el
    /// JOIN hypotheses h ON h.id = el.target_id
    /// WHERE el.source_type = 'study' AND el.source_id = ?
    ///   AND el.target_type = 'hypothesis'
    /// ```
    pub async fn study_progress(&self, study_id: &str) -> Result<StudyProgress, DatabaseError> {
        let mut rows = self.db.conn().query(
            "SELECT
                COUNT(*) as total,
                SUM(CASE WHEN h.status = 'confirmed' THEN 1 ELSE 0 END),
                SUM(CASE WHEN h.status = 'debunked' THEN 1 ELSE 0 END),
                SUM(CASE WHEN h.status = 'unverified' THEN 1 ELSE 0 END)
             FROM entity_links el
             JOIN hypotheses h ON h.id = el.target_id
             WHERE el.source_type = 'study' AND el.source_id = ?1
               AND el.target_type = 'hypothesis'",
            [study_id],
        ).await?;
        let row = rows.next().await?.ok_or(DatabaseError::NoResult)?;
        Ok(StudyProgress {
            total: row.get(0)?,
            confirmed: row.get(1)?,
            debunked: row.get(2)?,
            untested: row.get(3)?,
        })
    }
}

/// Full study state including linked entities.
pub struct StudyFullState {
    pub study: Study,
    pub assumptions: Vec<Hypothesis>,
    pub findings: Vec<Finding>,
    pub conclusions: Vec<Insight>,
}

/// Hypothesis progress counts for a study.
pub struct StudyProgress {
    pub total: i64,
    pub confirmed: i64,
    pub debunked: i64,
    pub untested: i64,
}
```

### B — Update Builder Specifications

| Entity | Builder Fields | `Option<Option<T>>` fields (nullable) |
|--------|---------------|---------------------------------------|
| Finding | content, source, confidence | source |
| Hypothesis | content, status, reason | reason |
| Research | title, description, status | description |
| Insight | content, confidence | — |
| Issue | title, description, status, priority, issue_type, parent_id | description, parent_id |
| Task | title, description, status, issue_id, research_id | description, issue_id, research_id |
| Compat | status, conditions, finding_id | conditions, finding_id |
| Study | topic, library, methodology, status, summary | library, summary |

All builders derive `Default` and implement `Serialize` with `#[serde(skip_serializing_if = "Option::is_none")]` on every field.

### B — Tests (PR 2)

For each of the 10 entity repos:

| Test Pattern | Count | Total |
|-------------|-------|-------|
| `create_{entity}_roundtrip` — create, get back, verify fields | 10 | 10 |
| `update_{entity}_partial` — update one field, verify only that field changed | 8 | 8 |
| `delete_{entity}` — create, delete, get returns NoResult | 10 | 10 |
| `list_{entities}` — create 3, list, verify count | 10 | 10 |
| `search_{entity}_fts` — create with content, FTS search returns it | 7 | 7 |
| `search_{entity}_porter_stemming` — "spawning" matches "spawn" | 7 | 7 |
| `{entity}_audit_on_create` — create entity, verify audit entry exists | 10 | 10 |
| `{entity}_audit_on_update` — update entity, verify audit with Updated action | 8 | 8 |
| `{entity}_trail_on_create` — create entity, read JSONL, verify trail op | 10 | 10 |
| `{entity}_trail_on_update` — update, verify trail has only changed fields | 8 | 8 |

Entity-specific tests:

| Test | Entity |
|------|--------|
| `hypothesis_valid_transition` — unverified → analyzing succeeds | Hypothesis |
| `hypothesis_invalid_transition` — unverified → confirmed fails | Hypothesis |
| `hypothesis_status_change_audit_detail` — verify StatusChangedDetail in audit | Hypothesis |
| `issue_parent_child` — create parent + child, query children | Issue |
| `issue_type_column_mapping` — SQL `type` maps to Rust `issue_type` | Issue |
| `task_issue_linkage` — create task with issue_id, query tasks by issue | Task |
| `finding_tag_untag` — tag, verify, untag, verify removed | Finding |
| `finding_tag_idempotent` — tag same tag twice, no error | Finding |
| `finding_tag_audit` — tag produces TaggedDetail audit | Finding |
| `impl_log_file_query` — query by file path prefix | ImplLog |
| `compat_package_pair_query` — query by (a,b) and (b,a) both work | Compat |
| `study_full_lifecycle` — create → add_assumption → record_test → conclude | Study |
| `study_progress_tracking` — verify confirmed/debunked/untested counts | Study |
| `study_full_state_query` — verify linked hypotheses/findings/insights | Study |
| `study_fts_search` — search by topic | Study |
| `project_meta_upsert` — set then update, verify latest value | Project |
| `project_dep_upsert` — insert then update version | Project |

**Estimated**: ~100 tests.

---

## 7. PR 3 — Stream C: Cross-Cutting + Replayer

Validation status:
- **Validated components**: replay ordering and rebuild parity (`spike_jsonl_replay_rebuild`), rebuild + FTS behavior (`spike_jsonl_rebuild_fts`), envelope version dispatch (`spike_replay_dispatch_routes_by_version`)
- **Design-only components**: complete `(op, entity)` replay coverage for all production entities in one module

### C1. `src/repos/link.rs` — LinkRepo

**Implements task 2.11**: Create, delete, query by source, query by target.

**Source**: `01-turso-data-model.md` §7 — entity_links table, UNIQUE constraint.

```rust
impl ZenService {
    /// Create an entity link.
    ///
    /// Returns error if the exact link already exists (UNIQUE constraint).
    pub async fn create_link(
        &self,
        session_id: &str,
        source_type: EntityType,
        source_id: &str,
        target_type: EntityType,
        target_id: &str,
        relation: Relation,
    ) -> Result<EntityLink, DatabaseError> {
        let id = self.db.generate_id(PREFIX_LINK).await?;
        let now = Utc::now();

        self.db.conn().execute(
            "INSERT INTO entity_links (id, source_type, source_id, target_type, target_id, relation, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            libsql::params![
                id.as_str(),
                source_type.as_str(), source_id,
                target_type.as_str(), target_id,
                relation.as_str(),
                now.to_rfc3339().as_str(),
            ],
        ).await?;

        let link = EntityLink {
            id: id.clone(), source_type, source_id: source_id.to_string(),
            target_type, target_id: target_id.to_string(),
            relation, created_at: now,
        };

        // Audit (Linked) with LinkedDetail
        let detail = LinkedDetail {
            source_type: source_type.as_str().to_string(),
            source_id: source_id.to_string(),
            target_type: target_type.as_str().to_string(),
            target_id: target_id.to_string(),
            relation: relation.as_str().to_string(),
        };
        // ... append_audit + trail (Link op)

        Ok(link)
    }

    /// Delete a link by ID.
    pub async fn delete_link(
        &self,
        session_id: &str,
        link_id: &str,
    ) -> Result<(), DatabaseError> {
        // Get the link first (for audit detail)
        let link = self.get_link(link_id).await?;

        self.db.conn().execute(
            "DELETE FROM entity_links WHERE id = ?1",
            [link_id],
        ).await?;

        // Audit (Unlinked) + trail (Unlink)
        Ok(())
    }

    /// Query all links FROM an entity.
    pub async fn get_links_from(
        &self,
        source_type: EntityType,
        source_id: &str,
    ) -> Result<Vec<EntityLink>, DatabaseError> {
        // SELECT * FROM entity_links WHERE source_type = ?1 AND source_id = ?2
    }

    /// Query all links TO an entity.
    pub async fn get_links_to(
        &self,
        target_type: EntityType,
        target_id: &str,
    ) -> Result<Vec<EntityLink>, DatabaseError> {
        // SELECT * FROM entity_links WHERE target_type = ?1 AND target_id = ?2
    }
}
```

### C2. `src/repos/whats_next.rs` — Aggregate Query

**Implements task 2.13**: Aggregate open tasks, pending hypotheses, recent audit.

**Source**: `01-turso-data-model.md` §13, `responses.rs` — `WhatsNextResponse`.

```rust
impl ZenService {
    /// Get the current project state for `znt whats-next`.
    ///
    /// Returns:
    /// - Last session (if any)
    /// - Open/in-progress tasks
    /// - Pending (unverified/analyzing) hypotheses
    /// - Recent audit entries (last 20)
    pub async fn whats_next(&self) -> Result<WhatsNextResponse, DatabaseError> {
        // Last session
        let sessions = self.list_sessions(None, 1).await?;
        let last_session = sessions.into_iter().next();

        // Open tasks (open + in_progress)
        let mut task_rows = self.db.conn().query(
            "SELECT id, research_id, issue_id, session_id, title, description, status, created_at, updated_at
             FROM tasks WHERE status IN ('open', 'in_progress')
             ORDER BY status, created_at",
            (),
        ).await?;
        let open_tasks = /* map rows to Vec<Task> */;

        // Pending hypotheses (unverified + analyzing)
        let mut hyp_rows = self.db.conn().query(
            "SELECT id, research_id, finding_id, session_id, content, status, reason, created_at, updated_at
             FROM hypotheses WHERE status IN ('unverified', 'analyzing')
             ORDER BY created_at DESC",
            (),
        ).await?;
        let pending_hypotheses = /* map rows to Vec<Hypothesis> */;

        // Recent audit (last 20)
        let recent_audit = self.query_audit(&AuditFilter {
            limit: Some(20),
            ..Default::default()
        }).await?;

        Ok(WhatsNextResponse {
            last_session,
            open_tasks,
            pending_hypotheses,
            recent_audit,
        })
    }
}
```

### C3. `src/trail/replayer.rs` — Trail Replayer

**Implements tasks 2.16 and 2.17**: Read all trail files, replay operations to rebuild DB, version dispatch.

**Validated in**: Spike 0.12 (15/15) — replay logic ~60 LOC, timestamp-sorted merge, FTS5 auto-rebuild. Spike 0.16 (10/10) — version dispatch with `match op.v`.

**Source**: `10-git-jsonl-strategy.md` (rebuild process), `14-trail-versioning-spike-plan.md` (Approach D).

**Proof snippet (`spike_replay_dispatch_routes_by_version`)**:

```rust
match op.v {
    1 => Ok(migrate_or_passthrough(op)),
    2 => Ok(op.data.clone()),
    v => Err(format!("Unsupported trail version: {}", v)),
}
```

```rust
use std::path::Path;
use zen_core::trail::TrailOperation;
use zen_core::enums::{TrailOp, EntityType};
use zen_core::responses::RebuildResponse;
use zen_schema::SchemaRegistry;

pub struct TrailReplayer;

impl TrailReplayer {
    /// Rebuild the database from JSONL trail files.
    ///
    /// Process:
    /// 1. Glob `.zenith/trail/*.jsonl`
    /// 2. Read all files via `serde_jsonlines::json_lines()`
    /// 3. Sort all operations by `ts` (timestamp-ordered merge)
    /// 4. For each op: dispatch by `(op.op, op.entity)` and execute SQL
    ///
    /// FTS5 triggers fire automatically on INSERT during replay —
    /// no manual FTS rebuild needed.
    ///
    /// Validated in spike 0.12: FTS5 survival after rebuild confirmed.
    pub async fn rebuild(
        service: &mut ZenService,
        trail_dir: &Path,
        strict: bool,
    ) -> Result<RebuildResponse, DatabaseError> {
        let start = std::time::Instant::now();

        // Disable trail writer to avoid re-writing replayed ops
        service.trail_mut().set_enabled(false);

        // 1. Glob all JSONL files
        let mut trail_files = Vec::new();
        for entry in std::fs::read_dir(trail_dir)
            .map_err(|e| DatabaseError::Other(e.into()))?
        {
            let entry = entry.map_err(|e| DatabaseError::Other(e.into()))?;
            let path = entry.path();
            if path.extension().is_some_and(|ext| ext == "jsonl") {
                trail_files.push(path);
            }
        }

        // 2. Read all operations
        let mut all_ops: Vec<TrailOperation> = Vec::new();
        for path in &trail_files {
            let ops: Vec<TrailOperation> = serde_jsonlines::json_lines(path)
                .map_err(|e| DatabaseError::Other(e.into()))?
                .collect::<std::io::Result<Vec<_>>>()
                .map_err(|e| DatabaseError::Other(e.into()))?;
            all_ops.extend(ops);
        }

        // 3. Sort by timestamp (stable sort preserves intra-session order)
        all_ops.sort_by(|a, b| a.ts.cmp(&b.ts));

        // 4. Optional strict validation
        let schema = if strict { Some(SchemaRegistry::new()) } else { None };

        // 5. Replay each operation
        let mut entities_created = 0u32;
        for op in &all_ops {
            // Version dispatch (spike 0.16 pattern)
            let op = match op.v {
                1 => op.clone(),
                // Future: 2 => migrate_v1_to_v2(op),
                v => return Err(DatabaseError::InvalidState(
                    format!("Unsupported trail version: {v}")
                )),
            };

            if strict {
                if let Some(ref schema) = schema {
                    if op.op == TrailOp::Create {
                        let name = entity_type_to_schema_name(&op.entity);
                        schema.validate(name, &op.data)
                            .map_err(|e| DatabaseError::InvalidState(
                                format!("Schema validation failed: {e:?}")
                            ))?;
                    }
                }
            }

            replay_operation(service.db(), &op).await?;
            if op.op == TrailOp::Create { entities_created += 1; }
        }

        // Re-enable trail writer
        service.trail_mut().set_enabled(true);

        Ok(RebuildResponse {
            rebuilt: true,
            trail_files: trail_files.len() as u32,
            operations_replayed: all_ops.len() as u32,
            entities_created,
            duration_ms: start.elapsed().as_millis() as u64,
        })
    }
}

/// Convert a JSON field to libsql::Value for replay.
/// None = key absent (skip in updates). Some(Null) = explicit null. Some(Text) = value.
fn json_to_value(data: &serde_json::Value, field: &str) -> libsql::Value {
    match data.get(field) {
        None | Some(serde_json::Value::Null) => libsql::Value::Null,
        Some(serde_json::Value::String(s)) => libsql::Value::Text(s.clone()),
        Some(v) => libsql::Value::Text(v.to_string()),
    }
}

/// Convert a JSON field to Option<libsql::Value> for replay updates.
/// None = key absent (don't change). Some(Null) = set to NULL. Some(Text) = set value.
fn json_to_update_value(data: &serde_json::Value, field: &str) -> Option<libsql::Value> {
    match data.get(field) {
        None => None,
        Some(serde_json::Value::Null) => Some(libsql::Value::Null),
        Some(serde_json::Value::String(s)) => Some(libsql::Value::Text(s.clone())),
        Some(v) => Some(libsql::Value::Text(v.to_string())),
    }
}

/// Replay a single trail operation by executing the corresponding SQL.
///
/// Double-dispatch on (op, entity) as validated in spike 0.12.
/// Data extraction via `op.data["field"].as_str()` with `.get()` for optionals.
async fn replay_operation(db: &ZenDb, op: &TrailOperation) -> Result<(), DatabaseError> {
    match (&op.op, &op.entity) {
        // --- Session ---
        (TrailOp::Create, EntityType::Session) => {
            db.conn().execute(
                "INSERT OR IGNORE INTO sessions (id, started_at, status, summary)
                 VALUES (?1, ?2, ?3, ?4)",
                vec![
                    libsql::Value::Text(op.id.clone()),
                    libsql::Value::Text(op.data["started_at"].as_str().unwrap_or(&op.ts).to_string()),
                    libsql::Value::Text(op.data["status"].as_str().unwrap_or("active").to_string()),
                    json_to_value(&op.data, "summary"),
                ],
            ).await?;
        }

        // --- Research ---
        (TrailOp::Create, EntityType::Research) => {
            db.conn().execute(
                "INSERT OR IGNORE INTO research_items (id, session_id, title, description, status, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                vec![
                    libsql::Value::Text(op.id.clone()),
                    json_to_value(&op.data, "session_id"),
                    json_to_value(&op.data, "title"),
                    json_to_value(&op.data, "description"),
                    libsql::Value::Text(op.data["status"].as_str().unwrap_or("open").to_string()),
                    libsql::Value::Text(op.data.get("created_at").and_then(|v| v.as_str()).unwrap_or(&op.ts).to_string()),
                    libsql::Value::Text(op.data.get("updated_at").and_then(|v| v.as_str()).unwrap_or(&op.ts).to_string()),
                ],
            ).await?;
        }

        // --- Finding ---
        (TrailOp::Create, EntityType::Finding) => {
            db.conn().execute(
                "INSERT OR IGNORE INTO findings (id, research_id, session_id, content, source, confidence, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                vec![
                    libsql::Value::Text(op.id.clone()),
                    json_to_value(&op.data, "research_id"),
                    json_to_value(&op.data, "session_id"),
                    json_to_value(&op.data, "content"),
                    json_to_value(&op.data, "source"),
                    libsql::Value::Text(op.data.get("confidence").and_then(|v| v.as_str()).unwrap_or("medium").to_string()),
                    libsql::Value::Text(op.data.get("created_at").and_then(|v| v.as_str()).unwrap_or(&op.ts).to_string()),
                    libsql::Value::Text(op.data.get("updated_at").and_then(|v| v.as_str()).unwrap_or(&op.ts).to_string()),
                ],
            ).await?;
        }

        // --- Hypothesis ---
        (TrailOp::Create, EntityType::Hypothesis) => { /* similar INSERT */ }

        // --- Update (any entity) ---
        (TrailOp::Update, entity) => {
            // Build dynamic UPDATE from data fields
            // Pattern from spike 0.12: check each field in data, build SET clause
            replay_update(db, entity, &op.id, &op.data).await?;
        }

        // --- Transition (status change) ---
        (TrailOp::Transition, entity) => {
            let new_status = op.data["to"].as_str()
                .ok_or_else(|| DatabaseError::InvalidState("Missing 'to' in transition".into()))?;
            let table = entity_type_to_table(entity);
            let reason = json_to_value(&op.data, "reason");
            db.conn().execute(
                &format!("UPDATE {table} SET status = ?1, updated_at = ?2 WHERE id = ?3"),
                vec![
                    libsql::Value::Text(new_status.to_string()),
                    libsql::Value::Text(op.ts.clone()),
                    libsql::Value::Text(op.id.clone()),
                ],
            ).await?;
            // For hypotheses, also update reason
            if *entity == EntityType::Hypothesis && !matches!(reason, libsql::Value::Null) {
                db.conn().execute(
                    "UPDATE hypotheses SET reason = ?1 WHERE id = ?2",
                    vec![reason, libsql::Value::Text(op.id.clone())],
                ).await?;
            }
        }

        // --- Delete ---
        (TrailOp::Delete, entity) => {
            let table = entity_type_to_table(entity);
            db.conn().execute(
                &format!("DELETE FROM {table} WHERE id = ?1"),
                [op.id.as_str()],
            ).await?;
        }

        // --- Tag ---
        (TrailOp::Tag, EntityType::Finding) => {
            let tag = op.data["tag"].as_str().unwrap_or("");
            db.conn().execute(
                "INSERT OR IGNORE INTO finding_tags (finding_id, tag) VALUES (?1, ?2)",
                libsql::params![op.id.as_str(), tag],
            ).await?;
        }

        // --- Untag ---
        (TrailOp::Untag, EntityType::Finding) => {
            let tag = op.data["tag"].as_str().unwrap_or("");
            db.conn().execute(
                "DELETE FROM finding_tags WHERE finding_id = ?1 AND tag = ?2",
                libsql::params![op.id.as_str(), tag],
            ).await?;
        }

        // --- Link ---
        (TrailOp::Link, EntityType::EntityLink) => {
            db.conn().execute(
                "INSERT OR IGNORE INTO entity_links (id, source_type, source_id, target_type, target_id, relation, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                libsql::params![
                    op.id.as_str(),
                    op.data["source_type"].as_str().unwrap_or(""),
                    op.data["source_id"].as_str().unwrap_or(""),
                    op.data["target_type"].as_str().unwrap_or(""),
                    op.data["target_id"].as_str().unwrap_or(""),
                    op.data["relation"].as_str().unwrap_or(""),
                    op.data.get("created_at").and_then(|v| v.as_str()).unwrap_or(&op.ts),
                ],
            ).await?;
        }

        // --- Unlink ---
        (TrailOp::Unlink, EntityType::EntityLink) => {
            db.conn().execute(
                "DELETE FROM entity_links WHERE id = ?1",
                [op.id.as_str()],
            ).await?;
        }

        // Unhandled
        (op_type, entity) => {
            tracing::warn!("Unhandled replay: {:?} on {:?}", op_type, entity);
        }
    }

    Ok(())
}

/// Map EntityType to SQL table name.
fn entity_type_to_table(entity: &EntityType) -> &'static str {
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
        EntityType::EntityLink => "entity_links",
        EntityType::Audit => "audit_trail",
    }
}
```

**Key design choices in the replayer**:

1. **`INSERT OR IGNORE`**: During rebuild, we don't want duplicate key errors if the same operation appears in multiple contexts. `OR IGNORE` silently skips duplicates.

2. **FTS5 auto-rebuild**: The triggers in `001_initial.sql` fire on every INSERT/UPDATE/DELETE during replay. No manual FTS population needed. Validated in spike 0.12.

3. **Audit entries are NOT replayed**: The audit trail is derived from mutations. During rebuild, we replay the entity mutations only. If we also replayed audit entries, we'd get audit entries about creating audit entries. The audit trail can be reconstructed from the trail operations themselves if needed.

4. **Trail writer disabled**: `service.trail_mut().set_enabled(false)` prevents re-writing JSONL during replay.

### C — Tests (PR 3)

| Test | What it validates |
|------|-------------------|
| `link_create_and_query_from` | Create link, query by source, verify returned |
| `link_create_and_query_to` | Create link, query by target, verify returned |
| `link_delete` | Create link, delete, query returns empty |
| `link_unique_constraint` | Create same link twice → error |
| `link_bidirectional` | Create A→B, query from A returns B, query to B returns A |
| `whats_next_empty` | Fresh DB, whats_next returns empty lists |
| `whats_next_with_data` | Create tasks + hypotheses, verify counts |
| `whats_next_last_session` | Start session, verify it appears in response |
| `rebuild_roundtrip` | Create entities via service → verify JSONL → delete DB → rebuild → verify identical state |
| `rebuild_multi_session` | Two sessions, each writes trail → rebuild merges by timestamp |
| `rebuild_fts_survives` | Create finding "tokio runtime" → rebuild → FTS search "runtime" works |
| `rebuild_strict_rejects_invalid` | Trail with invalid data → strict mode returns error |
| `rebuild_strict_accepts_valid` | Trail with valid data → strict mode succeeds |
| `rebuild_version_dispatch` | v=1 operations replay correctly |
| `rebuild_unsupported_version` | v=99 → error "Unsupported trail version" |
| `rebuild_concurrent_sessions` | 3 session trail files, interleaved timestamps, merged correctly |
| `rebuild_tag_untag_survives` | Tag → untag → rebuild → tags match |
| `rebuild_link_survives` | Create link → rebuild → link exists |
| `rebuild_transition_survives` | Transition hypothesis → rebuild → status updated |

**Estimated**: ~19 tests.

---

## 8. Execution Order

```
Phase 2 Execution:

 PR 1 — Infrastructure
 ─────────────────────
  1. [A1]  Update zen-db Cargo.toml (add zen-schema)
  2. [A2]  Create src/helpers.rs (parse_datetime, parse_enum, get_opt_string, etc.)
  3. [A5]  Create src/service.rs (ZenService struct)
  4. [A3]  Create src/repos/mod.rs + src/repos/audit.rs (AuditRepo)
  5. [A4]  Create src/trail/mod.rs + src/trail/writer.rs (TrailWriter)
  6. [A6]  Create src/repos/session.rs (SessionRepo)
  7.       Update src/lib.rs (wire new modules)
  8.       Write PR 1 tests (~17 tests)
     ─── cargo test -p zen-db passes ───

 PR 2 — Entity Repos (can begin immediately after PR 1)
 ───────────────────
  9.       Create src/updates/mod.rs + 8 update builder files
 10. [B1]  Create src/repos/research.rs
 11. [B2]  Create src/repos/finding.rs (most complete — tag/untag/FTS)
 12. [B3]  Create src/repos/hypothesis.rs (status transitions)
 13. [B4]  Create src/repos/insight.rs
 14. [B5]  Create src/repos/issue.rs (parent-child)
 15. [B6]  Create src/repos/task.rs (issue linkage)
 16. [B7]  Create src/repos/impl_log.rs (file path queries)
 17. [B8]  Create src/repos/compat.rs (package pair queries)
 18. [B9]  Create src/repos/project.rs (meta + deps upsert)
 19. [B10] Create src/repos/study.rs (full lifecycle — most complex)
 20.       Write PR 2 tests (~100 tests)
     ─── cargo test -p zen-db passes ───

 PR 3 — Cross-Cutting + Replayer (can begin after PR 2)
 ──────────────────────────────
 21. [C1]  Create src/repos/link.rs (LinkRepo)
 22. [C2]  Create src/repos/whats_next.rs
 23. [C3]  Create src/trail/replayer.rs (TrailReplayer + version dispatch)
 24.       Write PR 3 tests (~19 tests)
     ─── cargo test -p zen-db passes ───
```

Total estimated: ~136 new tests (17 + 100 + 19).

---

## 9. Gotchas & Warnings

### 9.1 SQLite datetime format vs Rust RFC 3339 — **HIGH**

**Source**: Phase 2 investigation of libsql 0.9.29, spike 0.2

**The problem**: There are two different datetime formats in play, and libsql does NOT do any automatic conversion between them:

| Origin | Format | Example |
|--------|--------|---------|
| SQL `DEFAULT (datetime('now'))` | SQLite format (no T, no TZ) | `2026-02-09 14:30:45` |
| Rust `Utc::now().to_rfc3339()` | RFC 3339 (T separator, TZ offset) | `2026-02-09T14:30:45+00:00` |

libsql's `row.get::<T>(idx)` only supports primitive types (`String`, `i64`, `f64`, `Vec<u8>`, `bool`). There is **no** `FromValue` impl for `chrono::DateTime<Utc>`. The crate has a `chrono` feature, but it's only used internally for sync protocol metadata — not for user-facing row conversions. You **must** extract as `String` and parse manually.

**The two-format problem**: If a row was created using the SQL DEFAULT (e.g., you INSERT without specifying `created_at`), the timestamp is in SQLite format. If it was created by our Rust repo code (e.g., `Utc::now().to_rfc3339()`), it's in RFC 3339 format. During rebuild, JSONL replay re-inserts with the RFC 3339 value from the trail, so rebuilt rows will have RFC 3339 format even if the original used SQL defaults.

**Mitigation**: The `parse_datetime()` helper in `helpers.rs` tries RFC 3339 first, then falls back to SQLite format. Both paths produce `DateTime<Utc>`. This is documented in Section 5, A2.

**Alternative considered but rejected**: Changing all SQL DEFAULTs to `strftime('%Y-%m-%dT%H:%M:%SZ', 'now')` would produce RFC 3339 and eliminate the dual-format issue. However, it would make raw `sqlite3` CLI inspection less readable and diverge from SQLite conventions. Keeping both parsers is safer.

---

### 9.2 Issue `type` column is a SQL/Rust keyword collision — **HIGH**

**Source**: Schema audit (Phase 2 pre-implementation)

**The problem**: The `issues` table has a column named `type` (for issue kind: bug, feature, spike, epic, request). But `type` is a reserved keyword in Rust, so the struct field is `issue_type`:

```rust
// zen-core/src/entities/issue.rs
pub struct Issue {
    pub issue_type: IssueType,  // maps to SQL column `type`
    // ...
}
```

This creates a positional mapping hazard. When writing repo code, you must:

1. **Always use explicit SELECT column lists** — never `SELECT *`. The column order in `SELECT` determines the positional indices for `row.get(idx)`, so the mapping between SQL column `type` and Rust field `issue_type` must be visually obvious:

```rust
// CORRECT — explicit column list, `type` at index 1
let mut rows = self.db.conn().query(
    "SELECT id, type, parent_id, title, description, status, priority, session_id, created_at, updated_at
     FROM issues WHERE id = ?1", [id]
).await?;
let row = rows.next().await?.ok_or(DatabaseError::NoResult)?;
Ok(Issue {
    id: row.get(0)?,
    issue_type: parse_enum(&row.get::<String>(1)?)?,  // `type` column at index 1
    parent_id: get_opt_string(&row, 2)?,
    // ...
})
```

2. **Use `type` in SQL, `issue_type` in Rust** — the SQL INSERT/UPDATE must reference the actual column name:

```rust
// SQL uses `type`, not `issue_type`
self.db.conn().execute(
    "INSERT INTO issues (id, type, parent_id, ...) VALUES (?1, ?2, ?3, ...)",
    libsql::params![id, issue.issue_type.as_str(), ...],
).await?;
```

3. **Document the mapping in a comment** at the top of `repos/issue.rs`:

```rust
//! IssueRepo — CRUD for the `issues` table.
//!
//! NOTE: SQL column `type` maps to Rust field `issue_type` (keyword collision).
//! All SELECT queries list columns explicitly. Index 1 = `type` = `issue_type`.
```

---

### 9.3 `libsql::params!` required for mixed-type parameters — **MEDIUM**

**Source**: Spike 0.2 (`spike_libsql.rs` lines 244-246, 564)

**The problem**: libsql's `execute()` accepts parameters in two forms:

```rust
// Form 1: Homogeneous &str array — only works when ALL params are strings
conn.execute("INSERT INTO t (a, b) VALUES (?, ?)", ["hello", "world"]).await?;

// Form 2: libsql::params! macro — works with mixed types
conn.execute("INSERT INTO t (a, b) VALUES (?, ?)", libsql::params!["hello", 42]).await?;
```

Form 1 will not compile if any parameter is not `&str`. The Zenith schema has `INTEGER` columns (e.g., `issues.priority`, `session_snapshots.open_tasks`, `implementation_log.start_line`) and `BOOLEAN` columns (`project_dependencies.indexed`). Any query touching these columns must use `libsql::params![]`.

**Rule**: Always use `libsql::params![]` in production repo code. Never use `[&str]` arrays. This eliminates an entire class of type errors and makes the code consistent.

**Spike evidence**: `spike_libsql.rs` line 246 demonstrates mixed types:
```rust
libsql::params!["committed", 42]  // String + Integer
```

**Additional subtlety**: When passing `Option<String>` values for nullable columns, you can't directly pass `None` to `libsql::params![]`. Instead, pass empty string `""` and let the SQL layer treat it as empty text, OR use `libsql::Value::Null` explicitly:

```rust
// Option A: Empty string convention (simpler, used in spikes)
libsql::params![research_id.unwrap_or("")]

// Option B: Explicit null (more correct, avoids "" vs NULL confusion)
match research_id {
    Some(id) => libsql::params![id],
    None => libsql::params![libsql::Value::Null],
}
```

We use Option A in Phase 2 (consistent with spike code) but `get_opt_string()` normalizes `""` back to `None` on read. See gotcha #9.4.

---

### 9.4 NULL handling in libsql — reading nullable columns — **MEDIUM**

**Source**: Spike 0.2 (`spike_libsql.rs` lines 535-586, `spike_null_handling` test)

**The problem**: libsql handles NULL in ways that differ from what you might expect from `rusqlite`:

1. **`row.get::<String>(idx)` on a NULL column returns an error**, not an empty string. The spike at line 577 confirms this:

```rust
// This would panic/error — NULL is not a String:
// let val = row.get::<String>(1).unwrap();  // ← ERROR on NULL

// Correct: use get_value() and match
let issue_val = row1.get_value(1).unwrap();
assert!(matches!(issue_val, libsql::Value::Null));
```

2. **`row.get::<Option<String>>(idx)` returns `Ok(None)` for NULL**. This is the idiomatic way to read nullable columns, but we still have the empty-string problem from the write side.

3. **Write side: empty strings stored for "null" params**. When we pass `research_id.unwrap_or("")` via `libsql::params!`, the database stores an empty string `""`, not SQL NULL. So `row.get::<Option<String>>(idx)` returns `Some("")`, not `None`.

**This creates a read-side ambiguity**: An empty string in the database could mean either "explicitly set to empty" or "was supposed to be NULL but we stored '' instead". In Zenith's domain, no entity field is meaningfully an empty string — `""` research_id, `""` source, `""` description are all semantically NULL.

**Mitigation**: The `get_opt_string()` helper normalizes both paths:

```rust
/// Read a nullable TEXT column. Returns None for both SQL NULL and empty string.
///
/// Two cases produce None:
/// 1. Column is SQL NULL → row.get::<Option<String>>() returns Ok(None)
/// 2. Column is "" (empty string stored by unwrap_or("") pattern) → normalized to None
///
/// This is safe because no Zenith entity field is meaningfully "".
pub fn get_opt_string(row: &libsql::Row, idx: i32) -> Result<Option<String>, DatabaseError> {
    match row.get::<Option<String>>(idx)? {
        Some(s) if s.is_empty() => Ok(None),
        other => Ok(other),
    }
}
```

**Alternative considered**: Using `libsql::Value::Null` for actual NULL inserts. This would preserve the NULL/empty distinction in the database. However, the spike code consistently uses `unwrap_or("")`, and the distinction is irrelevant for Zenith's domain. Keeping the simpler pattern is preferred.

---

### 9.5 Trail validation is warn-only — `--strict` rebuild is the enforcement point — **MEDIUM**

**Source**: Spike 0.16 (10/10 tests), `14-trail-versioning-spike-plan.md`

**The problem**: During normal operation (creating findings, updating hypotheses, etc.), the `TrailWriter` validates `Create` operation data against per-entity schemas from `SchemaRegistry`. The question is: what happens when validation fails?

**Decision**: **Warn and continue.** The trail write always succeeds, even if the data doesn't match the current schema. A `tracing::warn!` is emitted.

**Rationale from spike 0.16**: The trail must remain permissive for two reasons:

1. **Forward compatibility**: A newer version of Zenith might add optional fields to entities. If an older version writes trail data without those fields, it should still be valid. The schema generated by `schemars` excludes `Option<T>` and `#[serde(default)]` fields from the `required` array, so this works automatically. But if someone writes data with _extra_ unknown fields (e.g., a plugin adds metadata), strict validation would reject it. Trails deliberately do NOT have `additionalProperties: false` (spike 0.16 §5 — confirmed that omitting `deny_unknown_fields` leaves `additionalProperties` permissive).

2. **Operational safety**: A validation bug should never prevent data from being recorded. Losing trail data is worse than recording technically-invalid data. The trail is the source of truth — if the DB is deleted, only the trail can rebuild it.

**The enforcement point**: `znt rebuild --strict` validates every operation before replaying. This is the right time to enforce schema correctness because:
- You're explicitly asking for strictness
- You can fix the trail data before replaying
- It catches corruption before it enters the fresh database

**What gets validated and when**:

| Operation | Normal write | `--strict` rebuild |
|-----------|-------------|-------------------|
| `Create` (full entity) | Warn if invalid | Error if invalid |
| `Update` (partial fields) | Not validated (partial data fails `required` checks) | Not validated |
| `Transition` (status change) | Not validated (small payload, no entity schema) | Not validated |
| `Tag`/`Untag`/`Link`/`Unlink` | Not validated (small payload) | Not validated |

---

### 9.6 `INSERT OR IGNORE` in replayer prevents duplicate-key errors — **MEDIUM**

**Source**: Spike 0.12 (15/15 tests), `10-git-jsonl-strategy.md`

**The problem**: During `znt rebuild`, the replayer reads all `.zenith/trail/*.jsonl` files, merges them by timestamp, and replays every operation. A `Create` operation translates to an `INSERT` statement. If the same entity ID appears in multiple trail operations (which shouldn't happen normally but can happen in edge cases), a plain `INSERT` would hit a `PRIMARY KEY` constraint violation and abort the rebuild.

**Edge cases where duplicates can appear**:

1. **Manually edited trail files**: A user duplicates a line while resolving a merge conflict in a `.jsonl` file.
2. **Partial write recovery**: If the process crashes mid-write, `serde-jsonlines` may have written a partial JSON line. On next run, the operation is retried, producing a duplicate in the trail.
3. **Git merge of concurrent sessions**: Two branches each have a session trail. After merge, both are in `.zenith/trail/`. If they happened to create an entity with the same ID (extremely unlikely with random IDs, but not impossible), rebuild would hit a conflict.

**Mitigation**: All `Create` replay operations use `INSERT OR IGNORE`:

```sql
-- Plain INSERT would fail on duplicate ID:
INSERT INTO findings (id, ...) VALUES (?1, ...);  -- ERROR if id exists

-- INSERT OR IGNORE silently skips if the ID already exists:
INSERT OR IGNORE INTO findings (id, ...) VALUES (?1, ...);  -- OK, skips duplicate
```

**Trade-off**: `INSERT OR IGNORE` means the first occurrence of an entity wins. If there are two `Create` operations for the same ID with different data, the second is silently dropped. This is acceptable because:
- Duplicate IDs with different data indicate trail corruption
- The first-writer-wins semantic is consistent with Turso's concurrent dedup behavior (spike 0.20, §L1)
- The alternative (failing the entire rebuild on any duplicate) is worse for recovery scenarios

---

### 9.7 Audit entries are NOT replayed during rebuild — **MEDIUM**

**Source**: Design decision

The `audit_trail` table is **derived state**, not source-of-truth state. The source of truth is the JSONL trail files. When the replayer rebuilds the database, it replays entity mutations (Create, Update, Delete, etc.) but does NOT replay audit entries. The reasoning:

1. **Avoid noise**: If we replayed audit entries, every replayed `Create` op would also trigger an `append_audit()` call inside the repo method, creating audit entries about the replay itself. We'd get a mix of original audit records (from the trail) and meta-audit records (audit entries about replaying audit entries).

2. **Trail writer is disabled during rebuild**: The `TrailWriter.enabled = false` flag prevents re-writing JSONL during replay. By the same logic, the audit append is also skipped during rebuild — the rebuild orchestrator disables both trail and audit side-effects.

3. **Audit is recoverable**: If the audit trail is needed after a rebuild, it can be reconstructed from the trail operations themselves (each trail op maps 1:1 to an audit action). A `znt rebuild --with-audit` flag could be added later if needed.

**In practice**: `replay_operation()` in `replayer.rs` calls `db.conn().execute()` directly (raw SQL), not `ZenService` repo methods. This naturally bypasses the audit + trail layers that `ZenService` methods would trigger.

---

### 9.8 FTS5 sync is automatic via triggers — no manual work needed — **LOW**

**Source**: `001_initial.sql` (22 triggers)

FTS5 virtual tables are kept in sync with content tables via SQL triggers defined in `001_initial.sql`. This means:

- **INSERT**: When you `INSERT INTO findings (...)`, the `findings_ai` trigger automatically inserts the corresponding row into `findings_fts`. No Rust code needed.
- **UPDATE**: The `findings_au` trigger does a **two-step FTS update**: first it deletes the old FTS entry using the special `INSERT INTO findings_fts(findings_fts, ...) VALUES ('delete', ...)` syntax, then inserts the new values. This is required by FTS5's design — you cannot directly update FTS rows. The special `'delete'` first-column value is FTS5's mechanism for removing entries from the inverted index.
- **DELETE**: The `findings_ad` trigger removes the FTS entry using the same `'delete'` syntax.

**Why this matters for Phase 2**: Repo methods do plain SQL `INSERT`/`UPDATE`/`DELETE` on content tables and get FTS sync for free. The replayer also gets FTS for free during rebuild — triggers fire on every replayed INSERT, so FTS5 indexes are fully populated after rebuild without any extra code. This was validated in spike 0.12 (FTS5 search works after rebuild).

---

### 9.9 `serde(alias)` is serde-safe but schema-unsafe — field rename rules — **LOW**

**Source**: Spike 0.16 (10/10 tests), `14-trail-versioning-spike-plan.md`

This gotcha matters for **future schema evolution**, not current Phase 2 work. But it must be documented now because violating it later would silently break `--strict` rebuild.

**The problem**: If we rename a field in a zen-core entity struct and add `#[serde(alias = "old_name")]` to maintain backward compatibility:

- **serde deserialization**: Works correctly. Old JSONL trail files with `"old_name"` deserialize into the renamed Rust field via the alias.
- **schemars schema generation**: Does NOT include the alias. The generated JSON Schema uses the Rust field name only. So `SchemaRegistry.validate()` will **reject** old trail data that uses `"old_name"`.
- **Consequence**: `znt rebuild --strict` would fail on old trail files that use the pre-rename field name, even though `serde` can deserialize them fine.

**Rules for field renames**:

1. **Preferred**: Don't rename fields. Add new fields with `#[serde(default)]` instead. Old field stays deprecated but functional.
2. **If rename is necessary**: Add `#[serde(alias = "old_name")]` for serde compat, but also add the old name to a "skip validation" list in the `--strict` replayer. Document the rename in a versioning changelog.
3. **If type changes**: Bump the trail version (`v: 2`) and add a migration function in the replayer's version dispatch (`match op.v { 1 => migrate_v1_to_v2(op), 2 => op, ... }`).
4. **The additive evolution rules** (from spike 0.16):
   - New `Option<T>` field → no version bump (deserializes as `None`, schema makes it not-required)
   - New `#[serde(default)]` field → no version bump (deserializes with default, schema excludes from `required`)
   - Change field type → version bump required
   - Make optional field required → version bump required

---

### 9.10 `Option<Option<T>>` in update builders — **LOW**

**Source**: Builder design decision

For nullable columns, update builders use the double-Option pattern:

```rust
pub struct FindingUpdate {
    pub source: Option<Option<String>>,  // nullable column
    pub content: Option<String>,          // non-nullable column
}
```

The semantics:
- `source: None` → "don't change this field" (omit from SET clause)
- `source: Some(Some("url"))` → "set to 'url'" (SET source = 'url')
- `source: Some(None)` → "set to NULL" (SET source = NULL)

For non-nullable columns (like `content`), a simple `Option<String>` suffices:
- `content: None` → "don't change"
- `content: Some("new text")` → "set to 'new text'"

Every update builder file must document this pattern in its module-level doc comment.

---

## 10. Milestone 2 Validation

### End-to-End Proof Chain (create -> audit -> trail -> rebuild)

1. Create/update/delete mutation executes SQL via `ZenService` repo method (planned mutation protocol)
2. Same mutation writes an audit row (`append_audit`)
3. Same mutation appends JSONL operation (`TrailWriter::append[_validated]`)
4. `TrailReplayer::rebuild()` replays all JSONL files and reproduces entity + FTS state

Concrete validation anchors:
- Rebuild parity: `spike_jsonl_replay_rebuild` (`zen-db/src/spike_jsonl.rs`)
- Rebuild + FTS survival: `spike_jsonl_rebuild_fts` (`zen-db/src/spike_jsonl.rs`)
- Version dispatch correctness: `spike_replay_dispatch_routes_by_version` (`zen-schema/src/spike_trail_versioning.rs`)

### Command

```bash
cargo test -p zen-db
```

### Acceptance Criteria

- [ ] All tests pass (Phase 1 tests + ~136 new Phase 2 tests)
- [ ] 13 repo modules complete: Session, Project, Research, Finding, Hypothesis, Insight, Issue, Task, ImplLog, Compat, Study, Link, Audit
- [ ] `whats_next()` returns correct aggregate counts
- [ ] Every mutation writes to audit trail (verified per entity)
- [ ] Every mutation writes to JSONL trail (verified per entity)
- [ ] JSONL trail validates Create ops against per-entity schema (warn-only)
- [ ] FTS5 search works for all 7 searchable entities + audit (porter stemming verified)
- [ ] Status transitions validated: hypothesis, task, issue, research, session, study
- [ ] Builder pattern produces correct partial SQL updates
- [ ] Trail replayer rebuilds DB from JSONL (identical state including FTS5)
- [ ] Trail version dispatch works (v=1 accepted, unsupported versions rejected)
- [ ] Multi-session concurrent trail files rebuild correctly (timestamp-ordered merge)
- [ ] Study full lifecycle works (create → add_assumption → record_test → conclude → progress)
- [ ] `cargo build --workspace` still succeeds (no regressions)

### What This Unlocks

Phase 2 completion unblocks:
- **Phase 4** (Search & Registry): `zen-search` uses `zen-db` FTS for knowledge entity search
- **Phase 5** (CLI Shell): All `znt` commands wire to `ZenService` repo methods
- **Phase 5 task 5.17** (`znt rebuild`): Calls `TrailReplayer::rebuild()`
- **Phase 5 tasks 5.18a-e** (Git hooks): Pre-commit validates JSONL via `zen-schema`

### Coverage Gaps (Integration Tests to Add)

1. **Mutation atomicity contract**: decide and test behavior when SQL succeeds but audit/trail write fails (rollback vs compensating write).
2. **Strict rebuild failure matrix**: add explicit integration tests for malformed lines, unknown entity data shapes, unsupported `op.v`, and error reporting guarantees.
3. **Cross-repo orchestration consistency**: table-driven tests asserting SQL -> audit -> trail ordering across representative repos (`finding`, `task`, `issue`, `study`).
4. **Audit detail action breadth**: expand integration coverage for all action-specific detail payloads (`status_changed`, `linked`, `tagged`, `indexed`) with pass/fail cases.
5. **Rebuild parity breadth**: add snapshot-level parity checks (counts, sampled IDs, links, FTS queries) before vs after rebuild across multi-session trails.

### Coverage Gaps -> Test Command Matrix

This matrix converts the gaps above into concrete first test targets.

| Gap | First test target | Suggested test name(s) | Run command |
|-----|-------------------|------------------------|-------------|
| Mutation atomicity contract | `zen-db/tests/integration/mutation_atomicity.rs` | `mutation_sql_succeeds_audit_fails_behavior`, `mutation_sql_and_audit_succeed_trail_fails_behavior` | `cargo test -p zen-db mutation_atomicity` |
| Strict rebuild failure matrix | `zen-db/tests/integration/rebuild_strict_failures.rs` | `rebuild_strict_rejects_malformed_json_line`, `rebuild_strict_rejects_invalid_create_payload`, `rebuild_rejects_unsupported_version` | `cargo test -p zen-db rebuild_strict_` |
| Cross-repo orchestration consistency | `zen-db/tests/integration/mutation_protocol.rs` | `finding_follows_sql_audit_trail_order`, `task_follows_sql_audit_trail_order`, `issue_follows_sql_audit_trail_order`, `study_follows_sql_audit_trail_order` | `cargo test -p zen-db mutation_protocol` |
| Audit detail action breadth | `zen-db/tests/integration/audit_detail_validation.rs` | `audit_status_changed_detail_validates`, `audit_linked_detail_validates`, `audit_tagged_detail_validates`, `audit_indexed_detail_validates` | `cargo test -p zen-db audit_detail_` |
| Rebuild parity breadth | `zen-db/tests/integration/rebuild_parity.rs` | `rebuild_parity_counts_ids_links_fts_single_session`, `rebuild_parity_counts_ids_links_fts_multi_session` | `cargo test -p zen-db rebuild_parity` |

Notes:
- Paths are suggested test locations to standardize where new integration tests live.
- If test module naming differs from current crate conventions, keep the test names and adapt only file/module paths.

---

## 11. Post-Review Amendments

**Review date**: 2026-02-09
**Review thread**: T-019c4342-0f0c-7749-926b-211e9d4ef1fa, T-019c43b3-189d-73cd-ac07-d9f19374b8dc
**Issues found**: 12 (3 BLOCKING, 4 HIGH, 4 MEDIUM, 1 LOW)
**Spike tests added**: 21 new tests in `spike_libsql.rs` (spikes 0.2b through 0.2g)
**All issues**: Resolved

### 11.1 Issue Register

| # | Issue | Severity | Resolution | Spike Evidence |
|---|-------|----------|------------|----------------|
| 1 | `unwrap_or("")` violates FK constraints with `PRAGMA foreign_keys = ON` | BLOCKING | Use `params!` with `Option<T>` for fixed-param queries (maps `None → NULL` natively); use `libsql::Value::Null` via `.into()` in `Vec<Value>` for dynamic updates | `spike_empty_string_violates_fk_constraint`, `spike_replay_unwrap_or_empty_breaks_fk` |
| 2 | `Vec<Box<dyn IntoValue>>` for dynamic UPDATE won't compile | BLOCKING | Use `Vec<libsql::Value>` + `params_from_iter()` | `spike_dynamic_update_with_params_from_iter`, `spike_dynamic_update_set_null_with_params_from_iter`, `spike_vec_value_directly_as_params` |
| 3 | Mutation protocol has no transaction boundary | HIGH | Wrap SQL + audit in `conn.transaction()`, write trail before `commit()` | `spike_transaction_rollback_on_trail_failure`, `spike_transaction_implicit_rollback_on_drop`, `spike_full_mutation_protocol_with_file_trail` |
| 4 | `ProjectMeta` and `ProjectDependency` missing from `EntityType` | HIGH | Add `ProjectMeta`, `ProjectDep` variants to `EntityType`, trail, and replayer | Design decision (no spike needed) |
| 5 | `get_linked_entities::<T>()` generic method undefined | HIGH | Replace with explicit per-type methods: `get_linked_hypotheses()`, `get_linked_findings()`, `get_linked_insights()` | Design decision (consistent with helpers-not-traits approach) |
| 6 | Replayer can't distinguish "set to NULL" from "not changed" | HIGH | `json_to_update_value()` helper: absent key → `None` (skip), JSON `null` → `Some(Null)`, string → `Some(Text)` | `spike_replay_null_vs_absent_vs_value`, `spike_option_option_serde_roundtrip_for_replay` |
| 7 | `AuditFilter` missing `#[derive(Default)]` | MEDIUM | Add `#[derive(Default)]` | Trivial fix |
| 8 | `query_audit` dynamic params use `Vec<String>` | MEDIUM | Use `Vec<libsql::Value>` (same as issue 2) | Covered by issue 2 spikes |
| 9 | Session snapshots not in trail | MEDIUM | Recomputed after rebuild (computed aggregates, not user-written) | Design decision |
| 10 | `conclude_study()` skips `Concluding` state | MEDIUM | Two-step: Active → Concluding → Completed inside `conclude_study()` | Design decision (state machine enforced) |
| 11 | Concurrent writes to same session JSONL file | LOW | POSIX O_APPEND atomic for lines < 4KB; validated safe | `spike_concurrent_same_session_file_append` |
| 12 | SQL injection surface in `count_by_status(&str)` | LOW | Change to `count_by_status(EntityType)`, table name from `&'static str` match | `spike_entity_type_table_mapping_is_exhaustive`, `spike_count_by_status_with_enum_is_safe` |

### 11.2 Key Patterns Established

**NULL binding (write side — INSERT/fixed params)**:
```rust
// Option<T> works natively in params! macro (spike 0.2g)
// .as_deref() converts Option<String> → Option<&str>
libsql::params![
    entity.id.as_str(),
    entity.nullable_fk.as_deref(),  // None → NULL, Some("id") → "id"
    entity.required_field.as_str()
]
```

**NULL binding (dynamic updates — Vec<Value>)**:
```rust
// .into() on Option<&str> produces Value::Null or Value::Text
let mut vals: Vec<libsql::Value> = Vec::new();
if let Some(ref source_opt) = update.source {
    vals.push(source_opt.as_deref().into());  // Option<&str> → Value
    sets.push(format!("source = ?{}", vals.len()));
}
```

**Replay field extraction** (read side):
```rust
fn json_to_value(data: &serde_json::Value, field: &str) -> libsql::Value {
    match data.get(field) {
        None | Some(serde_json::Value::Null) => libsql::Value::Null,
        Some(serde_json::Value::String(s)) => libsql::Value::Text(s.clone()),
        Some(v) => libsql::Value::Text(v.to_string()),
    }
}

fn json_to_update_value(data: &serde_json::Value, field: &str) -> Option<libsql::Value> {
    match data.get(field) {
        None => None,  // absent → don't change this column
        Some(serde_json::Value::Null) => Some(libsql::Value::Null),  // explicit null → SET to NULL
        Some(serde_json::Value::String(s)) => Some(libsql::Value::Text(s.clone())),
        Some(v) => Some(libsql::Value::Text(v.to_string())),
    }
}
```

**Dynamic UPDATE builder** (update repos):
```rust
let mut sets = Vec::new();
let mut vals: Vec<libsql::Value> = Vec::new();

if let Some(ref content) = update.content {
    vals.push(content.as_str().into());
    sets.push(format!("content = ?{}", vals.len()));
}
if let Some(ref source_opt) = update.source {
    vals.push(source_opt.as_deref().into());
    sets.push(format!("source = ?{}", vals.len()));
}
// ... more fields ...
vals.push(id.into());
let sql = format!("UPDATE findings SET {} WHERE id = ?{}", sets.join(", "), vals.len());
conn.execute(&sql, vals).await?;
```

**Transaction-wrapped mutation protocol**:
```rust
let tx = self.db.conn().transaction().await?;
tx.execute(entity_sql, entity_params).await?;     // 1. SQL
tx.execute(audit_sql, audit_params).await?;        // 2. Audit (same tx)
self.trail.append(&trail_op)?;                     // 3. Trail (file I/O)
tx.commit().await?;                                // 4. Commit
// If trail fails → tx drops → implicit rollback → no orphaned DB state
```

### 11.3 EntityType Additions Required

Before PR 2 begins, add these variants to `zen-core/src/enums.rs`:

```rust
pub enum EntityType {
    // ... existing variants ...
    ProjectMeta,    // NEW — for project_meta table trail coverage
    ProjectDep,     // NEW — for project_dependencies table trail coverage
}
```

Update `as_str()`, `Display`, and serde roundtrip tests accordingly.

### 11.4 Test Count Update

| Crate | Phase 1 Tests | New Spike Tests | Total |
|-------|--------------|-----------------|-------|
| zen-core | 73 | 0 | 73 |
| zen-schema | 42 | 0 | 42 |
| zen-db (production) | 12 | 0 | 12 |
| zen-db (spikes) | 56 | 21 | 77 |
| **Total** | **183** | **21** | **204** |

Note: 9 remote Turso/Clerk spike tests require network access and may fail locally.

---

## Cross-References

- Entity SQL schemas: [01-turso-data-model.md](./01-turso-data-model.md)
- Crate designs (repo patterns, module layout): [05-crate-designs.md](./05-crate-designs.md) §5
- JSONL trail strategy (Approach B): [10-git-jsonl-strategy.md](./10-git-jsonl-strategy.md)
- Studies spike (Approach B, entity_links): [08-studies-spike-plan.md](./08-studies-spike-plan.md)
- Schema validation spike (SchemaRegistry, per-entity dispatch): [12-schema-spike-plan.md](./12-schema-spike-plan.md)
- Trail versioning spike (Approach D, `v` field): [14-trail-versioning-spike-plan.md](./14-trail-versioning-spike-plan.md)
- Implementation plan (phase overview): [07-implementation-plan.md](./07-implementation-plan.md) §4
- Phase 1 plan (predecessor): [19-phase1-foundation-plan.md](./19-phase1-foundation-plan.md)
- Validated spike code:
  - `zen-db/src/spike_libsql.rs` — raw SQL patterns, session/finding/audit insert
  - `zen-db/src/spike_studies.rs` (15/15) — study lifecycle, entity_links join, progress query
  - `zen-db/src/spike_jsonl.rs` (15/15) — trail write/read/replay, concurrent safety, FTS survival
  - `zen-schema/src/spike_schema_gen.rs` (22/22) — per-entity schema dispatch, SchemaRegistry
  - `zen-schema/src/spike_trail_versioning.rs` (10/10) — version dispatch, additive evolution
