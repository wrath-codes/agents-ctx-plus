# Zenith: JSONL Trail Schema Versioning -- Spike Plan

**Version**: 2026-02-08
**Status**: DONE -- 10/10 tests pass
**Purpose**: Validate the "Hybrid" versioning strategy (Approach D) for JSONL trail schema evolution using only existing crates (`serde`, `schemars`, `jsonschema`, `serde-jsonlines`). Prove that old trail files remain readable after entity shape changes, that version-dispatch replay works, and that the `additionalProperties` convention is correct.
**Spike ID**: 0.16
**Crate**: zen-schema (spike file alongside existing spike_schema_gen.rs)
**Blocks**: Phase 2 (tasks 2.15-2.17 trail writer/replayer/versioning), Phase 5 (pre-commit schema swap to schemars-generated)

---

## Table of Contents

1. [Motivation](#1-motivation)
2. [Background & Prior Art](#2-background--prior-art)
3. [Approach D: Hybrid Strategy](#3-approach-d-hybrid-strategy)
4. [What We're Validating](#4-what-were-validating)
5. [Dependencies](#5-dependencies)
6. [Spike Tests](#6-spike-tests)
7. [Evaluation Criteria](#7-evaluation-criteria)
8. [What This Spike Does NOT Test](#8-what-this-spike-does-not-test)
9. [Success Criteria](#9-success-criteria)
10. [Post-Spike Actions](#10-post-spike-actions)

---

## 1. Motivation

Zenith's JSONL trail is the source of truth (spike 0.12 decision). The SQLite database is derived and rebuildable from trail files via `znt rebuild`. As we build Phases 1-5, entity shapes will change -- new fields, renamed fields, possibly type changes. Without a versioning strategy validated in code, the first entity change will either:

- **Silently produce corrupt databases** (replay uses `op.data["field"].as_str().unwrap_or("")` -- missing fields become empty strings)
- **Break replay entirely** (required field missing causes deserialization error, rebuild fails)

### What Beads Does

Beads (our closest reference) takes the simplest possible approach: **no explicit versioning**. Its JSONL layer has an optional `# {"version": "1.0"}` header comment that is never used. Recovery is always "delete DB, rebuild from JSONL" with the *current* code. Beads gets away with this because its domain is narrow (issues + labels + dependencies + comments) and format changes are rare.

### Why Zenith Can't Copy Beads

Zenith has 15+ entity types with complex fields (nested JSON, typed enums, `DateTime<Utc>`, entity_links, studies, compatibility checks). The `data: serde_json::Value` payload varies per entity type. Development velocity during Phases 1-5 makes format changes likely, not rare.

---

## 2. Background & Prior Art

### Industry Patterns (from research)

| Pattern | Used by | Approach |
|---------|---------|----------|
| Additive-only + defaults | Beads, most NoSQL DBs | Never remove fields, only add with defaults |
| Version field in envelope | Kafka Schema Registry, Cosmos DB | `schema_version` field, dispatch by version |
| Tagged version enum | `serde-evolve`, `pro-serde-versioned` | Separate V1/V2 structs, compile-time migration chains |
| Compatibility modes | Avro, Protobuf, Confluent | BACKWARD / FORWARD / FULL compatibility rules |

### Crate Options Evaluated

| Crate | Version | Approach | Verdict |
|-------|---------|----------|---------|
| `serde-evolve` | 0.1.0 (Oct 2025) | `#[derive(Versioned)]` enum with `_version` tag, `From` chain | Too new (0.1), requires separate V1/V2 structs per entity, `_version` tag conflicts with our `data: Value` design |
| `serde-flow` | 1.1.1 | `#[derive(Flow)]` with `#[flow(variant = N)]` | Binary-oriented (uses u16 flow_id prefix), not suited for human-readable JSONL |
| `pro-serde-versioned` | 1.0.2 | `VersionedSerialize`/`VersionedDeserialize` traits | Wraps in `VersionedEnvelope`, adds overhead per operation |
| **No new crate** | -- | `#[serde(default)]` + envelope `v` field + manual dispatch | Works with existing deps, matches our JSON-lines design, zero new risk |

**Decision**: No new crate. Use serde's built-in `default`, `alias`, and `deny_unknown_fields` attributes combined with `schemars` + `jsonschema` for validation.

---

## 3. Approach D: Hybrid Strategy

**Default**: Additive-only evolution with `#[serde(default)]` for all new fields.
**Escape hatch**: When a breaking change is unavoidable, bump the envelope version.

### Envelope Change

```rust
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
struct TrailOperation {
    #[serde(default = "default_trail_version")]
    v: u32,       // NEW -- defaults to 1 for old trails without this field
    ts: String,
    ses: String,
    op: TrailOp,
    entity: EntityType,
    id: String,
    data: serde_json::Value,
}

fn default_trail_version() -> u32 { 1 }
```

### Evolution Rules

| Change type | Allowed without version bump? | Mechanism |
|---|---|---|
| Add optional field | Yes | `new_field: Option<T>` deserializes as `None` from old data |
| Add field with default | Yes | `#[serde(default)]` or `#[serde(default = "fn")]` |
| Remove field | No -- deprecate only | Stop writing, keep in struct with `#[serde(default)]` |
| Rename field | No -- alias | `#[serde(alias = "old_name")]` on new field |
| Change field type | No -- bump version | Replayer migrates v1 `Value` to v2 shape |
| Make optional -> required | No -- bump version | Replayer fills default during migration |

### Replay Dispatch

```rust
fn replay_operation(op: TrailOperation, db: &ZenDb) -> Result<()> {
    let op = match op.v {
        1 => op,
        // Future: 2 => migrate_v1_to_v2(op),
        v => return Err(ZenError::UnsupportedTrailVersion(v)),
    };
    apply_to_db(op, db)
}
```

### `additionalProperties` Convention

| Context | Setting | Rationale |
|---------|---------|-----------|
| Trail operations | `true` (permissive) | Forward-compatible: old writers don't break new readers |
| Config (figment) | `false` (strict) | Catches typos (confirmed figment gotcha from zen-config spike) |
| CLI output | `true` (permissive) | LLM consumers shouldn't break when we add response fields |

---

## 4. What We're Validating

7 hypotheses that must hold for Approach D to work:

| # | Hypothesis | Risk if wrong |
|---|---|---|
| H1 | `#[serde(default = "fn")]` on `v: u32` allows old trail lines (no `v` field) to deserialize as v1 | Old trails unreadable after adding version field |
| H2 | `schemars` schema for struct with `#[serde(default)]` fields works correctly with `jsonschema` validation when those fields are absent | Schema validation rejects old trails |
| H3 | Adding `Option<String>` to an entity is fully backward-compatible through JSONL write/read roundtrip | Additive evolution (the 90% case) breaks |
| H4 | `#[serde(alias = "old_name")]` allows old trail lines with old field name to deserialize | Field renames require version bump |
| H5 | Version-dispatch migration on `serde_json::Value` works (transform v1 data shape to v2 before applying) | Breaking changes have no migration path |
| H6 | `additionalProperties: true` (schemars default) passes validation for data with fewer fields than schema; `deny_unknown_fields` + explicit `additionalProperties: false` rejects unknown fields | Convention doesn't work as expected |
| H7 | `serde-jsonlines` roundtrip preserves the `v` field and all versioning behavior | JSONL serialization layer interferes with versioning |

---

## 5. Dependencies

No new crates. All already in zen-schema or workspace:

| Crate | Version | Role in spike |
|-------|---------|--------------|
| `serde` | workspace | `#[serde(default)]`, `#[serde(alias)]`, `#[serde(deny_unknown_fields)]` |
| `schemars` | workspace | `#[derive(JsonSchema)]`, schema generation |
| `jsonschema` | workspace | Runtime validation |
| `serde_json` | workspace | `Value` manipulation, JSON parsing |
| `serde-jsonlines` | workspace (dev) | JSONL roundtrip tests |
| `tempfile` | workspace (dev) | Temp files for JSONL tests |
| `chrono` | workspace (dev) | `DateTime<Utc>` in entity structs |

---

## 6. Spike Tests

10 tests across 4 sections in `zen-schema/src/spike_trail_versioning.rs`:

### Part A: Envelope Versioning (3 tests)

| # | Test | Validates |
|---|------|-----------|
| A1 | `spike_v_field_defaults_to_1_when_absent` | Deserialize old trail JSON (no `v` field) -> `v == 1` (H1) |
| A2 | `spike_v_field_preserved_when_present` | Deserialize trail JSON with `"v": 2` -> `v == 2` |
| A3 | `spike_schema_validates_with_and_without_v` | Generate schema from `TrailOperation`, validate JSON both with and without `v` field. Both must pass. (H2) |

### Part B: Additive Evolution (3 tests)

| # | Test | Validates |
|---|------|-----------|
| B1 | `spike_new_optional_field_backward_compat` | Old `Finding` JSON (no `source_url`) deserializes into new struct with `source_url: Option<String>` = `None` (H3) |
| B2 | `spike_new_default_field_backward_compat` | Old entity JSON (no `methodology`) deserializes into struct with `#[serde(default)]` `methodology: String` = `""` (H3) |
| B3 | `spike_alias_field_rename_backward_compat` | Old JSON with `"package_a"` deserializes into struct with `#[serde(alias = "package_a")] pkg_a: String` (H4) |

### Part C: Version-Dispatch Migration (2 tests)

| # | Test | Validates |
|---|------|-----------|
| C1 | `spike_v1_to_v2_value_migration` | Transform v1 `Finding` data (`confidence: "high"` as String) to v2 shape (`confidence: {"level": "high", "basis": "unknown"}`). Validate migrated data against v2 schema. (H5) |
| C2 | `spike_replay_dispatch_routes_by_version` | Build vec of mixed v1+v2 operations, dispatch each, confirm correct migration applied and all produce valid v2 data. (H5) |

### Part D: additionalProperties Convention + JSONL Roundtrip (2 tests)

| # | Test | Validates |
|---|------|-----------|
| D1 | `spike_additional_properties_convention` | Trail schema (permissive) accepts unknown fields. Config schema with `deny_unknown_fields` rejects unknown fields. (H6) |
| D2 | `spike_jsonlines_roundtrip_preserves_version` | Write versioned `TrailOperation` to JSONL via `serde-jsonlines`, read back, confirm `v` field and all data preserved. Also write old-format (no `v`), read back, confirm `v` defaults to 1. (H7) |

---

## 7. Evaluation Criteria

| Criterion | Pass | Fail |
|-----------|------|------|
| All 10 tests pass | Approach D validated | Re-evaluate, consider `serde-evolve` |
| Old trails (no `v`) deserialize correctly | H1 confirmed | Versioning strategy blocked |
| Schema validation accepts missing `v` and missing optional fields | H2, H3 confirmed | Schema validation too strict for evolution |
| `serde(alias)` works for renames | H4 confirmed | Renames require version bump (acceptable fallback) |
| Value migration composes correctly | H5 confirmed | Migration approach needs redesign |
| `additionalProperties` convention works | H6 confirmed | Need explicit schemars config |
| `serde-jsonlines` roundtrip clean | H7 confirmed | Need custom JSONL handling |

---

## 8. What This Spike Does NOT Test

- **Actual rebuild performance** with version checks (deferred to Phase 2 measurement)
- **Multiple sequential version bumps** (v1 -> v2 -> v3 chain) -- first version bump hasn't happened yet
- **Concurrent JSONL writes** with versioned operations -- already validated in spike 0.12
- **Real entity struct changes** -- spike uses test-local structs, not zen-core types (those don't exist yet)
- **JSONL compaction** -- still deferred, orthogonal to versioning

---

## 9. Success Criteria

- **10/10 tests pass**
- **No new crate dependencies** required
- **Clear documentation** of evolution rules, `additionalProperties` convention, and migration pattern
- **Gotchas documented** (any serde/schemars/jsonschema interaction surprises)

---

## 10. Post-Spike Actions

If spike passes:

| Doc | Update |
|-----|--------|
| `07-implementation-plan.md` | Add spike 0.16 to Phase 0 table (DONE), update task 2.17 description with validated approach |
| `10-git-jsonl-strategy.md` | Add versioning section: envelope `v` field, evolution rules, migration dispatch |
| `12-schema-spike-plan.md` | Update "What This Spike Does NOT Test" -- versioning is now tested |
| `05-crate-designs.md` | Add `additionalProperties` convention to zen-schema design section |

If hypothesis H4 (alias) fails:
- Field renames become version-bump-required. Update evolution rules table.

If hypothesis H6 (additionalProperties) fails:
- Need explicit schemars `SchemaSettings` configuration. Document the workaround.

---

## 11. Results

**10/10 tests pass. Approach D (Hybrid) fully validated. No new crate dependencies required.**

### Hypothesis Results

| # | Hypothesis | Result | Detail |
|---|---|---|---|
| H1 | `#[serde(default = "fn")]` on `v: u32` allows old trails to deserialize as v1 | **CONFIRMED** | Old trail JSON without `v` field deserializes with `v == 1`. |
| H2 | schemars schema + jsonschema validation accepts JSON without `v` field | **CONFIRMED** | schemars respects `#[serde(default)]` and does NOT include `v` in the `required` array. Schema `required` = `["ts", "ses", "op", "entity", "id", "data"]`. Validation passes without `v`. |
| H3 | Adding `Option<String>` and `#[serde(default)]` fields is backward-compatible | **CONFIRMED** | Both `Option<T>` (deserializes as `None`) and `#[serde(default)]` (deserializes as default) work. schemars does NOT include `#[serde(default)]` fields in `required`, so schema validation also passes. |
| H4 | `#[serde(alias)]` allows old field names to deserialize | **CONFIRMED with caveat** | Serde deserialization works perfectly (old `"package_a"` deserializes into `pkg_a` field). **However**: schemars schema uses the Rust field name (`pkg_a`), NOT the alias. Schema validation rejects old field names. **Implication**: field renames via alias are serde-compatible but NOT schema-validation-compatible. Pre-commit hook and `--strict` rebuild must not validate field names for aliased fields. |
| H5 | Version-dispatch migration on `serde_json::Value` works | **CONFIRMED** | Transform v1 `Value` data to v2 shape, validate against v2 schema, dispatch by `op.v` all work cleanly. Unsupported versions produce clear errors. |
| H6 | `additionalProperties` convention works | **CONFIRMED** | Trail schema (default schemars): `additionalProperties` absent from schema (JSON Schema treats absent as `true`), unknown fields accepted. Config schema (`#[serde(deny_unknown_fields)]`): schemars generates `"additionalProperties": false`, unknown fields rejected. Convention works exactly as designed. |
| H7 | `serde-jsonlines` roundtrip preserves version field | **CONFIRMED** | Write with `v`, read back with `v`. Write old-format (no `v`), read back with `v == 1` via default. Append new v2 operations to old files, read mixed file -- all version fields correct. |

### Key Findings

1. **schemars respects `#[serde(default)]`**: Fields with `#[serde(default)]` are NOT added to the schema's `required` array. This is critical -- it means additive evolution with defaults works end-to-end through both serde deserialization AND schema validation.

2. **schemars + `deny_unknown_fields` generates `additionalProperties: false`**: This confirms our convention: use `deny_unknown_fields` on config types (strict), leave it off trail types (permissive). No explicit schemars configuration needed.

3. **`serde(alias)` is deserialization-only -- schemars doesn't know about aliases**: Schema properties use the Rust field name, not the alias. Old data with aliased field names passes serde but fails schema validation. **Decision**: Field renames via alias should be treated as "serde-safe but schema-unsafe." Validation in pre-commit and rebuild should either skip renamed fields or use serde round-trip validation instead of schema validation for aliased types.

4. **`additionalProperties` is absent (not `true`) in default schemars output**: JSON Schema spec treats absent `additionalProperties` as `true`, so it works. But if we ever need to be explicit, we'd need to configure schemars.

5. **No `chrono` needed in this spike**: `DateTime<Utc>` wasn't needed for versioning tests since `ts` is a `String` in the trail envelope (matching the actual spike 0.12 design).

### Gotchas

| Gotcha | Severity | Mitigation |
|--------|----------|------------|
| `serde(alias)` not reflected in schemars schema | Medium | Don't rely on schema validation for aliased fields. Serde deserialization is the primary correctness check. Schema validation is a bonus layer, not the authority. |
| `additionalProperties` absent vs explicit `true` | Low | Works today. If JSON Schema tooling requires explicit `true`, use schemars `SchemaSettings`. |
| serde-jsonlines requires each JSON object on its own line (no multi-line JSON) | Low | Already known from spike 0.12. JSONL format is one JSON object per line by definition. |

### Decisions Confirmed

| Decision | Rationale |
|----------|-----------|
| **Approach D (Hybrid)** adopted | All 7 hypotheses confirmed. No new crates needed. |
| **`additionalProperties` convention**: trail=permissive, config=strict | Validated in test D1. schemars generates the right schemas automatically. |
| **`v: u32` with `#[serde(default)]` in TrailOperation** | Old trails deserialize cleanly, schema validation passes, JSONL roundtrip works. |
| **Field renames via `#[serde(alias)]`** are serde-safe | But schema-unsafe. Acceptable: schema validation is opt-in (`--strict`), serde is the primary deserializer. |
| **Value migration (v1 -> v2)** via `serde_json::Value` transform | Works cleanly. Transform data in-place, validate against target schema. |
