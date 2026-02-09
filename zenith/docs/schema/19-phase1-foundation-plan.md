# Phase 1: Foundation — Implementation Plan

**Version**: 2026-02-08
**Status**: Ready to Execute
**Depends on**: Phase 0 (all 20 spikes DONE)
**Produces**: Milestone 1 — `cargo test -p zen-core -p zen-config -p zen-db -p zen-schema` all pass

---

## Table of Contents

1. [Overview](#1-overview)
2. [Current State](#2-current-state)
3. [Key Decisions](#3-key-decisions)
4. [Stream A: zen-core](#4-stream-a-zen-core)
5. [Stream B: zen-schema](#5-stream-b-zen-schema)
6. [Stream C: zen-db Foundation](#6-stream-c-zen-db-foundation)
7. [Execution Order](#7-execution-order)
8. [Milestone 1 Validation](#8-milestone-1-validation)

---

## 1. Overview

**Goal**: Core types, error handling, configuration (done), database schema, and schema validation registry.

**Crates touched**: `zen-core` (heavy), `zen-schema` (medium), `zen-db` (medium)

**zen-config**: Already DONE — 46/46 tests, production code. No work needed.

**Estimated deliverables**: ~20 new files, ~2500–3500 LOC production code, ~1500 LOC tests

---

## 2. Current State

| Crate | Status | What Exists |
|-------|--------|-------------|
| **zen-config** | DONE | Full production code, 46/46 tests. 7 modules (lib, error, turso, motherduck, r2, clerk, axiom, general). Figment layered config. |
| **zen-core** | Stub | Only `arrow_serde.rs` (205 lines). No entity structs, no enums, no errors, no IDs. |
| **zen-db** | Stub | 6 spike modules behind `#[cfg(test)]`. No production code, no migrations directory. |
| **zen-schema** | Stub | 2 spike modules behind `#[cfg(test)]`. No `SchemaRegistry`, no validation. |

All entity definitions exist in validated spike code (`spike_schema_gen.rs`, 22/22 tests) and design docs (`05-crate-designs.md`, `01-turso-data-model.md`) — they need to be promoted to production modules.

---

## 3. Key Decisions

Decisions made before execution, with rationale:

### 3.1 TrailOp Enum: Broad (8 variants)

**Decision**: `Create, Update, Delete, Link, Unlink, Tag, Untag, Transition`

**Rationale**: Maps naturally to audit actions. The git hooks spike schema (0.13) already uses this broader set. More explicit than overloading `Create` for link/tag operations.

### 3.2 Error Hierarchy: Deferred Unified ZenError

**Decision**: zen-core defines only cross-cutting errors (`CoreError` with `NotFound`, `InvalidTransition`, `Validation`, `Other`). Each downstream crate (zen-db, zen-lake, zen-parser, etc.) defines its own error type wrapping its real dependencies. `ZenError` (the unified enum with `#[from]` for all sub-errors) is deferred to zen-cli where all crate errors converge.

**Rationale**: zen-core can't import `libsql::Error` (that's in zen-db). Premature unification would either require stub errors or `Box<dyn Error>` type erasure. Each crate owning its error type is cleaner and avoids circular deps.

### 3.3 zen-schema depends on zen-core

**Decision**: zen-schema imports zen-core entity types and uses `schemars::schema_for!()` on them. Single source of truth for type definitions.

**Rationale**: Avoids duplicating entity structs. The spike defined structs locally inside `#[cfg(test)]` which was appropriate for validation but wrong for production.

### 3.4 zen-core Gets schemars + anyhow

**Decision**: Add `schemars` (with `chrono04` feature) for `#[derive(JsonSchema)]` on all entity types and enums. Add `anyhow` for `CoreError::Other`.

### 3.5 additionalProperties Convention

**Decision** (from spike 0.15/0.16): Permissive for trail operations (accept unknown fields for forward-compat). Strict for config (`#[serde(deny_unknown_fields)]` generates `additionalProperties: false`).

---

## 4. Stream A: zen-core

Everything downstream depends on this. Must go first.

### A1. Update `Cargo.toml`

Add to `[dependencies]`:
```toml
schemars.workspace = true    # for #[derive(JsonSchema)] — uses chrono04 feature from workspace
anyhow.workspace = true      # for CoreError::Other
```

### A2. `src/ids.rs` — ID Prefix Constants + Helpers

12 prefix constants and 2 helper functions.

```rust
pub const PREFIX_SESSION: &str = "ses";
pub const PREFIX_RESEARCH: &str = "res";
pub const PREFIX_FINDING: &str = "fnd";
pub const PREFIX_HYPOTHESIS: &str = "hyp";
pub const PREFIX_INSIGHT: &str = "ins";
pub const PREFIX_ISSUE: &str = "iss";
pub const PREFIX_TASK: &str = "tsk";
pub const PREFIX_IMPL_LOG: &str = "imp";
pub const PREFIX_COMPAT: &str = "cmp";
pub const PREFIX_STUDY: &str = "stu";
pub const PREFIX_LINK: &str = "lnk";
pub const PREFIX_AUDIT: &str = "aud";

/// Format a prefixed ID. Called after DB generates the random part.
pub fn format_id(prefix: &str, random: &str) -> String;

/// SQL expression for generating a prefixed ID inside an INSERT.
pub fn gen_id_sql(prefix: &str) -> String;
```

**Source**: `05-crate-designs.md` §3 lines 236–258

### A3. `src/enums.rs` — All Status/Type Enums (14 enums)

All enums derive: `Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema`
All use: `#[serde(rename_all = "snake_case")]`

| Enum | Variants | Notes |
|------|----------|-------|
| `Confidence` | High, Medium, Low | |
| `HypothesisStatus` | Unverified, Analyzing, Confirmed, Debunked, PartiallyConfirmed, Inconclusive | Has `allowed_next_states()` |
| `TaskStatus` | Open, InProgress, Done, Blocked | Has `allowed_next_states()` |
| `IssueStatus` | Open, InProgress, Done, Blocked, Abandoned | Has `allowed_next_states()` |
| `IssueType` | Bug, Feature, Spike, Epic, Request | |
| `ResearchStatus` | Open, InProgress, Resolved, Abandoned | Has `allowed_next_states()` |
| `SessionStatus` | Active, WrappedUp, Abandoned | Has `allowed_next_states()` |
| `StudyStatus` | Active, Concluding, Completed, Abandoned | Has `allowed_next_states()` |
| `StudyMethodology` | Explore, TestDriven, Compare | |
| `CompatStatus` | Compatible, Incompatible, Conditional, Unknown | |
| `AuditAction` | Created, Updated, StatusChanged, Linked, Unlinked, Tagged, Untagged, Indexed, SessionStart, SessionEnd, WrapUp | |
| `EntityType` | Session, Research, Finding, Hypothesis, Insight, Issue, Task, ImplLog, Compat, Study, EntityLink, Audit | |
| `Relation` | Blocks, Validates, Debunks, Implements, RelatesTo, DerivedFrom, Triggers, Supersedes, DependsOn | |
| `TrailOp` | Create, Update, Delete, Link, Unlink, Tag, Untag, Transition | 8 variants (broad set) |

Status enums with state machines get `allowed_next_states(&self) -> &[Self]` implementing the transitions documented in `01-turso-data-model.md`. All enums get `as_str(&self) -> &str` for SQL storage.

**Transition rules** (from data model):

```
HypothesisStatus: unverified → analyzing → confirmed|debunked|partially_confirmed|inconclusive
TaskStatus:       open → in_progress → done|blocked; blocked → in_progress
IssueStatus:      open → in_progress → done|blocked|abandoned; blocked → in_progress
ResearchStatus:   open → in_progress → resolved|abandoned
SessionStatus:    active → wrapped_up|abandoned
StudyStatus:      active → concluding → completed|abandoned; active → abandoned
```

### A4. `src/entities/` — 14 Entity Structs

Module structure:
```
entities/
  mod.rs          re-exports all entities
  session.rs      Session, SessionSnapshot
  research.rs     ResearchItem
  finding.rs      Finding
  hypothesis.rs   Hypothesis
  insight.rs      Insight
  issue.rs        Issue
  task.rs         Task
  impl_log.rs     ImplLog
  compat.rs       CompatCheck
  study.rs        Study
  link.rs         EntityLink
  audit.rs        AuditEntry
  project.rs      ProjectMeta, ProjectDependency
```

All structs derive: `Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq`

Field types come from `01-turso-data-model.md` (SQL columns) + `spike_schema_gen.rs` (validated shapes). Timestamps as `DateTime<Utc>`.

**Entity field specifications** (from SQL schema + spike):

```rust
// session.rs
pub struct Session {
    pub id: String,
    pub started_at: DateTime<Utc>,
    pub ended_at: Option<DateTime<Utc>>,
    pub status: SessionStatus,
    pub summary: Option<String>,
}

pub struct SessionSnapshot {
    pub session_id: String,
    pub open_tasks: i64,
    pub in_progress_tasks: i64,
    pub pending_hypotheses: i64,
    pub unverified_hypotheses: i64,
    pub recent_findings: i64,
    pub open_research: i64,
    pub summary: String,
    pub created_at: DateTime<Utc>,
}

// research.rs
pub struct ResearchItem {
    pub id: String,
    pub session_id: Option<String>,
    pub title: String,
    pub description: Option<String>,
    pub status: ResearchStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// finding.rs
pub struct Finding {
    pub id: String,
    pub research_id: Option<String>,
    pub session_id: Option<String>,
    pub content: String,
    pub source: Option<String>,
    pub confidence: Confidence,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// hypothesis.rs
pub struct Hypothesis {
    pub id: String,
    pub research_id: Option<String>,
    pub finding_id: Option<String>,
    pub session_id: Option<String>,
    pub content: String,
    pub status: HypothesisStatus,
    pub reason: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// insight.rs
pub struct Insight {
    pub id: String,
    pub research_id: Option<String>,
    pub session_id: Option<String>,
    pub content: String,
    pub confidence: Confidence,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// issue.rs
pub struct Issue {
    pub id: String,
    pub issue_type: IssueType,
    pub parent_id: Option<String>,
    pub title: String,
    pub description: Option<String>,
    pub status: IssueStatus,
    pub priority: u8,         // 1 (highest) to 5 (lowest)
    pub session_id: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// task.rs
pub struct Task {
    pub id: String,
    pub research_id: Option<String>,
    pub issue_id: Option<String>,
    pub session_id: Option<String>,
    pub title: String,
    pub description: Option<String>,
    pub status: TaskStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// impl_log.rs
pub struct ImplLog {
    pub id: String,
    pub task_id: String,
    pub session_id: Option<String>,
    pub file_path: String,
    pub start_line: Option<i64>,
    pub end_line: Option<i64>,
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
}

// compat.rs
pub struct CompatCheck {
    pub id: String,
    pub package_a: String,
    pub package_b: String,
    pub status: CompatStatus,
    pub conditions: Option<String>,
    pub finding_id: Option<String>,
    pub session_id: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// study.rs
pub struct Study {
    pub id: String,
    pub session_id: Option<String>,
    pub research_id: Option<String>,
    pub topic: String,
    pub library: Option<String>,
    pub methodology: StudyMethodology,
    pub status: StudyStatus,
    pub summary: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// link.rs
pub struct EntityLink {
    pub id: String,
    pub source_type: EntityType,
    pub source_id: String,
    pub target_type: EntityType,
    pub target_id: String,
    pub relation: Relation,
    pub created_at: DateTime<Utc>,
}

// audit.rs
pub struct AuditEntry {
    pub id: String,
    pub session_id: Option<String>,
    pub entity_type: EntityType,
    pub entity_id: String,
    pub action: AuditAction,
    pub detail: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
}

// project.rs
pub struct ProjectMeta {
    pub key: String,
    pub value: String,
    pub updated_at: DateTime<Utc>,
}

pub struct ProjectDependency {
    pub ecosystem: String,
    pub name: String,
    pub version: Option<String>,
    pub source: String,
    pub indexed: bool,
    pub indexed_at: Option<DateTime<Utc>>,
}
```

### A5. `src/errors.rs` — Core Error Types

Cross-cutting errors only. Each downstream crate owns its domain-specific error type.

```rust
#[derive(Debug, Error)]
pub enum CoreError {
    #[error("Entity not found: {entity_type} {id}")]
    NotFound { entity_type: String, id: String },

    #[error("Invalid state transition: {entity_type} {id} from {from} to {to}")]
    InvalidTransition { entity_type: String, id: String, from: String, to: String },

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("{0}")]
    Other(#[from] anyhow::Error),
}
```

### A6. `src/trail.rs` — Trail Operation Envelope

```rust
fn default_trail_version() -> u32 { 1 }

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct TrailOperation {
    #[serde(default = "default_trail_version")]
    pub v: u32,
    pub ts: String,
    pub ses: String,
    pub op: TrailOp,
    pub entity: EntityType,
    pub id: String,
    pub data: serde_json::Value,
}
```

Per spike 0.16: `v` defaults to 1 when absent from old JSONL (backward compat with `#[serde(default)]`).

### A7. `src/responses.rs` — CLI Response Types

```rust
pub struct FindingCreateResponse { pub finding: Finding }
pub struct SessionStartResponse { pub session: Session, pub previous_session: Option<Session> }
pub struct WhatsNextResponse {
    pub last_session: Option<Session>,
    pub open_tasks: Vec<Task>,
    pub pending_hypotheses: Vec<Hypothesis>,
    pub recent_audit: Vec<AuditEntry>,
}
pub struct SearchResult {
    pub package: String, pub ecosystem: String, pub kind: String,
    pub name: String, pub signature: Option<String>, pub doc_comment: Option<String>,
    pub file_path: Option<String>, pub line_start: Option<u32>, pub score: f64,
}
pub struct SearchResultsResponse { pub query: String, pub results: Vec<SearchResult>, pub total_results: u32 }
pub struct RebuildResponse {
    pub rebuilt: bool, pub trail_files: u32, pub operations_replayed: u32,
    pub entities_created: u32, pub duration_ms: u64,
}
```

### A8. `src/audit_detail.rs` — Audit Detail Sub-Types

```rust
pub struct StatusChangedDetail { pub from: String, pub to: String, pub reason: Option<String> }
pub struct LinkedDetail {
    pub source_type: String, pub source_id: String,
    pub target_type: String, pub target_id: String, pub relation: String,
}
pub struct TaggedDetail { pub tag: String }
pub struct IndexedDetail { pub package: String, pub ecosystem: String, pub symbols: u32, pub duration_ms: u64 }
```

### A9. Update `src/lib.rs`

Wire all new modules:
```rust
pub mod arrow_serde;
pub mod ids;
pub mod enums;
pub mod entities;
pub mod errors;
pub mod trail;
pub mod responses;
pub mod audit_detail;
```

### A10. Tests

Located in `src/` module tests or `tests/` directory:

- **ID tests**: All 12 prefixes produce correct format. `gen_id_sql("fnd")` produces `'fnd-' || lower(hex(randomblob(4)))`. `format_id("fnd", "a3f8b2c1")` produces `"fnd-a3f8b2c1"`.
- **Enum serde tests**: Each enum serializes to snake_case and deserializes back. E.g., `HypothesisStatus::PartiallyConfirmed` → `"partially_confirmed"` → `HypothesisStatus::PartiallyConfirmed`.
- **Enum `as_str` tests**: Every variant produces the expected string.
- **Transition tests**: Valid transitions succeed, invalid transitions return the correct error. E.g., `HypothesisStatus::Unverified.allowed_next_states()` contains `Analyzing` but not `Confirmed`.
- **Entity serde roundtrip**: Every entity struct serializes to JSON and deserializes back identically.
- **JsonSchema generation**: `schema_for!(Finding)` produces a valid JSON Schema. Serialize a `Finding` instance → validate against the schema → passes. Serialize invalid JSON → validate → fails with descriptive errors.

---

## 5. Stream B: zen-schema

Depends on zen-core types being complete (imports entity types for `schema_for!`).

### B1. Update `Cargo.toml`

Add to `[dependencies]`:
```toml
zen-core.workspace = true
chrono.workspace = true      # needed for schema_for! on DateTime types
```

### B2. `src/error.rs` — `SchemaError`

```rust
#[derive(Debug, Error)]
pub enum SchemaError {
    #[error("Schema not found: {0}")]
    NotFound(String),

    #[error("Validation failed: {errors:?}")]
    ValidationFailed { errors: Vec<String> },

    #[error("Schema generation error: {0}")]
    Generation(String),
}
```

### B3. `src/registry.rs` — `SchemaRegistry`

Core struct: `HashMap<String, serde_json::Value>` storing named JSON Schemas.

```rust
pub struct SchemaRegistry {
    schemas: HashMap<String, serde_json::Value>,
}

impl SchemaRegistry {
    /// Build registry with all ~39 schemas from zen-core types.
    pub fn new() -> Self;

    /// Get a schema by name. Returns None if not found.
    pub fn get(&self, name: &str) -> Option<&serde_json::Value>;

    /// Validate a JSON value against a named schema.
    pub fn validate(&self, name: &str, instance: &serde_json::Value) -> Result<(), SchemaError>;

    /// List all registered schema names.
    pub fn list(&self) -> Vec<&str>;

    /// Number of registered schemas.
    pub fn schema_count(&self) -> usize;
}
```

Schema categories populated in `new()`:

| Category | Count | Types |
|----------|-------|-------|
| Entities | 14 | Session, SessionSnapshot, ResearchItem, Finding, Hypothesis, Insight, Issue, Task, ImplLog, CompatCheck, Study, EntityLink, AuditEntry, ProjectMeta, ProjectDependency |
| Trail | 1 | TrailOperation |
| Responses | 6 | FindingCreateResponse, SessionStartResponse, WhatsNextResponse, SearchResult, SearchResultsResponse, RebuildResponse |
| Audit details | 4 | StatusChangedDetail, LinkedDetail, TaggedDetail, IndexedDetail |
| **Total** | **~25** | |

(Enums are referenced within entity schemas, not stored as separate top-level entries unless needed.)

### B4. Wire `src/lib.rs`

```rust
pub mod error;
pub mod registry;

pub use error::SchemaError;
pub use registry::SchemaRegistry;
```

Keep spike modules behind `#[cfg(test)]`.

### B5. Tests

- **Construction**: `SchemaRegistry::new()` succeeds. `schema_count()` returns expected number.
- **Get**: `get("finding")` returns `Some`. `get("nonexistent")` returns `None`.
- **List**: `list()` contains all expected names.
- **Valid entity roundtrip**: For each entity type — create struct, serialize to JSON, `validate("finding", &json)` → `Ok(())`.
- **Invalid entity rejection**: Missing required field → `Err(ValidationFailed)` with descriptive errors.
- **Trail validation**: Valid `TrailOperation` JSON → `Ok`. Missing `ts` → `Err`.
- **Audit detail validation**: `StatusChangedDetail` JSON matches schema. Wrong type fails.

---

## 6. Stream C: zen-db Foundation

Depends on zen-core types (used in test assertions). Independent of zen-schema.

### C1. `migrations/001_initial.sql` — Full Database Schema

Assembled from `01-turso-data-model.md`. All statements use `IF NOT EXISTS` for idempotent re-running.

Contents (in order):
1. `PRAGMA foreign_keys = ON;`
2. 14 `CREATE TABLE IF NOT EXISTS` (project_meta, project_dependencies, sessions, session_snapshots, research_items, findings, finding_tags, hypotheses, insights, issues, tasks, implementation_log, studies, compatibility_checks, entity_links, audit_trail)
3. 8 `CREATE VIRTUAL TABLE IF NOT EXISTS` (findings_fts, hypotheses_fts, insights_fts, research_fts, tasks_fts, issues_fts, studies_fts, audit_fts)
4. 31 `CREATE INDEX IF NOT EXISTS`
5. 22 triggers (AFTER INSERT/UPDATE/DELETE for FTS5 sync)

**Source**: `01-turso-data-model.md` §§2–11 (complete SQL for all statements)

### C2. `src/error.rs` — `DatabaseError`

```rust
#[derive(Debug, Error)]
pub enum DatabaseError {
    #[error("Query failed: {0}")]
    Query(String),

    #[error("Migration failed: {0}")]
    Migration(String),

    #[error("No result returned")]
    NoResult,

    #[error("Invalid state: {0}")]
    InvalidState(String),

    #[error("libSQL error: {0}")]
    LibSql(#[from] libsql::Error),

    #[error("{0}")]
    Other(#[from] anyhow::Error),
}
```

### C3. `src/lib.rs` — `ZenDb` Struct

```rust
pub struct ZenDb {
    db: libsql::Database,
    conn: libsql::Connection,
}

impl ZenDb {
    /// Open local-only database (no cloud sync).
    pub async fn open_local(path: &str) -> Result<Self, DatabaseError>;

    /// Access the connection for repo operations.
    pub fn conn(&self) -> &libsql::Connection;

    /// Generate a prefixed ID via libSQL. Returns e.g., "fnd-a3f8b2c1".
    pub async fn generate_id(&self, prefix: &str) -> Result<String, DatabaseError>;
}
```

Keep existing spike modules behind `#[cfg(test)]`.

### C4. `src/migrations.rs` — Migration Runner

```rust
const MIGRATION_001: &str = include_str!("../migrations/001_initial.sql");

impl ZenDb {
    pub(crate) async fn run_migrations(&self) -> Result<(), DatabaseError>;
}
```

Uses `conn.execute_batch(MIGRATION_001)`. Single migration for Phase 1. Version tracking can be added in Phase 2 if needed.

### C5. Tests

All tests are `#[tokio::test]` since libsql is async.

- **Schema creation**: `open_local(":memory:")` succeeds. Query `sqlite_master` — verify all 14 tables exist, 8 FTS5 tables exist.
- **ID generation**: `generate_id("ses")` returns string matching `ses-[0-9a-f]{8}`. Generate 100 IDs, assert all unique. Each prefix produces correct format.
- **Basic INSERT+SELECT per table**: Insert a row into each of the 14 content tables, SELECT it back, verify all fields match. (Uses raw SQL, not repo methods — repos are Phase 2.)
- **FTS5 works**: Insert a finding with content "tokio async runtime". Query `findings_fts MATCH 'runtime'` returns the row.
- **FTS5 triggers work**: Insert a finding, verify `findings_fts` is populated (trigger fires). Update the finding, verify FTS reflects new content.
- **Foreign keys**: Insert a finding referencing nonexistent `session_id` → fails (foreign key constraint).
- **Unique constraints**: Insert duplicate `entity_links` with same (source, target, relation) → fails.

---

## 7. Execution Order

```
Phase 1 Execution:

 1. [A1]  Update zen-core Cargo.toml (add schemars, anyhow)
 2. [A2]  Create src/ids.rs
 3. [A3]  Create src/enums.rs (14 enums)
 4. [A4]  Create src/entities/ (14 entity structs in 14 files + mod.rs)
 5. [A5]  Create src/errors.rs (CoreError)
 6. [A6]  Create src/trail.rs (TrailOperation envelope)
 7. [A7]  Create src/responses.rs (CLI response types)
 8. [A8]  Create src/audit_detail.rs
 9. [A9]  Update src/lib.rs (wire all modules)
10. [A10] Write zen-core tests
    ─── cargo test -p zen-core passes ───

    ┌─────────────────────────────────────┐
    │ B and C can run in parallel from    │
    │ here — both depend only on zen-core │
    └─────────────────────────────────────┘

11. [B1]  Update zen-schema Cargo.toml (add zen-core, chrono)
12. [B2]  Create src/error.rs (SchemaError)
13. [B3]  Create src/registry.rs (SchemaRegistry)
14. [B4]  Wire src/lib.rs
15. [B5]  Write zen-schema tests
    ─── cargo test -p zen-schema passes ───

16. [C1]  Create migrations/001_initial.sql
17. [C2]  Create src/error.rs (DatabaseError)
18. [C3]  Rewrite src/lib.rs (ZenDb struct + open_local)
19. [C4]  Create src/migrations.rs
20. [C5]  Write zen-db tests
    ─── cargo test -p zen-db passes ───
```

Steps 11–15 and 16–20 are independent and can be parallelized.

---

## 8. Milestone 1 Validation

### Command

```bash
cargo test -p zen-core -p zen-config -p zen-db -p zen-schema
```

### Acceptance Criteria

- [ ] All tests pass (zen-core, zen-config, zen-db, zen-schema)
- [ ] Database opens with `open_local()`, full schema applied (14 tables, 8 FTS5, 31 indexes, 22 triggers)
- [ ] `generate_id()` produces correctly prefixed 12-char IDs
- [ ] Every entity struct serializes/deserializes via serde (JSON roundtrip)
- [ ] Every entity validates against its JSON Schema via `SchemaRegistry`
- [ ] FTS5 search returns results (porter stemming works)
- [ ] Status transition validation: valid transitions succeed, invalid rejected
- [ ] Basic INSERT+SELECT works for every table
- [ ] `SchemaRegistry` loads all entity + trail + response + audit detail schemas
- [ ] `cargo build --workspace` still succeeds (no regressions to other stub crates)

### What This Unlocks

Phase 1 completion unblocks:
- **Phase 2** (Storage Layer): All 13 repo modules build on `ZenDb` + entity types
- **Phase 3** (Parsing & Indexing): Uses zen-core types for `ParsedItem` → `ApiSymbol` mapping
- **Phase 5 tasks 5.18a–e** (Git hooks): Uses zen-schema for JSONL trail validation

---

## Cross-References

- Entity SQL schemas: [01-turso-data-model.md](./01-turso-data-model.md)
- Crate designs (entity structs, module layouts): [05-crate-designs.md](./05-crate-designs.md)
- Schema spike (schemars + jsonschema patterns): [12-schema-spike-plan.md](./12-schema-spike-plan.md)
- Trail versioning spike (v field, additive evolution): [14-trail-versioning-spike-plan.md](./14-trail-versioning-spike-plan.md)
- Implementation plan (phase overview): [07-implementation-plan.md](./07-implementation-plan.md)
- Validated spike code: `zen-schema/src/spike_schema_gen.rs` (22/22), `zen-db/src/spike_jsonl.rs` (15/15), `zen-schema/src/spike_trail_versioning.rs` (10/10)
