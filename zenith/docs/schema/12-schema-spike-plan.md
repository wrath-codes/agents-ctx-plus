# Zenith: JSON Schema Generation & Validation — Spike Plan

**Version**: 2026-02-08
**Status**: Pending
**Purpose**: Validate `schemars` 1.x for auto-generating JSON Schemas from all Zenith types, and `jsonschema` 0.28 for runtime validation at every JSON boundary. Prove the full pipeline: Rust struct → `#[derive(JsonSchema)]` → generated schema → `jsonschema::validate()` → descriptive errors.
**Spike ID**: 0.14
**Crate**: zen-schema (new, 11th in workspace)
**Blocks**: Phase 1 (entity structs get `#[derive(JsonSchema)]`), Phase 2 (trail + audit validation), Phase 3 (DuckDB metadata validation), Phase 5 (`znt schema` command, pre-commit uses generated schemas)

---

## Table of Contents

1. [Motivation](#1-motivation)
2. [Background & Prior Art](#2-background--prior-art)
3. [The 10 Integration Points](#3-the-10-integration-points)
4. [New Crate: zen-schema](#4-new-crate-zen-schema)
5. [Dependencies](#5-dependencies)
6. [Spike Tests](#6-spike-tests)
7. [Evaluation Criteria](#7-evaluation-criteria)
8. [What This Spike Does NOT Test](#8-what-this-spike-does-not-test)
9. [Success Criteria](#9-success-criteria)
10. [Post-Spike Actions](#10-post-spike-actions)

---

## 1. Motivation

Zenith has **10 distinct JSON boundaries** where data crosses between systems or components. Today, only one of these has schema validation: the pre-commit hook from spike 0.13, which uses a hand-written JSON Schema to validate JSONL trail files. Every other boundary relies on serde deserialization alone — which catches type errors but not semantic errors (wrong enum values, missing required fields in freeform JSON, invalid data shapes for a given entity type).

The highest-risk boundary is `Operation.data: serde_json::Value` — the payload field in every JSONL trail operation. Its shape varies by entity type (a finding create needs `{content, confidence, ...}`, a hypothesis update needs `{status, ...}`), but the write path has **zero validation**. The replay function uses `op.data["field"].as_str().unwrap_or("")` — silent defaults on malformed data. Corrupt trail files produce corrupt databases with no error.

### Why Now (Phase 0)

Schema infrastructure must be available starting Phase 1, because:
- Phase 1 defines entity structs → they need `#[derive(JsonSchema)]`
- Phase 2 uses schemas for trail writer and audit detail validation
- Phase 3 uses schemas for DuckDB metadata column validation
- Phase 5 uses schemas for CLI output, `znt schema` command, and pre-commit hooks

If we discover `schemars` doesn't handle our types (nested optionals, HashMaps, `DateTime<Utc>`, `serde_json::Value`, `#[serde(rename_all)]` enums), we find out in Phase 0, not Phase 2.

### Why a New Crate

JSON Schema generation and validation cross-cuts every crate in the workspace:

| Crate | Uses Schema For |
|-------|----------------|
| zen-core | Entity structs define schemas via `#[derive(JsonSchema)]` |
| zen-schema | SchemaRegistry, validate(), export(), per-entity dispatch |
| zen-db | Trail writer validation, audit detail validation, rebuild `--strict` |
| zen-hooks | Pre-commit JSONL validation (replaces hand-written schema from 0.13) |
| zen-lake | DuckDB metadata column validation (Phase 3) |
| zen-cli | `znt schema` command, response struct schemas, input validation |

A dedicated `zen-schema` crate:
- Centralizes all schema generation, validation utilities, and the schema registry
- Avoids circular deps — depends on zen-core (types), consumed by zen-db/zen-hooks/zen-cli
- Isolates `schemars` + `jsonschema` from crates that don't need them
- Provides a single `SchemaRegistry` that loads once at startup

---

## 2. Background & Prior Art

### Current JSON Schema State in Zenith

| Location | What Exists |
|----------|-------------|
| `zen-hooks/src/spike_git_hooks.rs` (lines 589-743) | Hand-written Draft-07 schema for JSONL trail operations. Uses `serde_json::json!({...})` to construct schema inline. Validates: required top-level fields (`ts`, `ses`, `op`, `entity`), enum values for `op` field. Does NOT validate the `data` field's per-entity shape. |
| `Cargo.toml` workspace | `jsonschema = "0.28"` declared, consumed only by zen-hooks |
| Everywhere else | No schema validation. Serde deserialization is the only guard. |

### Spike 0.13 Findings on `jsonschema`

From the git hooks spike (11-git-hooks-spike-plan.md):
- `jsonschema` 0.28 `validator_for()` returns `Err(ValidationError)` (single) — use `iter_errors()` for all errors
- Rich error messages: `"INVALID_OP" is not one of ["create","update",...]`, `[1,2,3] is not of type "object"`
- Draft auto-detection works — `jsonschema` accepts both Draft-07 and Draft 2020-12
- Performance: validation of individual JSONL lines is sub-microsecond

### `schemars` 1.x Capabilities

From research:
- `#[derive(JsonSchema)]` respects all `#[serde(...)]` attributes: `rename`, `rename_all`, `tag`/`content`/`untagged`, `default`, `skip`, etc.
- Generates Draft 2020-12 by default (configurable via `SchemaSettings`)
- Feature flags: `chrono04` for `DateTime<Utc>`, `url2`, `uuid1`, `semver1`, etc.
- `serde_json::Value` produces `{}` (any JSON) — expected, handled via per-entity dispatch
- Nested structs, `Option<T>`, `Vec<T>`, `HashMap<K,V>` all supported
- `schema_for!(T)` macro for quick generation; `SchemaGenerator` for batched generation

---

## 3. The 10 Integration Points

### Critical Boundaries (data corruption risk)

| # | Boundary | Phase | Risk |
|---|----------|-------|------|
| 1 | **JSONL trail operation envelope** — top-level fields (`ts`, `ses`, `op`, `entity`, `id`, `data`) | 1, 2, 5 | Missing/malformed fields produce unrebuildable trail files |
| 2 | **JSONL trail `data` sub-schemas** — per-entity payload (finding data ≠ hypothesis data) | 1, 2 | Wrong entity data silently produces corrupt DB on rebuild |
| 3 | **Audit trail `detail` field** — polymorphic per-action JSON (`status_changed` needs `{from,to}`, `linked` needs `{source_type,...}`) | 2 | Silent data loss on replay, incorrect audit queries |

### High-Value Boundaries (contract enforcement)

| # | Boundary | Phase | Risk |
|---|----------|-------|------|
| 4 | **CLI JSON output** — 37+ documented JSON shapes in 04-cli-api-design.md | 5 | Drift between docs and implementation; LLM consumers get unexpected shapes |
| 5 | **CLI JSON input** — `--tasks` JSON array in PRD workflow | 5, 6 | Malformed input from LLMs silently accepted or panics |
| 6 | **Config schema** — `ZenConfig` with nested Turso/MotherDuck/R2/General sections | 1 | Invalid config silently ignored by figment; no editor autocompletion |

### Medium-Value Boundaries (structured metadata)

| # | Boundary | Phase | Risk |
|---|----------|-------|------|
| 7 | **DuckDB `metadata JSON` column** — per-language extras (Rust lifetimes, Python decorators, TS type params) | 3 | Freeform JSON queried via `->>`; wrong shape = silent null returns |
| 8 | **DuckDB `attributes TEXT` column** — JSON array stored as TEXT | 3 | Minor — simple `Vec<String>`, serde handles it |

### Lower-Value Boundaries

| # | Boundary | Phase | Risk |
|---|----------|-------|------|
| 9 | **Registry API responses** — crates.io, npm, PyPI, hex.pm | 4 | Serde already validates; schemas useful for fixture validation |
| 10 | **AgentFS KV store** — `SessionMeta` and other serde structs | 7 | AgentFS controls serialization; low risk |

---

## 4. New Crate: zen-schema

### Architecture

```
zen-core (entity structs + enums with #[derive(JsonSchema)])
  │
  └─► zen-schema (SchemaRegistry, validate(), export())
        │
        ├─► zen-db (trail validation, audit validation, rebuild --strict)
        ├─► zen-hooks (pre-commit JSONL validation)
        ├─► zen-lake (DuckDB metadata validation, Phase 3)
        └─► zen-cli (znt schema command, response schemas)
```

### Module Structure (Production, Post-Spike)

```
zen-schema/src/
├── lib.rs              # Re-exports, SchemaRegistry
├── registry.rs         # SchemaRegistry: stores schemas, provides get/validate/export
├── trail.rs            # TrailOperation schema, per-entity data sub-schemas
├── audit.rs            # Per-action audit detail schemas
├── config.rs           # ZenConfig schema generation + export
├── responses.rs        # CLI response struct schemas (Phase 5)
└── metadata.rs         # DuckDB per-language metadata schemas (Phase 3)
```

### Key Type: SchemaRegistry

```rust
/// Central registry of all JSON Schemas in the system.
/// Initialized once at startup, used by trail writer, audit repo,
/// pre-commit hook, and `znt schema` command.
pub struct SchemaRegistry {
    schemas: HashMap<String, serde_json::Value>,
}

impl SchemaRegistry {
    /// Build registry with all entity, trail, config, and response schemas.
    pub fn new() -> Self { ... }

    /// Get a schema by name (e.g., "finding", "trail_operation", "config").
    pub fn get(&self, name: &str) -> Option<&serde_json::Value> { ... }

    /// Validate a JSON value against a named schema. Returns all errors.
    pub fn validate(&self, name: &str, value: &serde_json::Value) -> Result<(), Vec<String>> { ... }

    /// Export all schemas to a directory as individual .schema.json files.
    pub fn export_all(&self, dir: &Path) -> Result<()> { ... }

    /// List all registered schema names.
    pub fn names(&self) -> Vec<&str> { ... }
}
```

---

## 5. Dependencies

### New Workspace Dependency

```toml
# In workspace Cargo.toml [workspace.dependencies]:
schemars = { version = "1.2", features = ["chrono04"] }
```

**Why `chrono04`**: All entity structs use `DateTime<Utc>` from chrono 0.4. Without this flag, `#[derive(JsonSchema)]` would fail to compile on timestamp fields.

### zen-schema Cargo.toml

```toml
[package]
name = "zen-schema"
description = "JSON Schema generation, validation, and registry for Zenith"
version.workspace = true
edition.workspace = true
license.workspace = true

[dependencies]
zen-core.workspace = true
schemars.workspace = true
jsonschema.workspace = true
serde.workspace = true
serde_json.workspace = true
thiserror.workspace = true

[dev-dependencies]
pretty_assertions.workspace = true
chrono.workspace = true

[lints]
workspace = true
```

### zen-core Addition (Post-Spike, Phase 1)

```toml
# zen-core gains schemars as a dependency:
schemars.workspace = true
```

This is NOT added during the spike. The spike defines sample types locally. Phase 1 adds `#[derive(JsonSchema)]` to the real entity structs in zen-core.

---

## 6. Spike Tests

**File**: `zenith/crates/zen-schema/src/spike_schema_gen.rs`

### Part A: Entity Schema Generation (5 tests)

| # | Test | What It Validates |
|---|------|-------------------|
| 1 | `spike_schema_entity_basic` | Define sample `Finding`, `Hypothesis`, `Issue`, `Task` structs matching `05-crate-designs.md` with `#[derive(Serialize, Deserialize, JsonSchema)]` and `#[serde(rename_all = "snake_case")]` enums. Generate schemas with `schema_for!()`. Verify: fields match serde representation, `Option<T>` produces nullable types, `DateTime<Utc>` with `chrono04` produces correct `"format": "date-time"`, enum values are snake_case strings. |
| 2 | `spike_schema_entity_all_twelve` | Define all 12 entity types (session, research, finding, hypothesis, insight, issue, task, impl_log, compat, study, link, audit). Generate schemas for each. Meta-validate every schema is itself valid JSON Schema via `jsonschema` draft detection. Count total fields across all schemas. |
| 3 | `spike_schema_entity_roundtrip` | For each of the 12 entities: create a representative instance → serialize to JSON with `serde_json` → validate the JSON against the schemars-generated schema → deserialize back → assert equality. This proves schemars and serde agree on the format for every entity type. |
| 4 | `spike_schema_enum_constraints` | Generate schemas for all 8 status/type enums: `HypothesisStatus`, `IssueType`, `IssueStatus`, `TaskStatus`, `Confidence`, `AuditAction`, `EntityType`, `Relation`. Verify each produces `{"type":"string","enum":["value1","value2",...]}` with correct snake_case values matching 05-crate-designs.md. |
| 5 | `spike_schema_entity_validation_errors` | For each entity type, create invalid JSON: (a) wrong enum value (`"confidence": "very_high"`), (b) missing required field (no `content` on finding), (c) wrong type (`"priority": "high"` instead of integer on issue), (d) extra unknown field (verify behavior — strict or permissive). Verify `jsonschema` produces descriptive error messages for each case. |

### Part B: Trail Operation Schema (4 tests)

| # | Test | What It Validates |
|---|------|-------------------|
| 6 | `spike_schema_trail_envelope` | Define `TrailOperation` struct: `ts: String`, `ses: String`, `op: TrailOp` (create/update/delete), `entity: EntityType`, `id: String`, `data: serde_json::Value`. Add `#[derive(JsonSchema)]`. Generate schema. Compare field-by-field with the hand-written schema from spike 0.13 (`zen-hooks/src/spike_git_hooks.rs` lines 595-636). Document: equivalent fields, missing fields, extra fields, type differences. |
| 7 | `spike_schema_trail_data_dispatch` | For each of the 13 entity types, generate a "create data" sub-schema from the entity's fields (excluding server-generated fields: `id`, `created_at`, `updated_at`). Implement dispatch: given `entity: "finding"`, look up the finding data schema and validate the `data` field. Test: (a) finding data with correct fields → passes, (b) hypothesis data submitted as finding entity → fails with descriptive error, (c) update operation with partial fields → behavior documented (update schemas are permissive subsets). |
| 8 | `spike_schema_trail_validation_matrix` | Replay the validation edge-case matrix from spike 0.13 test 9 using the schemars-generated trail schema: (a) valid operation → passes, (b) malformed JSON line → caught by `serde_json::from_str` before schema check, (c) BOM prefix `\xEF\xBB\xBF` → detected, (d) conflict markers `<<<<<<<` → detected, (e) missing required `ts` field → schema error, (f) invalid `op` enum value `"INVALID_OP"` → schema error, (g) missing `entity` field → schema error. All must produce clear error messages. |
| 9 | `spike_schema_trail_export` | Serialize the trail envelope schema and all 13 entity data sub-schemas to individual `.schema.json` files in a temp directory. Verify: each file is valid JSON, each file is loadable and re-validatable, total file count = 14 (1 envelope + 13 data schemas). |

### Part C: Config Schema (3 tests)

| # | Test | What It Validates |
|---|------|-------------------|
| 10 | `spike_schema_config_derive` | Define `ZenConfig`, `TursoConfig`, `MotherDuckConfig`, `R2Config`, `GeneralConfig` matching `05-crate-designs.md` (lines 462-520). Add `#[derive(JsonSchema)]`. Generate schema. Verify: nested structs produce `$ref` or inline object schemas, `Option<String>` produces nullable, `#[serde(default)]` is reflected (field becomes non-required), all sections appear as properties of root object. |
| 11 | `spike_schema_config_validate` | Create valid config JSON → passes. Create invalid: (a) wrong type for `sync_interval_secs` (string instead of integer) → fails, (b) completely unknown top-level section → document behavior (schemars may or may not add `additionalProperties: false`), (c) missing all sections (empty object) → should pass because all sections have `#[serde(default)]`. |
| 12 | `spike_schema_config_export` | Export config schema to `config.schema.json`. Verify well-formed. Document: could be shipped in `.zenith/` directory for editor TOML validation plugins. |

### Part D: CLI Response & Input Schemas (3 tests)

| # | Test | What It Validates |
|---|------|-------------------|
| 13 | `spike_schema_response_structs` | Define 5 representative response structs matching `04-cli-api-design.md`: `FindingCreateResponse` (lines 497-508), `SessionStartResponse` (lines 224-236), `WhatsNextResponse` (lines 1011-1040), `SearchResultsResponse` (lines 361-382), `RebuildResponse` (lines 982-989). All with `#[derive(Serialize, JsonSchema)]`. Generate schemas. Verify: nested entities produce correct sub-schemas, arrays produce `"type":"array"` with correct `items`, optional fields are nullable. |
| 14 | `spike_schema_response_validate` | Create valid response JSON for each struct → validate against schema → passes. Mutate each response (add wrong type, remove required field, add extra nested field) → fails with descriptive errors. |
| 15 | `spike_schema_input_validate` | Define input type for PRD `--tasks` flag: `TasksInput(Vec<String>)`. Generate schema. Validate: `["task1","task2"]` → passes, `[123,null]` → fails, `"not-array"` → fails, `[]` → passes (empty array is valid). Also test: a more complex input type with `TaskDefinition { title: String, description: Option<String> }` as array elements. |

### Part E: Audit Detail Schemas (2 tests)

| # | Test | What It Validates |
|---|------|-------------------|
| 16 | `spike_schema_audit_detail_types` | Define per-action detail types: `StatusChangedDetail { from: String, to: String, reason: Option<String> }`, `LinkedDetail { source_type: String, source_id: String, target_type: String, target_id: String, relation: String }`, `TaggedDetail { tag: String }`, `IndexedDetail { package: String, ecosystem: String, symbols: u32, duration_ms: u64 }`. Derive `JsonSchema` for each. Validate against concrete examples from `01-turso-data-model.md`. |
| 17 | `spike_schema_audit_detail_dispatch` | Implement dispatch function: given an `AuditAction` enum value, return the corresponding detail schema. Test: (a) `StatusChanged` → validate `{from: "open", to: "done"}` passes, (b) `StatusChanged` → validate `{tag: "verified"}` (wrong detail type) fails, (c) `Created` → validate any object passes (generic detail), (d) `Tagged` → validate `{tag: "verified"}` passes. |

### Part F: DuckDB Metadata Schemas (2 tests)

| # | Test | What It Validates |
|---|------|-------------------|
| 18 | `spike_schema_metadata_rust` | Define `RustMetadata` matching `02-ducklake-data-model.md` lines 231-250: `lifetimes: Option<Vec<String>>`, `where_clause: Option<String>`, `is_pyo3: bool`, `trait_name: Option<String>`, `for_type: Option<String>`, `variants: Option<Vec<String>>`, `fields: Option<Vec<String>>`, `methods: Option<Vec<String>>`, `associated_types: Option<Vec<String>>`, `abi: Option<String>`, `doc_sections: Option<RustDocSections>` where `RustDocSections { errors: Option<Vec<String>>, panics: Option<String>, safety: Option<String>, examples: Option<Vec<String>> }`. Derive `JsonSchema`. Validate against the inline doc example. Test: nested `Option<Vec<String>>` produces correct nullable array schema, nested optional struct produces correct schema. |
| 19 | `spike_schema_metadata_python_ts` | Define `PythonMetadata` matching lines 254-269: `is_generator: bool`, `is_property: bool`, `is_pydantic: bool`, `is_protocol: bool`, `is_dataclass: bool`, `base_classes: Option<Vec<String>>`, `decorators: Option<Vec<String>>`, `parameters: Option<Vec<String>>`, `doc_sections: Option<PythonDocSections>` where `PythonDocSections { args: Option<HashMap<String,String>>, returns: Option<String>, raises: Option<HashMap<String,String>> }`. And `TypeScriptMetadata` matching lines 273-279: `is_exported: bool`, `is_default_export: bool`, `type_parameters: Option<Vec<String>>`, `implements: Option<Vec<String>>`. Derive schemas. Verify: `HashMap<String,String>` produces `{"type":"object","additionalProperties":{"type":"string"}}`. Validate against doc examples. |

### Part G: Schema Registry & Cross-Cutting (3 tests)

| # | Test | What It Validates |
|---|------|-------------------|
| 20 | `spike_schema_draft_compat` | Generate a schema with schemars (defaults to Draft 2020-12). Validate JSON data against it using `jsonschema` 0.28 with auto-detection. Repeat with explicit Draft 7 via schemars `SchemaSettings::draft07()`. Both must work. Document: which draft schemars generates, which drafts jsonschema accepts, any incompatibilities. |
| 21 | `spike_schema_registry` | Implement prototype `SchemaRegistry`: register all entity schemas (12), trail envelope (1), entity data sub-schemas (13), config (1), response schemas (5), audit detail schemas (4), metadata schemas (3). Total: 39 schemas. Verify: `get("finding")` returns the finding schema, `validate("finding", &valid_json)` passes, `validate("finding", &invalid_json)` returns error list, `names()` returns all 39 names, `export_all()` writes 39 `.schema.json` files. Measure: registry construction time (must be < 50ms). |
| 22 | `spike_schema_compare_handwritten` | Load the hand-written trail schema from spike 0.13. Generate the equivalent with schemars. Compare field-by-field: required fields, enum values, type constraints. Document all differences. Benchmark: validate 1000 JSONL operations against each schema, compare times. Recommend: keep schemars-generated or hand-written or both. |

**Total: 22 tests**

---

## 7. Evaluation Criteria

| Criterion | Weight | How We Measure |
|-----------|--------|---------------|
| schemars + serde agreement | **Critical** | Entity roundtrip tests (test 3): every entity serializes/validates/deserializes correctly |
| DateTime<Utc> handling | **Critical** | Test 1: `chrono04` feature produces `"format":"date-time"` |
| Enum `rename_all` handling | **Critical** | Test 4: all 8 enums produce correct snake_case string values |
| Per-entity data dispatch | **High** | Test 7: correct entity data passes, wrong entity data fails |
| Nested optional structs | **High** | Tests 18-19: `Option<Vec<String>>`, `Option<HashMap<K,V>>`, nested `Option<Struct>` |
| HashMap schema generation | **High** | Test 19: `HashMap<String,String>` → `additionalProperties` |
| jsonschema draft compatibility | **High** | Test 20: schemars 2020-12 + jsonschema auto-detect both work |
| Schema registry performance | **Medium** | Test 21: construction < 50ms, validation < 1μs per item |
| Error message quality | **Medium** | Tests 5, 8: descriptive errors for wrong enum, missing field, wrong type |
| Export to .schema.json | **Medium** | Tests 9, 12: files are valid, loadable, re-validatable |
| schemars compile time | **Medium** | Measure delta: `cargo build -p zen-schema` with and without schemars |

---

## 8. What This Spike Does NOT Test

- **Runtime wiring** — actual integration into zen-db trail writer, zen-hooks pre-commit, or zen-cli is Phase 1-5 work. The spike only proves the schema generation and validation pipeline works in isolation.
- **SQL-generated JSON** — `json_group_array(json_object(...))` in SQLite produces JSON strings that aren't validated by schema. This is a future concern (would need to deserialize SQL output into typed structs first).
- **Parquet/Lance schema alignment** — DuckDB table schemas and JSON Schemas are different systems. Alignment is a Phase 3 stretch goal.
- **Schema versioning/migration** — what happens when entity structs change. Future concern — JSONL trail files may reference older schema versions.
- **`additionalProperties: false`** — schemars may or may not set this by default. The spike documents the behavior but doesn't force a decision.
- **Schema publishing** — hosting schemas at a URL for external consumers. Out of scope.

---

## 9. Success Criteria

- `schemars` 1.x compiles with `chrono04` feature in Rust 2024 edition
- `#[derive(JsonSchema)]` works on all entity structs with existing serde attributes (`rename_all`, `Option`, `DateTime<Utc>`, `serde_json::Value`)
- Generated schemas match serde serialization for all 12 entity types (validated via roundtrip)
- `jsonschema` 0.28 accepts schemars-generated schemas (Draft 2020-12 cross-crate compatibility confirmed)
- Per-entity data dispatch works: correct entity data passes, wrong entity data for a given entity type fails
- HashMap types produce correct `additionalProperties` schemas (for Python/TS metadata)
- Nested optional structs produce correct nullable sub-schemas (for Rust metadata `doc_sections`)
- Trail envelope schema matches or supersedes the hand-written spike 0.13 schema
- SchemaRegistry prototype loads 39 schemas in < 50ms
- All 22 tests pass
- schemars compile time documented

---

## 10. Post-Spike Actions

### Regardless of Outcome

1. Add `zen-schema` crate to Cargo workspace (11th crate)
2. Add `schemars = { version = "1.2", features = ["chrono04"] }` to workspace deps
3. Update `05-crate-designs.md`: add zen-schema crate design section
4. Update `07-implementation-plan.md`:
   - Add spike 0.14 to Phase 0 with results
   - Add zen-schema to dependency graph (sits alongside zen-core in Phase 1)
   - Update Phase 1 tasks: entity structs get `#[derive(JsonSchema)]`
   - Update Phase 2 tasks: trail writer and audit repo reference zen-schema for validation
   - Update Phase 3 tasks: metadata types get `#[derive(JsonSchema)]`
   - Update Phase 5: add `znt schema <type>` command task, update task 5.18b to use generated schema
5. Update workspace `Cargo.toml`: add `schemars`, `zen-schema` crate member
6. Update `INDEX.md`: add doc 12 to document map, add zen-schema to crate list

### If Spike Passes (Expected Path)

1. **Phase 1 (task 1.1-1.2)**: Add `schemars.workspace = true` to zen-core deps. Add `#[derive(JsonSchema)]` to all entity structs and enums. Define `TrailOperation` and per-action audit detail types with `JsonSchema`.
2. **Phase 1 (task 1.5)**: Add `#[derive(JsonSchema)]` to config structs. `zen init` exports `config.schema.json` to `.zenith/`.
3. **Phase 2 (task 2.12)**: Validate audit `detail` payloads against per-action schemas on write.
4. **Phase 2 (task 2.15)**: Trail writer validates `Operation.data` against per-entity data schema before appending (configurable: on by default, `--no-validate` to skip).
5. **Phase 2 (task 2.16)**: Replayer validates each trail line when `--strict` flag is passed on `znt rebuild`.
6. **Phase 3 (task 3.2)**: Define `RustMetadata`, `PythonMetadata`, `TypeScriptMetadata` with `#[derive(JsonSchema)]` in zen-parser or zen-schema.
7. **Phase 5 (task 5.18b)**: Pre-commit hook imports trail schema from zen-schema (replaces hand-written schema from spike 0.13 in zen-hooks).
8. **Phase 5 (new task)**: Implement `znt schema <type>` command — dumps JSON Schema for any registered type. Uses `SchemaRegistry.get()` + pretty print.
9. Update `11-git-hooks-spike-plan.md`: note that the hand-written trail schema is superseded by the schemars-generated one from zen-schema.

### If schemars Doesn't Work (Fallback)

If schemars fails on critical types (DateTime, nested optionals, HashMaps):
1. Keep `jsonschema` 0.28 (already works)
2. Hand-write schemas for the critical boundaries only (trail operations, audit details)
3. Store schemas as `.json` files in zen-schema, loaded at runtime
4. Skip CLI response and config schemas (lower priority)
5. Re-evaluate schemars when a new version ships

### Risk Register Additions

| Risk | Impact | Likelihood | Mitigation |
|------|--------|-----------|------------|
| `schemars` `chrono04` doesn't support `DateTime<Utc>` format correctly | Timestamp fields produce wrong schema, roundtrip validation fails | Low (docs confirm chrono04) | Spike test 1 validates directly |
| schemars generates Draft 2020-12 but jsonschema 0.28 can't validate it | Cross-crate incompatibility | Low (jsonschema auto-detects) | Spike test 20 validates directly |
| `serde_json::Value` produces unhelpfully permissive `{}` schema | Trail `data` field validation is useless from envelope schema alone | Expected | Per-entity data sub-schemas (test 7) provide the real validation — dispatch by `entity` field |
| schemars adds significant compile time to zen-core | Slower workspace builds | Medium | Measure in spike. Proc-macro cost is per-derive, not per-crate. zen-schema isolates the heavy validation logic. |
| Schema versioning: old trail files reference outdated entity shapes | Rebuild fails on schema-validated replay of old trail data | Low (Phase 0-2 only) | `--strict` is opt-in for rebuild. Non-strict mode skips schema validation. Versioning is a Phase 8+ concern. |
| `additionalProperties` behavior differs between schemars and hand-written schemas | Validation strictness mismatch | Medium | Test 22 compares hand-written vs generated. Document and decide on convention. |

---

## Cross-References

- Turso data model (entity field definitions): [01-turso-data-model.md](./01-turso-data-model.md)
- DuckLake data model (metadata JSON specs): [02-ducklake-data-model.md](./02-ducklake-data-model.md)
- CLI API design (JSON output shapes): [04-cli-api-design.md](./04-cli-api-design.md)
- Crate designs (entity struct definitions): [05-crate-designs.md](./05-crate-designs.md)
- Implementation plan: [07-implementation-plan.md](./07-implementation-plan.md)
- Git & JSONL strategy (trail format): [10-git-jsonl-strategy.md](./10-git-jsonl-strategy.md)
- Git hooks spike (hand-written trail schema, jsonschema usage): [11-git-hooks-spike-plan.md](./11-git-hooks-spike-plan.md)
