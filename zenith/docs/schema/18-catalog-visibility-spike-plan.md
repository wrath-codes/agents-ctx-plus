# Zenith: Turso Catalog + Clerk Visibility Scoping -- Spike Plan

**Version**: 2026-02-08
**Status**: DONE -- 9/9 tests pass
**Purpose**: Validate Turso as the global `indexed_packages` catalog with Clerk JWT-driven visibility scoping (public/team/private). Validates embedded replicas for local catalog access, three-tier search federation, and operational concerns (concurrent writes, multiple replicas).
**Spike ID**: 0.20
**Crate**: zen-db (spike file, reuses existing libsql + clerk-rs infrastructure from spikes 0.3/0.17)
**Blocks**: Phase 9 (crowdsourced global index, team visibility, private code indexing)

---

## Table of Contents

1. [Motivation](#1-motivation)
2. [Architecture Context](#2-architecture-context)
3. [What We're Validating](#3-what-were-validating)
4. [Dependencies](#4-dependencies)
5. [Spike Tests](#5-spike-tests)
6. [Evaluation Criteria](#6-evaluation-criteria)
7. [What This Spike Does NOT Test](#7-what-this-spike-does-not-test)
8. [Success Criteria](#8-success-criteria)
9. [Post-Spike Actions](#9-post-spike-actions)

---

## 1. Motivation

Spikes 0.17-0.19 validated:
- Clerk JWT auth + Turso JWKS (spike 0.17 — 14/14 pass)
- Lance on R2 for search (spike 0.18 — 18/18 pass)
- Native lancedb writes + serde_arrow production path (spike 0.19 — 10/10 pass)

What remains unvalidated is the **catalog layer** — how users discover what's indexed, how visibility is enforced, and how the global/team/private index tiers connect.

The architecture decision (from this session) is:
- **Turso Cloud** holds the `indexed_packages` catalog with visibility scoping
- **Every user** gets an embedded replica of the global catalog
- **Clerk JWT claims** (`sub`, `org.id`, `org.role`) drive visibility — no custom RBAC
- **DuckDB** is read-only, querying Lance datasets whose paths come from the Turso catalog

This spike validates the Turso + Clerk + Lance integration end-to-end.

---

## 2. Architecture Context

### Three-Tier Index Model

```
┌───────────────────────────────────────────────────────┐
│ Turso Cloud: zenith_global                             │
│                                                        │
│  indexed_packages (catalog)                            │
│    ├── visibility = 'public'  → anyone can discover    │
│    ├── visibility = 'team'    → org members only       │
│    └── visibility = 'private' → owner only             │
│                                                        │
│  Embedded replica on every authenticated user's machine│
└───────────────────────────────────────────────────────┘

┌───────────────────────────────────────────────────────┐
│ R2: Lance datasets                                     │
│                                                        │
│  s3://zenith/lance/{ecosystem}/{package}/{version}/    │
│  Access controlled by application — Turso catalog is   │
│  the discovery layer. If you can't find the path in    │
│  the catalog, you can't query the Lance dataset.       │
└───────────────────────────────────────────────────────┘

Search flow:
  1. Query Turso replica: SELECT r2_lance_path FROM indexed_packages
     WHERE (visibility = 'public'
         OR (visibility = 'team' AND team_id = <jwt.org.id>)
         OR (visibility = 'private' AND owner_id = <jwt.sub>))
     AND package IN (user's deps)

  2. DuckDB: lance_hybrid_search(r2_lance_paths, ...)
```

### Clerk JWT Claims Used

```json
{
  "sub": "user_39PB2iMuMcpYGrHobrukpqZ8UjE",
  "org_id": "org_xxx",
  "org_slug": "acme-corp",
  "org_role": "org:admin",
  "p": { "rw": { "ns": ["*"] } }
}
```

- `sub` → owner_id for private visibility
- `org_id` → team_id for team visibility
- `org_role` → admin/member (informational, not used for filtering)
- `p` → Turso permissions (from JWT template)

---

## 3. What We're Validating

8 hypotheses:

| # | Hypothesis | Risk if wrong |
|---|---|---|
| J1 | `indexed_packages` schema works in Turso with visibility columns | Schema doesn't support three-tier model |
| J2 | Embedded replica syncs the catalog correctly | Users can't discover packages offline |
| J3 | Clerk JWT claims can drive visibility-scoped queries | Custom RBAC needed (defeats the purpose) |
| J4 | Turso catalog → Lance dataset path → DuckDB search works end-to-end | The full `znt search` flow is broken |
| K1 | Three-tier federated search returns correct scoped results | Users see packages they shouldn't |
| K2 | Private code indexing is discoverable only by the owner | Privacy leak |
| L1 | PRIMARY KEY constraint prevents duplicate concurrent indexing | Race condition → corrupt data |
| L3 | Two embedded replicas (different DBs) coexist in same process | Team + global DB clash |

---

## 4. Dependencies

### Existing (no new crates needed)

All dependencies come from spikes 0.3 and 0.17:

| Crate | Version | Role in spike |
|-------|---------|--------------|
| `libsql` | workspace | Turso Cloud + embedded replicas |
| `clerk-rs` | 0.4.2 | JWT decoding for claim extraction |
| `reqwest` | workspace | Turso Platform API calls (create temp DB) |
| `lancedb` | 0.26 | Write test Lance datasets for K1/K2 |
| `serde_arrow` | 0.13 | Rust struct → RecordBatch for Lance writes |
| `duckdb` | 1.4 | Lance extension reads for J4/K1/K2 |

### New spike file

`zenith/crates/zen-db/src/spike_catalog_visibility.rs`

This spike is in `zen-db` (not `zen-lake`) because Turso/libsql is the primary concern. Tests J4/K1/K2 also use lancedb + DuckDB but those are dev-dependencies.

### Dev-dependencies to add to zen-db

```toml
lancedb.workspace = true
arrow-array.workspace = true
arrow-schema.workspace = true
serde_arrow.workspace = true
duckdb.workspace = true
```

---

## 5. Spike Tests

**File**: `zenith/crates/zen-db/src/spike_catalog_visibility.rs`

### Part J: Turso Catalog + Visibility (4 tests)

| # | Test | Validates |
|---|------|-----------|
| J1 | `spike_turso_indexed_packages_schema` | Create `indexed_packages` table in Turso Cloud with all columns (visibility, owner_id, team_id, r2_lance_path, etc.). INSERT rows with visibility = public, team, private. Query with visibility-scoped WHERE clause. Verify correct rows per scope. |
| J2 | `spike_turso_catalog_embedded_replica` | Write catalog rows to Turso Cloud. Create embedded replica. `sync()`. Query replica — verify same results as remote. Validates offline catalog access. |
| J3 | `spike_clerk_jwt_visibility_scoping` | Decode the Clerk JWT from env (`ZENITH_AUTH__TEST_TOKEN`). Extract `sub` (user_id). Build visibility WHERE clause using the claim. Execute against Turso. Verify: owner sees public + private (their own), non-owner sees only public. |
| J4 | `spike_catalog_to_lance_search_e2e` | Full end-to-end: Write a small Lance dataset locally via lancedb (serde_arrow production path). INSERT catalog row into Turso with `r2_lance_path` pointing to local Lance. Query Turso to get the lance path. Pass path to DuckDB `lance_vector_search()`. Verify search results. This is the complete `znt search` flow. |

### Part K: Three-Tier Search (2 tests)

| # | Test | Validates |
|---|------|-----------|
| K1 | `spike_three_tier_search` | Write 3 Lance datasets locally (public, team, private symbols). Insert 3 catalog rows in Turso with appropriate visibility. Query Turso as team member (simulate org_id match) → get public + team paths. Run `lance_vector_search()` on each path, merge results. Verify team member doesn't see private. |
| K2 | `spike_private_code_indexing` | Simulate `znt index .` for private code. Create Lance dataset with private symbols via serde_arrow. Insert into Turso: `visibility='private', owner_id=<jwt.sub>`. Search as owner — finds private symbols. Search as different user — excluded. |

### Part L: Operational (2 tests)

| # | Test | Validates |
|---|------|-----------|
| L1 | `spike_concurrent_index_turso_lock` | Two concurrent INSERTs for same (ecosystem, package, version) into Turso. PRIMARY KEY constraint → first writer wins, second gets `SQLITE_CONSTRAINT`. Validates distributed lock via Turso for crowdsource deduplication. |
| L3 | `spike_two_turso_replicas_same_process` | Create a temporary second Turso DB via Platform API. Open embedded replicas of both DBs simultaneously in the same process. Write to both, sync both, query both. Verify no interference. Validates global DB + team DB coexistence. |

**Total: 8 tests**

---

## 6. Evaluation Criteria

| Criterion | Weight | How We Measure |
|-----------|--------|---------------|
| Catalog schema in Turso | **Critical** | Test J1: all column types, INSERT, SELECT with WHERE |
| Embedded replica sync | **Critical** | Test J2: replica matches remote after sync |
| Clerk JWT → visibility scoping | **Critical** | Test J3: correct rows per user identity |
| End-to-end catalog → search | **Critical** | Test J4: Turso catalog → Lance path → DuckDB search |
| Three-tier scoping | **High** | Test K1: team member sees public + team, not private |
| Private code isolation | **High** | Test K2: only owner sees private |
| Concurrent write dedup | **High** | Test L1: PRIMARY KEY prevents duplicates |
| Multiple replicas coexist | **Medium** | Test L3: two DBs in same process |

---

## 7. What This Spike Does NOT Test

- **R2 writes** — spike 0.19 validated lancedb → R2. This spike uses local Lance only.
- **R2 temporary credentials** — CF Worker credential minting is Phase 9.
- **Real package data** — uses synthetic test data.
- **Turso partial sync** — v0.4 feature, not yet in Rust SDK.
- **Schema migrations** — production migration system is Phase 2.
- **Turso billing/quotas** — free tier limits are known (500MB, 1 DB free; Scaler $29/mo).

---

## 8. Success Criteria

- **Turso `indexed_packages` schema works** with visibility scoping (J1 passes)
- **Embedded replica syncs catalog** (J2 passes)
- **Clerk JWT claims drive visibility** without custom RBAC (J3 passes)
- **Full search flow works**: Turso catalog → Lance path → DuckDB search (J4 passes)
- **Three-tier search returns correct scoped results** (K1 passes)
- **Private code is isolated** (K2 passes)
- **Concurrent writes are deduplicated** via PRIMARY KEY (L1 passes)
- **All 8 tests pass** (Turso tests skipped if credentials missing)

---

## 9. Post-Spike Actions

### If Spike Passes (Expected Path)

| Doc | Update |
|-----|--------|
| `07-implementation-plan.md` | Add spike 0.20 to Phase 0 table. Update Phase 8/9 with Turso catalog + visibility model. Remove all MotherDuck references. |
| `02-ducklake-data-model.md` | Rewrite as `02-data-architecture.md` — three-tier index, Turso catalog, Lance storage, DuckDB query engine. |
| `05-crate-designs.md` | Update zen-db with `indexed_packages` schema. Add zen-auth crate design. Drop DuckLake/MotherDuck. |
| `INDEX.md` | Add docs 17 + 18. Update crate list. |

### If Turso Doesn't Support Multiple DBs per Process (Fallback)

- Use a single Turso DB with table prefixes (`global_indexed_packages`, `team_indexed_packages`)
- Or use separate connections with different auth tokens but same DB

---

## Cross-References

- Native lancedb writes: [spike_native_lance.rs](../../crates/zen-lake/src/spike_native_lance.rs) (spike 0.19)
- Turso embedded replicas: [spike_libsql_sync.rs](../../crates/zen-db/src/spike_libsql_sync.rs) (spike 0.3)
- Clerk auth + Turso JWKS: [spike_clerk_auth.rs](../../crates/zen-db/src/spike_clerk_auth.rs) (spike 0.17)
- R2 Lance: [spike_r2_parquet.rs](../../crates/zen-lake/src/spike_r2_parquet.rs) (spike 0.18)
- Implementation plan: [07-implementation-plan.md](./07-implementation-plan.md)
- Native lance spike plan: [17-native-lance-spike-plan.md](./17-native-lance-spike-plan.md) (spike 0.19)
