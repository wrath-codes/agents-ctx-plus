# Phase 9: Visibility & Teams — Implementation Plan

**Version**: 2026-02-20
**Status**: Planning
**Depends on**: Phase 8 (Authentication & Identity — **DONE**: zen-auth crate, CLI auth commands, AppContext identity-aware startup, Turso JWKS wiring), Phase 8 Cloud (Turso catalog + R2 Lance infrastructure — **DONE**: `dl_data_file` table with `visibility`/`org_id`/`owner_sub` columns, basic `write_to_r2()` for public packages, `search_cloud_vector()` without visibility scoping), Spikes 0.17 (Clerk auth + Turso JWKS — **14/14 pass**), 0.18 (R2 Lance — **18/18 pass**), 0.19 (native lancedb — **10/10 pass**), and 0.20 (catalog visibility — **9/9 pass**)
**Produces**: Milestone 9 — Three-tier visibility model (public/team/private) threaded through all entity repos and catalog queries. Identity-aware entity creation (org_id set from Clerk JWT). Visibility-scoped search across Turso catalog and R2 Lance datasets. Crowdsource dedup. Private code indexing (`znt index .`). Team management commands (`znt team invite/list`). Federated search across visibility tiers.

> **⚠️ Scope**: Phase 9 is **visibility, teams, and cloud identity integration**. It builds on the auth infrastructure from Phase 8 (zen-auth crate, AppContext identity, Turso JWKS). Phase 8 provided the identity layer — who you are. Phase 9 provides the authorization layer — what you can see and do.
>
> **📌 Numbering note**: The master roadmap (`07-implementation-plan.md` §11) defines Phase 9 tasks. Tasks 9.1–9.7, 9.13–9.14, 9.17–9.20 were completed in Phase 8 (authentication infrastructure). This plan covers remaining tasks: 9.8 (org_id columns + migration), 9.9 (AuthContext struct), 9.10 (visibility-scoped repo queries), 9.11 (visibility-scoped catalog queries), 9.12 (crowdsource dedup), 9.15 (R2 Lance uploads with visibility), 9.16 (federated search), 9.21 (team mode startup wiring), 9.22 (znt team invite/list), 9.23 (znt index . for private code).

---

## Table of Contents

1. [Overview](#1-overview)
2. [Implementation Outcome](#2-implementation-outcome)
3. [Key Decisions](#3-key-decisions)
4. [Architecture](#4-architecture)
5. [PR 1 — Stream A: Schema Migration + Identity Threading](#5-pr-1--stream-a-schema-migration--identity-threading)
6. [PR 2 — Stream B: Catalog Visibility + R2 Writes + Federated Search](#6-pr-2--stream-b-catalog-visibility--r2-writes--federated-search)
7. [PR 3 — Stream C: CLI Team Commands + Private Indexing + Wiring](#7-pr-3--stream-c-cli-team-commands--private-indexing--wiring)
8. [Execution Order](#8-execution-order)
9. [Gotchas & Warnings](#9-gotchas--warnings)
10. [Milestone 9 Validation](#10-milestone-9-validation)
11. [Validation Traceability Matrix](#11-validation-traceability-matrix)
12. [Plan Review — Mismatch Log](#12-plan-review--mismatch-log)

---

## 1. Overview

**Goal**: Thread the identity layer from Phase 8 through all entity repositories and catalog queries so that entities are owned by organizations, catalog entries have visibility tiers (public/team/private), and search respects visibility boundaries. Add team management CLI commands and private code indexing.

**Current state**: Phase 8 provides `AppContext.identity: Option<AuthIdentity>` with `user_id`, `org_id`, `org_slug`, `org_role` extracted from a validated Clerk JWT. The `dl_data_file` catalog table already has `visibility`, `org_id`, and `owner_sub` columns (from `002_catalog.sql`). However:
- Entity tables (sessions, findings, hypotheses, etc.) have no `org_id` column
- `register_catalog_data_file()` hardcodes `visibility='public'` and ignores `org_id`/`owner_sub`
- `discover_catalog_paths` and `catalog_paths_for_package` don't filter by visibility
- `install.rs` always writes public catalog entries regardless of auth state
- `search.rs` uses `ctx.config.turso.auth_token` directly instead of the resolved auth token
- No `znt team` or `znt index .` commands exist

**Crates touched**:
- `zen-core` — **light**: Add `Visibility` enum (~10 LOC)
- `zen-db` — **heavy**: New migration `003_team.sql`, store identity in `ZenService`, update all 13 repo modules to write `org_id` on creates and filter by identity on reads, update catalog repo for visibility-scoped queries and crowdsource dedup (~600 LOC)
- `zen-lake` — **medium**: Update `write_to_r2()` to accept visibility + owner metadata, update catalog path discovery with visibility filtering, implement federated search across visibility tiers (~250 LOC)
- `zen-cli` — **medium**: New `commands/team/` module with invite/list, new `commands/index.rs` for `znt index .`, pass identity through install/search commands, wire team mode startup (~350 LOC)
- `zen-auth` — **light**: Add `org_members()` and `org_invite()` helpers for Clerk organization API (~60 LOC)

**Dependency changes needed**:
- `zen-cli`: `zen-auth.workspace = true` already present from Phase 8
- No new external dependencies — all required crates already in workspace

**Estimated deliverables**: ~20 modified production files, ~5 new files, ~1,270 LOC production code, ~300 LOC tests

**PR strategy**: 3 PRs by stream. Stream A provides schema foundation and identity threading. Stream B implements cloud-layer visibility. Stream C adds CLI commands and wires everything together.

| PR | Stream | Contents | Depends On |
|----|--------|----------|------------|
| PR 1 | A: Schema + Identity Threading | `003_team.sql`, `ZenService` identity field, repo org_id writes/filters, `Visibility` enum | None (Phase 8 complete) |
| PR 2 | B: Catalog Visibility + Cloud | `catalog.rs` visibility queries, `r2_write.rs` with visibility, `cloud_search.rs` federated search, crowdsource dedup | Stream A |
| PR 3 | C: CLI Commands + Wiring | `commands/team/`, `commands/index.rs`, install/search identity pass-through, team mode startup | Streams A + B |

---

## 2. Implementation Outcome

| Task | Description | Status | Notes |
|------|-------------|--------|-------|
| 9.8 | org_id columns + `003_team.sql` migration | ✅ Done | 10 entity tables, 5 indexes, idempotent migration runner |
| 9.9 | Identity in `ZenService` | ✅ Done | `identity`, `org_id()`, `user_id()`, `org_id_filter()` helpers |
| 9.10 | Visibility-scoped repo queries | ✅ Done | All 10 entity repos write org_id on create, filter on list/search. `whats_next` scoped. |
| 9.11 | Visibility-scoped catalog queries | ✅ Done | `catalog_paths_for_package_scoped()`, unscoped defaults to public-only, `visibility_filter_sql()` |
| 9.12 | Crowdsource dedup | ✅ Done | `catalog_check_before_index()` + wired into install.rs before clone. `ON CONFLICT DO NOTHING` on register. |
| 9.15 | R2 Lance uploads with visibility | ✅ Done | `write_to_r2()` accepts `Visibility`, R2 paths include visibility prefix |
| 9.16 | Federated search | ✅ Done | `discover_catalog_paths_scoped()`, `search_cloud_vector_scoped()` in zen-lake |
| 9.21 | Team mode startup wiring | ✅ Done | `AppContext` passes identity + auth_token to `ZenService`, search uses resolved token |
| 9.22 | `znt team invite/list` | ✅ Done | `zen-auth/org.rs` + `zen-cli/commands/team/` with auth guards |
| 9.23 | `znt index .` | ✅ Done | Private visibility, requires auth, uses `local` ecosystem |

**Plan deviations**:
- §4 lists `audit.rs` as MODIFIED, but §3.3 explicitly excludes `audit_trail` from org_id ("append-only log"). §3.3 takes precedence — audit.rs unchanged.
- `Visibility` enum uses `Copy` + `Hash` derives (not in plan but idiomatic for small enums).
- Crowdsource dedup check placed before git clone (not just before indexing) to also skip the expensive clone step.

---

## 3. Key Decisions

All decisions derive from spike 0.20 findings ([spike_catalog_visibility.rs](../../crates/zen-db/src/spike_catalog_visibility.rs)), spike 0.18 findings ([spike_r2_parquet.rs](../../crates/zen-lake/src/spike_r2_parquet.rs)), spike 0.19 findings ([spike_native_lance.rs](../../crates/zen-lake/src/spike_native_lance.rs)), and Phase 8 conventions.

### 3.1 Identity Stored in ZenService (Not Passed Per-Method)

**Decision**: Add `identity: Option<AuthIdentity>` to `ZenService`. Set it once at construction time from `AppContext.identity`. All repo methods access `self.identity()` internally for org_id scoping. No changes to the existing repo method signatures.

**Rationale**: Threading `Option<&AuthIdentity>` through every repo method (13 modules × ~5 methods each = ~65 method signatures) would be extremely invasive and create merge pain with any in-flight work. Storing identity in `ZenService` follows the same pattern as `TrailWriter` and `SchemaRegistry` — set once, used throughout. The identity is immutable for the lifetime of a CLI command.

**Impact**: `ZenService::new_local()` and `ZenService::new_synced()` gain an `identity: Option<AuthIdentity>` parameter. `ZenService::from_db()` gains the same. `AppContext::init()` passes `identity.clone()` to the service constructor.

**Migration path**: Existing repo methods continue to work unchanged. Entity creation methods internally call `self.org_id()` (returns `Option<&str>`) to populate the `org_id` column. Query methods internally call `self.visibility_filter()` to build WHERE clauses. No callers outside `zen-db` need to change.

### 3.2 Three-Tier Visibility Model

**Decision**: The catalog table `dl_data_file` uses three visibility tiers:

| Tier | `visibility` | Who Writes | Who Reads | `org_id` | `owner_sub` |
|------|-------------|------------|-----------|----------|-------------|
| **Public** | `'public'` | Any authenticated user | Everyone | NULL | `user_id` (informational) |
| **Team** | `'team'` | Team members | Same-org members | `org_id` from JWT | `user_id` |
| **Private** | `'private'` | Package owner | Owner only | NULL | `user_id` |

**Rationale**: Validated in spike 0.20. The `dl_data_file` table already has `visibility`, `org_id`, and `owner_sub` columns from `002_catalog.sql`. No schema change needed for the catalog. The scoping query is:

```sql
WHERE visibility = 'public'
   OR (visibility = 'team' AND org_id = ?1)
   OR (visibility = 'private' AND owner_sub = ?2)
-- ?1 = identity.org_id, ?2 = identity.user_id
```

**Free vs Pro boundary**: No license check. Visibility tier is determined by context: `znt install <pkg>` writes public entries. `znt index .` writes private entries (requires auth). Team visibility requires `org_id` in JWT (Clerk org membership).

### 3.3 Entity `org_id` Column — Nullable, Write-Only Scoping

**Decision**: Add `org_id TEXT` column to all entity tables via `003_team.sql`. The column is:
- **Nullable**: `NULL` means local-only / pre-auth entity. Existing data migrates naturally.
- **Write-once on create**: Set from `identity.org_id` when creating entities. Never modified after creation.
- **Read-scoped on list/search**: Entity list and search queries add `AND (org_id = ? OR org_id IS NULL)` when identity has an org_id, or `AND org_id IS NULL` when no identity. This ensures team members see their team's entities plus any pre-auth local entities. **Get-by-ID methods are intentionally unscoped** — entity IDs are random UUIDs (not enumerable), the database is project-scoped, and entity-link traversal (e.g., `get_parent_issue`) needs to work across org boundaries.

**Rationale**: Adding `org_id` to entities enables team-scoped research workflows where multiple agents/users in the same Clerk org share findings, hypotheses, and tasks via Turso sync. The nullable default preserves backward compatibility — all existing entities (created without auth) continue to be visible.

**Tables receiving `org_id`**: `sessions`, `research_items`, `findings`, `hypotheses`, `insights`, `issues`, `tasks`, `studies`, `implementation_log`, `compatibility_checks`. NOT added to: `project_meta` (key-value, no entity semantics), `project_dependencies` (package-level, not user-scoped), `session_snapshots` (derived from session), `finding_tags` (child of findings), `entity_links` (references other entities), `audit_trail` (append-only log).

### 3.4 Catalog Visibility-Scoped Queries Replace Unscoped Queries

**Decision**: Add new `_scoped` methods (`catalog_paths_for_package_scoped()`, `discover_catalog_paths_scoped()`, `search_cloud_vector_scoped()`) that accept identity and apply visibility filtering. The original unscoped methods (`catalog_paths_for_package()`, `discover_catalog_paths()`) are **updated to default to `visibility = 'public'` only** as a safe default. This ensures that any code path that forgets to use the scoped variant will never leak private or team data.

**Rationale**: Currently the unscoped methods return ALL catalog entries regardless of visibility. Simply adding new `_scoped` methods while leaving the originals returning everything would create a security footgun — any caller accidentally using the unscoped version would leak private entries. Defaulting unscoped methods to public-only is the safe default. `catalog_has_package()` is updated to check public entries only (used for crowdsource dedup, which is inherently public-scoped).

**Breaking change**: Callers that need multi-tier visibility (install.rs, search.rs) must switch to the `_scoped` variants. Callers that only need public data (existing behavior) continue to work unchanged.

### 3.5 Crowdsource Dedup — Check Before Index, Handle SQLITE_CONSTRAINT

**Decision**: Before indexing a public package, check the catalog for existing entries. If the package+version already exists with `visibility='public'`, skip indexing and use the existing Lance paths. If a concurrent write race produces `SQLITE_CONSTRAINT` on the unique index `ux_dl_data_file_triplet_path`, catch the error and treat it as success (first writer wins).

**Rationale**: Validated in spike 0.20 test `test_concurrent_dedup_first_writer_wins`. Two authenticated users running `znt install tokio@1.49` simultaneously should not produce duplicate Lance datasets. The unique constraint on `(ecosystem, package, version, lance_path)` in `002_catalog.sql` provides the safety net.

**Implementation**: Add `catalog_check_before_index()` method to `ZenService` that returns `Option<Vec<String>>` (existing paths) or `None` (not indexed). `install.rs` calls this before the indexing pipeline. On `SQLITE_CONSTRAINT`, `register_catalog_data_file()` already uses `ON CONFLICT DO NOTHING` — this is correct.

### 3.6 Federated Search — Multi-Tier Lance Query with Merged Results

**Decision**: Implement `search_federated()` in `ZenLake` that:
1. Queries the Turso catalog for visibility-scoped Lance paths
2. Groups paths by visibility tier
3. Runs `lance_vector_search()` against each path
4. Merges and re-ranks results by distance

**Rationale**: A user searching for "spawn task" should see results from (a) public packages like tokio, (b) their team's private SDK if team-scoped, and (c) their own private code. The merge strategy is simple distance-based ranking across all tiers — no tier-based boosting for MVP.

**Implementation**: The existing `search_lance_paths()` method already iterates over paths and merges results. The change is in path discovery — `discover_catalog_paths` gains visibility filtering.

### 3.7 `register_catalog_data_file()` Gains Visibility Parameters

**Decision**: Update `register_catalog_data_file()` to accept `visibility`, `org_id`, and `owner_sub` parameters instead of hardcoding `visibility='public'`.

**Rationale**: Currently the method hardcodes `'public'`. Team and private entries need the caller to specify visibility tier plus the identity metadata. The caller (`install.rs` or `index.rs`) determines visibility from context: `install` = public, `index .` = private, future team indexing = team.

### 3.8 `znt index .` — Private Code Indexing

**Decision**: `znt index .` indexes the current project directory as a private package. It:
1. Requires authentication (fails with message if `identity` is `None`)
2. Parses source code in the project root using the existing `IndexingPipeline`
3. Exports to R2 with `visibility='private'` and `owner_sub=identity.user_id`
4. Registers in Turso catalog with private visibility
5. Uses the project name from `project_meta` or directory name as the package identifier

**Rationale**: This enables searching your own codebase via cloud-backed vector search. Only the owner can discover and search their private indexed code.

**Ecosystem**: Uses `'local'` as the ecosystem for private code, distinguishing it from registry packages (`'rust'`, `'npm'`, etc.).

### 3.9 Team Commands Use Clerk Backend API Directly

**Decision**: `znt team invite` and `znt team list` call the Clerk Backend API via `reqwest` (same pattern as `api_key.rs` in zen-auth). They use the `config.clerk.secret_key` for authentication. The Clerk organization API endpoints are:
- `POST /v1/organizations/{org_id}/invitations` — invite a user
- `GET /v1/organizations/{org_id}/memberships` — list members

**Rationale**: Spike 0.20 confirmed that `clerk-rs` doesn't expose organization management APIs. Raw `reqwest` calls work (same as `api_key.rs`). These endpoints are well-documented in the Clerk API reference.

**Requirement**: `identity.org_id` must be `Some` (user must be in an org). The `config.clerk.secret_key` must be non-empty. Both conditions are checked before calling the API.

### 3.10 Search Uses Resolved Auth Token (Not Config Token)

**Decision**: `search.rs` `try_cloud_vector_search()` must use the resolved auth token from Phase 8 (stored alongside identity in AppContext) instead of `ctx.config.turso.auth_token`. This ensures that Clerk JWT-authenticated users use their JWT for catalog + Lance queries.

**Rationale**: Currently `search.rs` line 251 passes `&ctx.config.turso.auth_token` to `search_cloud_vector()`. This is the tier 4 legacy Platform API token. Phase 8 resolved the actual auth token but only stored the identity — the raw token isn't available in AppContext. The fix is to add `auth_token: Option<String>` to AppContext alongside `identity`, populated from `resolve_auth()`.

**Impact**: `AppContext` gains `pub auth_token: Option<String>`. `resolve_auth()` already returns `(Option<String>, Option<AuthIdentity>)` — the `Option<String>` is the raw token. Currently stored in a local variable; now also stored in AppContext.

---

## 4. Architecture

### Module Structure — Changes

```
zen-core/src/
├── identity.rs                        # UNCHANGED — AuthIdentity struct
└── enums.rs                           # MODIFIED — add Visibility enum

zen-db/
├── migrations/
│   └── 003_team.sql                   # NEW — org_id columns on entity tables
├── src/
│   ├── migrations.rs                  # MODIFIED — add MIGRATION_003
│   ├── service.rs                     # MODIFIED — add identity field + helpers
│   └── repos/
│       ├── audit.rs                   # MODIFIED — write org_id on audit entries
│       ├── catalog.rs                 # MODIFIED — visibility-scoped queries + crowdsource dedup
│       ├── compat.rs                  # MODIFIED — write/filter org_id
│       ├── finding.rs                 # MODIFIED — write/filter org_id
│       ├── hypothesis.rs              # MODIFIED — write/filter org_id
│       ├── impl_log.rs                # MODIFIED — write/filter org_id
│       ├── insight.rs                 # MODIFIED — write/filter org_id
│       ├── issue.rs                   # MODIFIED — write/filter org_id
│       ├── research.rs                # MODIFIED — write/filter org_id
│       ├── session.rs                 # MODIFIED — write/filter org_id
│       ├── study.rs                   # MODIFIED — write/filter org_id
│       ├── task.rs                    # MODIFIED — write/filter org_id
│       └── whats_next.rs              # MODIFIED — filter org_id in aggregates

zen-auth/src/
└── org.rs                             # NEW — Clerk organization API (invite, list members)

zen-lake/src/
├── cloud_search.rs                    # MODIFIED — visibility-scoped catalog discovery
├── r2_write.rs                        # MODIFIED — accept visibility + owner metadata
└── store.rs                           # MODIFIED — add catalog dedup check method

zen-cli/src/
├── cli/
│   ├── root_commands.rs               # MODIFIED — add Team + Index variants
│   └── subcommands/
│       ├── mod.rs                     # MODIFIED — add TeamCommands export
│       └── team.rs                    # NEW — TeamCommands subcommands
├── commands/
│   ├── dispatch.rs                    # MODIFIED — add Team + Index dispatch
│   ├── init.rs                        # MODIFIED — pass None identity to ZenService::new_local()
│   ├── install.rs                     # MODIFIED — pass identity for visibility
│   ├── search.rs                      # MODIFIED — use resolved auth token
│   ├── rebuild/handle.rs              # MODIFIED — pass None identity to ZenService::new_local()
│   ├── hook/rebuild_trigger.rs        # MODIFIED — pass None identity to ZenService::new_local()
│   ├── team/
│   │   ├── mod.rs                     # NEW — dispatch team subcommands
│   │   ├── invite.rs                  # NEW — znt team invite
│   │   └── list.rs                    # NEW — znt team list
│   └── index.rs                       # NEW — znt index . handler
└── context/
    └── app_context.rs                 # MODIFIED — add auth_token field, pass identity to service

zen-search/src/
├── graph.rs                           # MODIFIED — pass None identity to ZenService::new_local() in tests
└── lib.rs                             # MODIFIED — pass None identity to ZenService::new_local() in tests

zen-db/
└── tests/pr1_infrastructure.rs        # MODIFIED — pass None identity to ZenService::new_local() in tests
```

### Upstream Dependencies — All Ready

| Dependency | Validated By | Usage |
|------------|-------------|-------|
| `dl_data_file.visibility` column | 002_catalog.sql | Already exists — `public`/`team`/`private` |
| `dl_data_file.org_id` column | 002_catalog.sql | Already exists — team-scoped entries |
| `dl_data_file.owner_sub` column | 002_catalog.sql | Already exists — owner tracking |
| `ux_dl_data_file_triplet_path` unique index | 002_catalog.sql | Already exists — dedup constraint |
| `ON CONFLICT DO NOTHING` on catalog insert | `catalog.rs` | Already used in `register_catalog_data_file()` |
| Visibility-scoped catalog SQL | Spike 0.20 J3 | `WHERE visibility='public' OR (visibility='team' AND org_id=?) OR (visibility='private' AND owner_sub=?)` |
| Concurrent dedup via PRIMARY KEY | Spike 0.20 L1 | First writer wins, `SQLITE_CONSTRAINT` caught |
| Programmatic org JWT with `org_id` | Spike 0.20 J0 | JWT template includes org claims |
| `write_to_r2()` serde_arrow pipeline | Spike 0.19 | Symbols + doc chunks to Lance on R2 |
| `lance_vector_search()` with visibility | Spike 0.18/0.20 | Lance search returns correct results |
| `AppContext.identity` | Phase 8 | `Option<AuthIdentity>` populated from Clerk JWT |
| `resolve_auth()` returning `(token, identity)` | Phase 8 | Four-tier token resolution |
| Clerk Backend API organization endpoints | Spike 0.20 | Raw `reqwest` calls to `/v1/organizations/` |

### Data Flow

```
Entity Creation (e.g., znt finding create):
  → ZenService.create_finding(session_id, content, ...)
    → self.org_id() → identity.as_ref().and_then(|i| i.org_id.as_deref())
    → INSERT INTO findings (..., org_id) VALUES (..., ?org_id)
    → Trail + Audit as before

Entity Query (e.g., znt finding list):
  → ZenService.list_findings(session_id)
    → self.org_id_filter() → builds "AND (org_id = ? OR org_id IS NULL)" or "AND org_id IS NULL"
    → SELECT ... FROM findings WHERE session_id = ? {org_id_filter}

Catalog Registration (e.g., znt install):
  → install.rs determines visibility from context:
    → `znt install tokio` → visibility='public', org_id=NULL, owner_sub=user_id
    → `znt index .` → visibility='private', org_id=NULL, owner_sub=user_id
  → register_catalog_data_file(eco, pkg, ver, path, visibility, org_id, owner_sub)

Catalog Search (e.g., znt search):
  → discover_catalog_paths(eco, pkg, ver, identity)
    → WHERE visibility = 'public'
       OR (visibility = 'team' AND org_id = ?identity.org_id)
       OR (visibility = 'private' AND owner_sub = ?identity.user_id)
  → search_lance_paths(paths, embedding, k)
  → merge results by distance

Team Commands:
  → znt team invite <email>
    → identity.org_id required
    → POST /v1/organizations/{org_id}/invitations with config.clerk.secret_key
  → znt team list
    → GET /v1/organizations/{org_id}/memberships
```

---

## 5. PR 1 — Stream A: Schema Migration + Identity Threading

**Tasks**: 9.8 (org_id columns), 9.9 (identity in ZenService), 9.10 (visibility-scoped repos)
**Estimated LOC**: ~600 production, ~150 tests

### A1. `zen-core/src/enums.rs` — Visibility Enum (task 9.8a)

Add to the existing `enums.rs` file:

```rust
/// Visibility tier for catalog entries and cloud-indexed data.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Visibility {
    /// Visible to all authenticated users.
    Public,
    /// Visible to members of the same Clerk organization.
    Team,
    /// Visible only to the owner.
    Private,
}

impl Visibility {
    /// SQL string representation.
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Public => "public",
            Self::Team => "team",
            Self::Private => "private",
        }
    }
}

impl std::fmt::Display for Visibility {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}
```

### A2. `zen-db/migrations/003_team.sql` — Entity org_id Columns (task 9.8)

```sql
-- 003_team.sql
-- Add org_id column to entity tables for team-scoped visibility.
-- NULL = local-only / pre-auth entity. Preserves backward compatibility.

ALTER TABLE sessions ADD COLUMN org_id TEXT;
ALTER TABLE research_items ADD COLUMN org_id TEXT;
ALTER TABLE findings ADD COLUMN org_id TEXT;
ALTER TABLE hypotheses ADD COLUMN org_id TEXT;
ALTER TABLE insights ADD COLUMN org_id TEXT;
ALTER TABLE issues ADD COLUMN org_id TEXT;
ALTER TABLE tasks ADD COLUMN org_id TEXT;
ALTER TABLE studies ADD COLUMN org_id TEXT;
ALTER TABLE implementation_log ADD COLUMN org_id TEXT;
ALTER TABLE compatibility_checks ADD COLUMN org_id TEXT;

-- Index for org_id filtering on frequently queried tables.
CREATE INDEX IF NOT EXISTS idx_sessions_org ON sessions(org_id);
CREATE INDEX IF NOT EXISTS idx_findings_org ON findings(org_id);
CREATE INDEX IF NOT EXISTS idx_tasks_org ON tasks(org_id);
CREATE INDEX IF NOT EXISTS idx_issues_org ON issues(org_id);
CREATE INDEX IF NOT EXISTS idx_research_org ON research_items(org_id);
```

**Important**: SQLite `ALTER TABLE ADD COLUMN` does NOT support `IF NOT EXISTS`. The migration runner must handle the case where columns already exist (re-running migration). Resolution: wrap each `ALTER TABLE` in a try-catch pattern in the migration runner, or check `PRAGMA table_info()` first. Simplest approach: catch the `duplicate column name` error per statement. See §A3.

### A3. `zen-db/src/migrations.rs` — Add Migration 003 (task 9.8)

```rust
const MIGRATION_001: &str = include_str!("../migrations/001_initial.sql");
const MIGRATION_002: &str = include_str!("../migrations/002_catalog.sql");
const MIGRATION_003: &str = include_str!("../migrations/003_team.sql");

impl ZenDb {
    pub(crate) async fn run_migrations(&self) -> Result<(), DatabaseError> {
        self.conn
            .execute_batch(MIGRATION_001)
            .await
            .map_err(|e| DatabaseError::Migration(format!("001_initial: {e}")))?;
        self.conn
            .execute_batch(MIGRATION_002)
            .await
            .map_err(|e| DatabaseError::Migration(format!("002_catalog: {e}")))?;

        // 003_team: ALTER TABLE ADD COLUMN statements may fail if columns already
        // exist (re-run on existing DB). Execute each statement individually and
        // ignore "duplicate column name" errors.
        //
        // Split on `;` (not by line) to handle multi-line statements correctly.
        for raw_stmt in MIGRATION_003.split(';') {
            let stmt = raw_stmt.trim();
            if stmt.is_empty() || stmt.starts_with("--") {
                continue;
            }
            // Skip comment-only fragments (lines starting with --)
            let non_comment: String = stmt
                .lines()
                .filter(|l| !l.trim_start().starts_with("--"))
                .collect::<Vec<_>>()
                .join(" ");
            let non_comment = non_comment.trim();
            if non_comment.is_empty() {
                continue;
            }
            match self.conn.execute(stmt, ()).await {
                Ok(_) => {}
                Err(e) if e.to_string().contains("duplicate column name") => {}
                Err(e) if e.to_string().contains("already exists") => {}
                Err(e) => return Err(DatabaseError::Migration(format!("003_team: {e}"))),
            }
        }

        Ok(())
    }
}
```

**Note**: Unlike 001/002 which use `IF NOT EXISTS` everywhere, SQLite `ALTER TABLE` doesn't support that clause. The migration splits on `;` (not newlines) to handle multi-line statements correctly (e.g., `CREATE INDEX IF NOT EXISTS`). Each statement is executed individually, and "duplicate column name" / "already exists" errors are ignored for idempotent re-running.

### A4. `zen-db/src/service.rs` — Identity Field + Helpers (task 9.9)

```rust
pub struct ZenService {
    db: ZenDb,
    trail: TrailWriter,
    schema: SchemaRegistry,
    identity: Option<AuthIdentity>,  // NEW
}

impl ZenService {
    pub async fn new_local(
        db_path: &str,
        trail_dir: Option<PathBuf>,
        identity: Option<AuthIdentity>,  // NEW
    ) -> Result<Self, DatabaseError> {
        let db = ZenDb::open_local(db_path).await?;
        let trail = match trail_dir {
            Some(dir) => TrailWriter::new(dir)?,
            None => TrailWriter::disabled(),
        };
        let schema = SchemaRegistry::new();
        Ok(Self { db, trail, schema, identity })
    }

    pub async fn new_synced(
        local_replica_path: &str,
        remote_url: &str,
        auth_token: &str,
        trail_dir: Option<PathBuf>,
        identity: Option<AuthIdentity>,  // NEW
    ) -> Result<Self, DatabaseError> {
        let db = ZenDb::open_synced(local_replica_path, remote_url, auth_token).await?;
        let trail = match trail_dir {
            Some(dir) => TrailWriter::new(dir)?,
            None => TrailWriter::disabled(),
        };
        let schema = SchemaRegistry::new();
        Ok(Self { db, trail, schema, identity })
    }

    #[must_use]
    pub fn from_db(db: ZenDb, trail: TrailWriter, identity: Option<AuthIdentity>) -> Self {
        Self {
            db,
            trail,
            schema: SchemaRegistry::new(),
            identity,
        }
    }

    /// Authenticated user identity, if available.
    #[must_use]
    pub const fn identity(&self) -> Option<&AuthIdentity> {
        self.identity.as_ref()
    }

    /// The org_id from the authenticated identity, for entity creation.
    /// Returns `None` when unauthenticated or when no org is active.
    #[must_use]
    pub fn org_id(&self) -> Option<&str> {
        self.identity.as_ref().and_then(|i| i.org_id.as_deref())
    }

    /// The user_id from the authenticated identity.
    /// Returns `None` when unauthenticated.
    #[must_use]
    pub fn user_id(&self) -> Option<&str> {
        self.identity.as_ref().map(|i| i.user_id.as_str())
    }

    // ... existing methods unchanged ...
}
```

### A5. Repo Org_id Writes — Pattern for All 10 Entity Repos (task 9.10a)

**Pattern for entity creation** (applied to all create methods in finding.rs, hypothesis.rs, insight.rs, issue.rs, task.rs, research.rs, session.rs, study.rs, impl_log.rs, compat.rs):

The INSERT SQL gains an `org_id` column. The value comes from `self.org_id()`.

Example for `create_finding()`:

```rust
// BEFORE (Phase 2):
self.db().conn().execute(
    "INSERT INTO findings (id, research_id, session_id, content, source, confidence, created_at, updated_at)
     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
    libsql::params![id.as_str(), research_id, session_id, content, source, confidence.as_str(), now.to_rfc3339(), now.to_rfc3339()],
).await?;

// AFTER (Phase 9):
self.db().conn().execute(
    "INSERT INTO findings (id, research_id, session_id, content, source, confidence, created_at, updated_at, org_id)
     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
    libsql::params![id.as_str(), research_id, session_id, content, source, confidence.as_str(), now.to_rfc3339(), now.to_rfc3339(), self.org_id()],
).await?;
```

**Note**: `self.org_id()` returns `Option<&str>`. The `libsql` `params!` macro supports `Option<&str>` natively (maps to SQL NULL when `None`), validated in spike 0.2g. No `Vec<Value>` needed.

### A6. Repo Org_id Read Filters — Pattern for All Query Methods (task 9.10b)

**Pattern for entity queries** (applied to all list/get/search methods):

Add an org_id filter to WHERE clauses. The filter logic:
- When `self.org_id()` is `Some(org_id)`: `AND (org_id = ? OR org_id IS NULL)` — see both team entities and pre-auth local entities
- When `self.org_id()` is `None`: `AND org_id IS NULL` — see only local entities (no team entities leak to unauthenticated users)

Helper method on `ZenService`:

```rust
impl ZenService {
    /// Build an org_id filter clause and its parameter.
    ///
    /// Returns `(sql_fragment, params)` where:
    /// - Authenticated with org: `("AND (org_id = ?N OR org_id IS NULL)", vec![org_id.into()])`
    /// - No org / unauthenticated: `("AND org_id IS NULL", vec![])`
    fn org_id_filter(&self, param_index: u32) -> (String, Vec<libsql::Value>) {
        match self.org_id() {
            Some(org_id) => (
                format!("AND (org_id = ?{param_index} OR org_id IS NULL)"),
                vec![org_id.into()],
            ),
            None => ("AND org_id IS NULL".to_string(), vec![]),
        }
    }
}
```

Example for `list_findings()`:

```rust
// BEFORE:
let mut rows = self.db().conn().query(
    "SELECT id, research_id, session_id, content, source, confidence, created_at, updated_at
     FROM findings WHERE session_id = ?1 ORDER BY created_at DESC",
    [session_id],
).await?;

// AFTER:
let (org_filter, org_params) = self.org_id_filter(2);
let sql = format!(
    "SELECT id, research_id, session_id, content, source, confidence, created_at, updated_at
     FROM findings WHERE session_id = ?1 {org_filter} ORDER BY created_at DESC"
);
let mut params: Vec<libsql::Value> = vec![session_id.into()];
params.extend(org_params);
let mut rows = self.db().conn().query(&sql, libsql::params_from_iter(params)).await?;
```

**Important**: The `get_finding()` (by ID) method does NOT add org_id filtering — if you have the ID, you can read it. Only list/search methods get the filter. This follows the principle that IDs are unforgeable (random hex) and entity links reference by ID across orgs.

### A7. `whats_next.rs` — Update Aggregate Queries (task 9.10c)

The `whats_next()` aggregate query must respect org_id scoping. All sub-queries (open tasks count, pending hypotheses count, etc.) gain the org_id filter:

```rust
// Each sub-query in whats_next() gains the org_id filter.
// Example for open tasks count:
let (org_filter, org_params) = self.org_id_filter(1);
let sql = format!(
    "SELECT COUNT(*) FROM tasks WHERE status IN ('open', 'in_progress') {org_filter}"
);
```

### A8. `test_support` — Update Test Helper (task 9.9b)

The `test_service()` helper must pass `None` for identity (tests run unauthenticated by default):

```rust
pub async fn test_service() -> ZenService {
    let db = ZenDb::open_local(":memory:").await.unwrap();
    let trail = TrailWriter::disabled();
    ZenService::from_db(db, trail, None)  // No identity for tests
}

/// Test service with a specific identity (for visibility tests).
pub async fn test_service_with_identity(identity: AuthIdentity) -> ZenService {
    let db = ZenDb::open_local(":memory:").await.unwrap();
    let trail = TrailWriter::disabled();
    ZenService::from_db(db, trail, Some(identity))
}
```

### A9. `context/app_context.rs` — Pass Identity to ZenService + Store Auth Token (task 9.21a)

Update `AppContext::init()` to pass identity to `ZenService` constructors and store the raw auth token:

```rust
pub struct AppContext {
    pub service: ZenService,
    pub config: ZenConfig,
    pub lake: ZenLake,
    pub source_store: SourceFileStore,
    pub embedder: EmbeddingEngine,
    pub registry: RegistryClient,
    pub project_root: PathBuf,
    pub identity: Option<AuthIdentity>,
    pub auth_token: Option<String>,  // NEW — raw token for Turso/catalog operations
}

// In init():
let (auth_token, identity) = resolve_auth(&config).await;

// Pass identity.clone() to service constructors:
let service = if config.turso.is_configured() {
    // ...
    ZenService::new_synced(replica_path, &config.turso.url, token, Some(trail_dir.clone()), identity.clone())
    // ...
    ZenService::new_local(&db_path_str, Some(trail_dir), identity.clone())
} else {
    ZenService::new_local(&db_path_str, Some(trail_dir), identity.clone())
};

// Store both:
Ok(Self {
    service,
    config,
    lake, source_store, embedder, registry,
    project_root,
    identity,
    auth_token,
})
```

### Tests (Stream A)

- **Migration 003**: `open_local` on fresh DB applies 003 without error; `PRAGMA table_info(findings)` includes `org_id` column
- **Migration 003 idempotent**: Running `run_migrations()` twice doesn't fail (duplicate column errors ignored)
- **Entity create with org_id**: `create_finding()` with identity → finding has `org_id` set; without identity → `org_id` is NULL
- **Entity list filtering**: Service with `org_id="org_abc"` sees entities with `org_id="org_abc"` AND `org_id IS NULL`; does NOT see entities with `org_id="org_xyz"`
- **Entity list no identity**: Service without identity sees only `org_id IS NULL` entities
- **Entity get by ID**: `get_finding(id)` returns entity regardless of org_id (no filtering by ID)
- **whats_next org scoping**: Aggregates respect org_id filter
- **test_service backward compat**: Existing tests pass unchanged (identity=None)

---

## 6. PR 2 — Stream B: Catalog Visibility + R2 Writes + Federated Search

**Tasks**: 9.11 (visibility-scoped catalog), 9.12 (crowdsource dedup), 9.15 (R2 Lance uploads with visibility), 9.16 (federated search)
**Estimated LOC**: ~350 production, ~100 tests

### B1. `zen-db/src/repos/catalog.rs` — Visibility-Scoped Catalog Queries (task 9.11)

Update `register_catalog_data_file()` to accept visibility, org_id, and owner_sub:

```rust
impl ZenService {
    pub async fn register_catalog_data_file(
        &self,
        ecosystem: &str,
        package: &str,
        version: &str,
        lance_path: &str,
        visibility: Visibility,        // NEW
        org_id: Option<&str>,           // NEW
        owner_sub: Option<&str>,        // NEW
    ) -> Result<(), DatabaseError> {
        let now = Utc::now().to_rfc3339();
        let snapshot_id = format!(
            "dls-{}-{}-{}",
            stable_key(ecosystem), stable_key(package), stable_key(version)
        );
        let file_id = self.db().generate_id("dlf").await?;

        self.db().conn().execute(
            "INSERT OR IGNORE INTO dl_snapshot (id, created_at, note) VALUES (?1, ?2, ?3)",
            libsql::params![snapshot_id.as_str(), now.as_str(), "auto"],
        ).await?;

        self.db().conn().execute(
            "INSERT INTO dl_data_file
             (id, snapshot_id, ecosystem, package, version, lance_path, visibility, org_id, owner_sub, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
             ON CONFLICT(ecosystem, package, version, lance_path) DO NOTHING",
            libsql::params![
                file_id.as_str(),
                snapshot_id.as_str(),
                ecosystem, package, version, lance_path,
                visibility.as_str(),
                org_id,
                owner_sub,
                now.as_str()
            ],
        ).await?;

        Ok(())
    }
}
```

Update `catalog_has_package()` to check public entries only (safe default for crowdsource dedup).
Update `catalog_paths_for_package()` to add `AND visibility = 'public'` as a safe default — prevents leaking private/team entries to callers that don't explicitly use the scoped variant. Add new `catalog_paths_for_package_scoped()` for multi-tier visibility:

```rust
impl ZenService {
    pub async fn catalog_has_package(
        &self,
        ecosystem: &str,
        package: &str,
        version: &str,
    ) -> Result<bool, DatabaseError> {
        // Safe default: only check public entries (used for crowdsource dedup).
        let mut rows = self.db().conn().query(
            "SELECT 1 FROM dl_data_file
             WHERE ecosystem = ?1 AND package = ?2 AND version = ?3
               AND lance_path LIKE '%symbols.lance%'
               AND visibility = 'public'
             LIMIT 1",
            libsql::params![ecosystem, package, version],
        ).await?;
        Ok(rows.next().await?.is_some())
    }

    /// Resolve catalog lance paths — **public only** (safe default).
    ///
    /// Use `catalog_paths_for_package_scoped()` for multi-tier visibility.
    pub async fn catalog_paths_for_package(
        &self,
        ecosystem: &str,
        package: &str,
        version: Option<&str>,
    ) -> Result<Vec<String>, DatabaseError> {
        let mut paths = Vec::new();
        let mut rows = if let Some(version) = version {
            self.db().conn().query(
                "SELECT lance_path FROM dl_data_file
                 WHERE ecosystem = ?1 AND package = ?2 AND version = ?3
                   AND visibility = 'public'
                 ORDER BY created_at DESC, id DESC",
                libsql::params![ecosystem, package, version],
            ).await?
        } else {
            self.db().conn().query(
                "SELECT lance_path FROM dl_data_file
                 WHERE ecosystem = ?1 AND package = ?2
                   AND visibility = 'public'
                 ORDER BY created_at DESC, id DESC",
                libsql::params![ecosystem, package],
            ).await?
        };
        while let Some(row) = rows.next().await? {
            paths.push(row.get::<String>(0)?);
        }
        Ok(paths)
    }

    /// Resolve catalog lance paths scoped to the current identity's visibility.
    pub async fn catalog_paths_for_package_scoped(
        &self,
        ecosystem: &str,
        package: &str,
        version: Option<&str>,
    ) -> Result<Vec<String>, DatabaseError> {
        let identity = self.identity();
        let mut paths = Vec::new();

        let (vis_filter, vis_params) = visibility_filter_sql(identity, 4);

        let mut all_params: Vec<libsql::Value> = vec![
            ecosystem.into(),
            package.into(),
        ];

        let version_clause = if let Some(v) = version {
            all_params.push(v.into());
            "AND version = ?3"
        } else {
            ""
        };

        all_params.extend(vis_params);

        let sql = format!(
            "SELECT lance_path FROM dl_data_file
             WHERE ecosystem = ?1 AND package = ?2 {version_clause}
             {vis_filter}
             ORDER BY created_at DESC, id DESC"
        );

        let mut rows = self.db().conn().query(
            &sql, libsql::params_from_iter(all_params)
        ).await?;

        while let Some(row) = rows.next().await? {
            paths.push(row.get::<String>(0)?);
        }
        Ok(paths)
    }
}

/// Build a visibility filter SQL clause.
///
/// When identity is available:
///   `AND (visibility = 'public' OR (visibility = 'team' AND org_id = ?N) OR (visibility = 'private' AND owner_sub = ?N+1))`
/// When identity is None:
///   `AND visibility = 'public'`
fn visibility_filter_sql(
    identity: Option<&AuthIdentity>,
    start_param: u32,
) -> (String, Vec<libsql::Value>) {
    match identity {
        Some(id) => {
            let mut params: Vec<libsql::Value> = Vec::new();
            let mut clauses = vec!["visibility = 'public'".to_string()];

            let mut idx = start_param;
            if let Some(ref org_id) = id.org_id {
                clauses.push(format!("(visibility = 'team' AND org_id = ?{idx})"));
                params.push(org_id.as_str().into());
                idx += 1;
            }

            clauses.push(format!("(visibility = 'private' AND owner_sub = ?{idx})"));
            params.push(id.user_id.as_str().into());

            (format!("AND ({})", clauses.join(" OR ")), params)
        }
        None => ("AND visibility = 'public'".to_string(), vec![]),
    }
}
```

### B2. `zen-db/src/repos/catalog.rs` — Crowdsource Dedup Check (task 9.12)

```rust
impl ZenService {
    /// Check whether a public package is already indexed in the catalog.
    ///
    /// Returns existing Lance paths if found, `None` if not indexed.
    /// Used by `znt install` to skip re-indexing of crowdsourced packages.
    pub async fn catalog_check_before_index(
        &self,
        ecosystem: &str,
        package: &str,
        version: &str,
    ) -> Result<Option<Vec<String>>, DatabaseError> {
        let mut rows = self.db().conn().query(
            "SELECT lance_path FROM dl_data_file
             WHERE ecosystem = ?1 AND package = ?2 AND version = ?3
               AND visibility = 'public'
               AND lance_path LIKE '%symbols.lance%'
             ORDER BY created_at DESC",
            libsql::params![ecosystem, package, version],
        ).await?;

        let mut paths = Vec::new();
        while let Some(row) = rows.next().await? {
            paths.push(row.get::<String>(0)?);
        }

        if paths.is_empty() {
            Ok(None)
        } else {
            Ok(Some(paths))
        }
    }
}
```

### B3. `zen-lake/src/cloud_search.rs` — Visibility-Scoped Discovery (task 9.11, 9.16)

Update `discover_catalog_paths` to accept identity for visibility scoping:

```rust
impl ZenLake {
    /// Discover catalog Lance paths with visibility scoping via a Turso connection.
    ///
    /// Uses the same visibility filter as `catalog_paths_for_package_scoped()`.
    pub async fn discover_catalog_paths_scoped(
        &self,
        turso_url: &str,
        turso_auth_token: &str,
        ecosystem: &str,
        package: &str,
        version: Option<&str>,
        identity: Option<&AuthIdentity>,
    ) -> Result<Vec<String>, LakeError> {
        let db = Builder::new_remote(turso_url.to_string(), turso_auth_token.to_string())
            .build()
            .await?;
        let conn = db.connect()?;
        Self::discover_catalog_paths_scoped_with_conn(
            &conn, ecosystem, package, version, identity
        ).await
    }

    async fn discover_catalog_paths_scoped_with_conn(
        conn: &Connection,
        ecosystem: &str,
        package: &str,
        version: Option<&str>,
        identity: Option<&AuthIdentity>,
    ) -> Result<Vec<String>, LakeError> {
        let mut params: Vec<libsql::Value> = vec![ecosystem.into(), package.into()];
        let mut idx: u32 = 3;

        let version_clause = if let Some(v) = version {
            params.push(v.into());
            idx = 4;
            "AND version = ?3"
        } else {
            ""
        };

        // Build visibility filter
        let vis_clause = match identity {
            Some(id) => {
                let mut clauses = vec!["visibility = 'public'".to_string()];
                if let Some(ref org_id) = id.org_id {
                    clauses.push(format!("(visibility = 'team' AND org_id = ?{idx})"));
                    params.push(org_id.as_str().into());
                    idx += 1;
                }
                clauses.push(format!("(visibility = 'private' AND owner_sub = ?{idx})"));
                params.push(id.user_id.as_str().into());
                format!("AND ({})", clauses.join(" OR "))
            }
            None => "AND visibility = 'public'".to_string(),
        };

        let sql = format!(
            "SELECT lance_path FROM dl_data_file
             WHERE ecosystem = ?1 AND package = ?2 {version_clause}
               AND lance_path LIKE '%symbols.lance%'
             {vis_clause}
             ORDER BY created_at DESC, id DESC"
        );

        let mut paths = Vec::new();
        let mut rows = conn.query(&sql, libsql::params_from_iter(params)).await?;
        while let Some(row) = rows.next().await? {
            paths.push(row.get::<String>(0)?);
        }
        Ok(paths)
    }

    /// Full cloud vector search with visibility scoping.
    pub async fn search_cloud_vector_scoped(
        &self,
        turso_url: &str,
        turso_auth_token: &str,
        ecosystem: &str,
        package: &str,
        version: Option<&str>,
        query_embedding: &[f32],
        k: u32,
        identity: Option<&AuthIdentity>,
    ) -> Result<Vec<CloudVectorSearchResult>, LakeError> {
        let paths = self
            .discover_catalog_paths_scoped(
                turso_url, turso_auth_token, ecosystem, package, version, identity
            )
            .await?;
        self.search_lance_paths(&paths, query_embedding, k)
    }
}
```

**Note**: Existing unscoped methods (`discover_catalog_paths`, `search_cloud_vector`, `search`) are NOT removed — they remain for backward compatibility and tests. New scoped variants are added alongside them. The CLI commands switch to using the scoped variants.

### B4. `zen-lake/src/r2_write.rs` — R2 Writes with Visibility Metadata (task 9.15)

The `write_to_r2()` method itself doesn't need visibility changes — Lance datasets don't contain visibility metadata. Visibility is a catalog-level concern (stored in `dl_data_file`). The change is in the callers: `install.rs` and `index.rs` pass the correct visibility/org_id/owner_sub to `register_catalog_data_file()`.

However, the R2 path structure should encode visibility for operational clarity:

```rust
fn symbols_dataset_root(
    r2: &R2Config,
    ecosystem: &str,
    package: &str,
    version: &str,
    visibility: Visibility,
) -> String {
    let ts = Utc::now().timestamp_millis();
    let vis_prefix = match visibility {
        Visibility::Public => "public",
        Visibility::Team => "team",
        Visibility::Private => "private",
    };
    format!(
        "s3://{}/lance/{}/{}/{}/{}/symbols/{}",
        r2.bucket_name,
        vis_prefix,
        sanitize_segment(ecosystem),
        sanitize_segment(package),
        sanitize_segment(version),
        ts
    )
}
```

Update `write_to_r2()` to accept visibility:

```rust
pub async fn write_to_r2(
    &self,
    r2: &R2Config,
    ecosystem: &str,
    package: &str,
    version: &str,
    visibility: Visibility,  // NEW
) -> Result<R2WriteResult, LakeError> {
    // ... existing validation ...
    let root = symbols_dataset_root(r2, ecosystem, package, version, visibility);
    // ... rest unchanged ...
}
```

### Tests (Stream B)

- **Catalog visibility filter**: With identity `{user_id: "u1", org_id: Some("org_a")}` → sees public + team(org_a) + private(u1). Without identity → sees public only.
- **Catalog dedup**: `catalog_check_before_index()` returns paths when package exists, `None` when not
- **Crowdsource race**: Two concurrent catalog inserts for same package — first wins, second succeeds (ON CONFLICT DO NOTHING)
- **register_catalog_data_file with visibility**: Public entry has `visibility='public'`, team entry has `org_id` set, private entry has `owner_sub` set
- **discover_catalog_paths_scoped**: Identity-based filtering returns correct paths
- **R2 path includes visibility tier**: Public → `s3://bucket/lance/public/...`, private → `s3://bucket/lance/private/...`

---

## 7. PR 3 — Stream C: CLI Team Commands + Private Indexing + Wiring

**Tasks**: 9.21 (team mode startup), 9.22 (znt team invite/list), 9.23 (znt index .)
**Estimated LOC**: ~320 production, ~50 tests

### C1. `zen-auth/src/org.rs` — Clerk Organization API (task 9.22a)

```rust
//! Clerk organization API helpers.
//!
//! Calls the Clerk Backend API directly via `reqwest` (clerk-rs doesn't expose
//! organization management endpoints). Requires `config.clerk.secret_key`.

use serde::{Deserialize, Serialize};
use crate::AuthError;

const CLERK_API_BASE: &str = "https://api.clerk.com/v1";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrgMember {
    pub user_id: String,
    pub role: String,
    pub created_at: String,
    /// Email from `public_user_data`, if available.
    pub email: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrgInvitation {
    pub id: String,
    pub email_address: String,
    pub role: String,
    pub status: String,
}

/// List members of a Clerk organization.
///
/// # Errors
///
/// Returns `AuthError::ClerkApiError` if the API call fails or returns non-200.
pub async fn list_members(
    secret_key: &str,
    org_id: &str,
) -> Result<Vec<OrgMember>, AuthError> {
    let url = format!("{CLERK_API_BASE}/organizations/{org_id}/memberships?limit=100");
    let client = reqwest::Client::new();
    let resp = client
        .get(&url)
        .header("Authorization", format!("Bearer {secret_key}"))
        .send()
        .await
        .map_err(|e| AuthError::ClerkApiError(format!("list members: {e}")))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(AuthError::ClerkApiError(
            format!("list members: HTTP {status}: {body}")
        ));
    }

    #[derive(Deserialize)]
    struct ListResponse {
        data: Vec<MembershipRecord>,
    }
    #[derive(Deserialize)]
    struct MembershipRecord {
        public_user_data: Option<PublicUserData>,
        role: String,
        created_at: i64,
    }
    #[derive(Deserialize)]
    struct PublicUserData {
        user_id: String,
        identifier: Option<String>,
    }

    let list: ListResponse = resp.json().await
        .map_err(|e| AuthError::ClerkApiError(format!("parse members: {e}")))?;

    Ok(list.data.into_iter().filter_map(|m| {
        let pud = m.public_user_data?;
        Some(OrgMember {
            user_id: pud.user_id,
            role: m.role,
            created_at: chrono::DateTime::from_timestamp(m.created_at / 1000, 0)
                .map(|dt| dt.to_rfc3339())
                .unwrap_or_default(),
            email: pud.identifier,
        })
    }).collect())
}

/// Invite a user to a Clerk organization by email.
///
/// # Errors
///
/// Returns `AuthError::ClerkApiError` if the API call fails.
pub async fn invite_member(
    secret_key: &str,
    org_id: &str,
    email: &str,
    role: &str,
) -> Result<OrgInvitation, AuthError> {
    let url = format!("{CLERK_API_BASE}/organizations/{org_id}/invitations");
    let client = reqwest::Client::new();
    let resp = client
        .post(&url)
        .header("Authorization", format!("Bearer {secret_key}"))
        .json(&serde_json::json!({
            "email_address": email,
            "role": role,
        }))
        .send()
        .await
        .map_err(|e| AuthError::ClerkApiError(format!("invite member: {e}")))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(AuthError::ClerkApiError(
            format!("invite member: HTTP {status}: {body}")
        ));
    }

    resp.json().await
        .map_err(|e| AuthError::ClerkApiError(format!("parse invitation: {e}")))
}
```

Update `zen-auth/src/lib.rs` to add `pub mod org;`.

### C2. `zen-cli/src/cli/subcommands/team.rs` — Team Subcommands (task 9.22b)

```rust
use clap::{Args, Subcommand};

/// Team management commands.
#[derive(Clone, Debug, Subcommand)]
pub enum TeamCommands {
    /// Invite a user to the current organization.
    Invite(TeamInviteArgs),
    /// List members of the current organization.
    List,
}

#[derive(Clone, Debug, Args)]
pub struct TeamInviteArgs {
    /// Email address to invite.
    pub email: String,
    /// Role to assign (default: org:member).
    #[arg(long, default_value = "org:member")]
    pub role: String,
}
```

Update `subcommands/mod.rs` to add `pub mod team; pub use team::TeamCommands;`.

### C3. `zen-cli/src/cli/root_commands.rs` — Add Team + Index Variants (task 9.22c)

```rust
// Add to Commands enum:

/// Team management.
Team {
    #[command(subcommand)]
    action: TeamCommands,
},
/// Index the current project for private cloud search.
Index(IndexArgs),

// Add imports:
use crate::cli::subcommands::TeamCommands;

// Add IndexArgs:
/// Arguments for `znt index`.
#[derive(Clone, Debug, Args)]
pub struct IndexArgs {
    /// Path to index (defaults to current project root).
    #[arg(default_value = ".")]
    pub path: String,
    /// Force re-index even if already indexed.
    #[arg(long)]
    pub force: bool,
}
```

### C4. `zen-cli/src/commands/team/mod.rs` — Team Command Dispatch (task 9.22d)

```rust
pub mod invite;
pub mod list;

use crate::cli::GlobalFlags;
use crate::cli::subcommands::team::TeamCommands;
use crate::context::AppContext;

pub async fn handle(
    action: &TeamCommands,
    ctx: &AppContext,
    flags: &GlobalFlags,
) -> anyhow::Result<()> {
    match action {
        TeamCommands::Invite(args) => invite::handle(args, ctx, flags).await,
        TeamCommands::List => list::handle(ctx, flags).await,
    }
}
```

### C5. `zen-cli/src/commands/team/invite.rs` — Team Invite (task 9.22e)

```rust
use anyhow::bail;
use serde::Serialize;

use crate::cli::GlobalFlags;
use crate::cli::subcommands::team::TeamInviteArgs;
use crate::context::AppContext;
use crate::output::output;

#[derive(Debug, Serialize)]
struct InviteResponse {
    email: String,
    role: String,
    status: String,
    invitation_id: String,
}

pub async fn handle(
    args: &TeamInviteArgs,
    ctx: &AppContext,
    flags: &GlobalFlags,
) -> anyhow::Result<()> {
    let identity = ctx.identity.as_ref()
        .ok_or_else(|| anyhow::anyhow!("team invite requires authentication — run `znt auth login`"))?;
    let org_id = identity.org_id.as_deref()
        .ok_or_else(|| anyhow::anyhow!("team invite requires an active organization — run `znt auth switch-org <slug>`"))?;

    if ctx.config.clerk.secret_key.is_empty() {
        bail!("team invite requires ZENITH_CLERK__SECRET_KEY to be configured");
    }

    let invitation = zen_auth::org::invite_member(
        &ctx.config.clerk.secret_key,
        org_id,
        &args.email,
        &args.role,
    ).await?;

    output(
        &InviteResponse {
            email: args.email.clone(),
            role: args.role.clone(),
            status: invitation.status,
            invitation_id: invitation.id,
        },
        flags.format,
    )
}
```

### C6. `zen-cli/src/commands/team/list.rs` — Team List (task 9.22f)

```rust
use anyhow::bail;
use serde::Serialize;

use crate::cli::GlobalFlags;
use crate::context::AppContext;
use crate::output::output;

#[derive(Debug, Serialize)]
struct ListResponse {
    org_id: String,
    org_slug: Option<String>,
    members: Vec<zen_auth::org::OrgMember>,
    count: usize,
}

pub async fn handle(ctx: &AppContext, flags: &GlobalFlags) -> anyhow::Result<()> {
    let identity = ctx.identity.as_ref()
        .ok_or_else(|| anyhow::anyhow!("team list requires authentication — run `znt auth login`"))?;
    let org_id = identity.org_id.as_deref()
        .ok_or_else(|| anyhow::anyhow!("team list requires an active organization — run `znt auth switch-org <slug>`"))?;

    if ctx.config.clerk.secret_key.is_empty() {
        bail!("team list requires ZENITH_CLERK__SECRET_KEY to be configured");
    }

    let members = zen_auth::org::list_members(
        &ctx.config.clerk.secret_key,
        org_id,
    ).await?;

    let count = members.len();
    output(
        &ListResponse {
            org_id: org_id.to_string(),
            org_slug: identity.org_slug.clone(),
            members,
            count,
        },
        flags.format,
    )
}
```

### C7. `zen-cli/src/commands/index.rs` — Private Code Indexing (task 9.23)

```rust
use anyhow::{Context, bail};
use serde::Serialize;
use zen_core::enums::Visibility;

use crate::cli::GlobalFlags;
use crate::cli::root_commands::IndexArgs;
use crate::context::AppContext;
use crate::output::output;
use crate::pipeline::IndexingPipeline;

#[derive(Debug, Serialize)]
struct IndexResponse {
    path: String,
    ecosystem: String,
    package: String,
    version: String,
    visibility: String,
    files_parsed: i32,
    symbols_extracted: i32,
    doc_chunks_created: i32,
    source_files_cached: i32,
    r2_exported: bool,
    catalog_registered: bool,
}

/// Handle `znt index .`.
pub async fn handle(
    args: &IndexArgs,
    ctx: &mut AppContext,
    flags: &GlobalFlags,
) -> anyhow::Result<()> {
    let identity = ctx.identity.as_ref()
        .ok_or_else(|| anyhow::anyhow!(
            "private indexing requires authentication — run `znt auth login`"
        ))?;

    let project_root = if args.path == "." {
        ctx.project_root.clone()
    } else {
        std::path::PathBuf::from(&args.path)
            .canonicalize()
            .context("failed to resolve index path")?
    };

    // Derive package name from project metadata or directory name.
    let package = ctx.service.get_meta("name").await?
        .or_else(|| project_root.file_name().map(|n| n.to_string_lossy().to_string()))
        .unwrap_or_else(|| "unnamed".to_string());

    let ecosystem = "local".to_string();
    let version = chrono::Utc::now().format("%Y%m%d").to_string();

    // Index using existing pipeline.
    let index = IndexingPipeline::index_directory_with(
        &ctx.lake,
        &ctx.source_store,
        &project_root,
        &ecosystem,
        &package,
        &version,
        &mut ctx.embedder,
        true, // skip tests
    ).context("indexing pipeline failed")?;

    // Export to R2 if configured.
    let mut r2_exported = false;
    let mut catalog_registered = false;

    if ctx.config.r2.is_configured() {
        match ctx.lake.write_to_r2(
            &ctx.config.r2, &ecosystem, &package, &version, Visibility::Private,
        ).await {
            Ok(export) => {
                r2_exported = true;
                if let Some(symbols_path) = export.symbols_lance_path.as_deref() {
                    if ctx.service.is_synced_replica() {
                        match ctx.service.register_catalog_data_file(
                            &ecosystem, &package, &version, symbols_path,
                            Visibility::Private,
                            None,                        // org_id: None for private
                            Some(&identity.user_id),     // owner_sub: user_id
                        ).await {
                            Ok(()) => catalog_registered = true,
                            Err(error) => tracing::warn!(
                                %error,
                                "index: failed to register private dataset in catalog"
                            ),
                        }
                    }
                }
            }
            Err(error) => tracing::warn!(
                %error,
                "index: failed to export private dataset to R2"
            ),
        }
    }

    output(
        &IndexResponse {
            path: project_root.to_string_lossy().to_string(),
            ecosystem,
            package,
            version,
            visibility: "private".to_string(),
            files_parsed: index.files_parsed,
            symbols_extracted: index.symbols_extracted,
            doc_chunks_created: index.doc_chunks_created,
            source_files_cached: index.source_files_cached,
            r2_exported,
            catalog_registered,
        },
        flags.format,
    )
}
```

### C8. `zen-cli/src/commands/install.rs` — Pass Visibility + Identity (task 9.21b)

Update install.rs to:
1. Check catalog dedup before indexing (crowdsource)
2. Pass `Visibility::Public` and owner metadata to catalog registration
3. Use resolved auth token for cloud search

```rust
// In handle(), before the indexing pipeline:
// NEW: Check catalog for existing public entries (crowdsource dedup).
if ctx.service.is_synced_replica() && !args.force {
    if let Ok(Some(existing_paths)) = ctx
        .service
        .catalog_check_before_index(&ecosystem, &args.package, &version)
        .await
    {
        tracing::info!(
            ecosystem = %ecosystem,
            package = %args.package,
            version = %version,
            paths = ?existing_paths,
            "install: package already indexed in cloud catalog; skipping re-index"
        );
        // Still index locally for fast search, but skip R2 export + catalog write.
        // ... proceed with local indexing only ...
    }
}

// In the R2 export + catalog registration section:
// BEFORE:
ctx.service.register_catalog_data_file(&ecosystem, &args.package, &version, symbols_path).await
// AFTER:
let owner_sub = ctx.identity.as_ref().map(|i| i.user_id.as_str());
ctx.service.register_catalog_data_file(
    &ecosystem, &args.package, &version, symbols_path,
    Visibility::Public,
    None,        // org_id: None for public packages
    owner_sub,   // owner_sub: user who indexed it
).await
```

### C9. `zen-cli/src/commands/search.rs` — Use Resolved Auth Token + Identity (task 9.21c)

```rust
// In try_cloud_vector_search():
// BEFORE:
let cloud = ctx.lake.search_cloud_vector(
    &ctx.config.turso.url,
    &ctx.config.turso.auth_token,  // tier 4 legacy token
    ecosystem, package, args.version.as_deref(),
    &query_embedding, limit,
).await;

// AFTER:
let auth_token = ctx.auth_token.as_deref()
    .unwrap_or(&ctx.config.turso.auth_token);
let cloud = ctx.lake.search_cloud_vector_scoped(
    &ctx.config.turso.url,
    auth_token,
    ecosystem, package, args.version.as_deref(),
    &query_embedding, limit,
    ctx.identity.as_ref(),
).await;
```

### C10. `zen-cli/src/commands/dispatch.rs` — Add Team + Index Dispatch (task 9.21d)

```rust
// Add to match arms:
Commands::Team { action } => commands::team::handle(&action, ctx, flags).await,
Commands::Index(args) => commands::index::handle(&args, ctx, flags).await,
```

### Tests (Stream C)

- **Team invite requires auth**: `znt team invite` without identity → error message
- **Team invite requires org**: `znt team invite` with identity but no org_id → error message
- **Team list requires auth**: `znt team list` without identity → error message
- **Index requires auth**: `znt index .` without identity → error message
- **Install crowdsource dedup**: Mock catalog with existing package → skip R2 export
- **Search uses auth token**: Verify `try_cloud_vector_search` uses `ctx.auth_token`

---

## 8. Execution Order

```
PR 1 (Stream A):
  1. Add Visibility enum to zen-core/src/enums.rs
  2. Create zen-db/migrations/003_team.sql
  3. Update zen-db/src/migrations.rs — add MIGRATION_003 with per-statement error handling
  4. Add identity field + helpers (org_id, user_id, org_id_filter) to ZenService
  5. Update ZenService constructors (new_local, new_synced, from_db) — add identity param
  6. Update AppContext::init() — pass identity to service, add auth_token field
  7. Update test_support::test_service() — pass None for identity
  7b. Update all other ZenService::new_local / from_db callers — pass None for identity:
      - zen-cli: init.rs, rebuild/handle.rs, hook/rebuild_trigger.rs
      - zen-search: graph.rs (make_service test helper), lib.rs (test helper)
      - zen-db: tests/pr1_infrastructure.rs (test_service, test_service_with_trail, service_new_local)
  8. Update all 10 entity repo create methods — add org_id to INSERT
  9. Update all entity repo list/search methods — add org_id_filter to WHERE
  10. Update whats_next.rs aggregate queries — add org_id_filter
  11. Verify: cargo build -p zen-core -p zen-db -p zen-cli
  12. Verify: cargo test -p zen-db (all existing tests pass with identity=None)

PR 2 (Stream B):
  13. Update register_catalog_data_file() — add visibility, org_id, owner_sub params
  14. Add catalog_check_before_index() for crowdsource dedup
  15. Add catalog_paths_for_package_scoped() with visibility filter
  16. Add visibility_filter_sql() helper function
  17. Add discover_catalog_paths_scoped() to zen-lake cloud_search.rs
  18. Add search_cloud_vector_scoped() to zen-lake cloud_search.rs
  19. Update write_to_r2() — accept Visibility, encode in R2 path prefix
  20. Update all callers of register_catalog_data_file — pass visibility params
  21. Verify: cargo build -p zen-lake -p zen-db
  22. Verify: cargo test -p zen-lake -p zen-db

PR 3 (Stream C):
  23. Create zen-auth/src/org.rs (list_members, invite_member)
  24. Update zen-auth/src/lib.rs — add pub mod org
  25. Create zen-cli/src/cli/subcommands/team.rs (TeamCommands)
  26. Create zen-cli/src/commands/team/ (mod, invite, list)
  27. Create zen-cli/src/commands/index.rs (znt index .)
  28. Add Team + Index to Commands enum in root_commands.rs
  29. Add Team + Index to dispatch.rs
  30. Update install.rs — crowdsource dedup check + visibility params
  31. Update search.rs — use ctx.auth_token + identity for scoped search
  32. Verify: cargo build -p zen-auth -p zen-cli
  33. Verify: cargo test --workspace
```

---

## 9. Gotchas & Warnings

### 9.1 SQLite ALTER TABLE ADD COLUMN Has No IF NOT EXISTS

**Problem**: `ALTER TABLE t ADD COLUMN c TEXT` fails with "duplicate column name: c" if the column already exists. Unlike `CREATE TABLE IF NOT EXISTS`, there is no idempotent variant.

**Impact**: Migration 003_team.sql cannot be safely re-run via `execute_batch()`. Running it on a database that already has the `org_id` columns (e.g., app restart) will fail.

**Resolution**: Execute each `ALTER TABLE` statement individually and catch "duplicate column name" errors. See §A3 for implementation. This is the standard pattern for SQLite schema migrations.

### 9.2 Entity Get-By-ID Does NOT Filter by Org_id

**Problem**: `get_finding(id)` returns the entity regardless of org_id. An unauthenticated user who knows a finding ID can read it.

**Impact**: IDs are 8-char random hex (`fnd-a3f8b2c1`), so brute-forcing is impractical. Entity links reference by ID across sessions and potentially across orgs. Adding org_id filtering to get-by-ID would break entity link resolution.

**Resolution**: Intentional design — IDs are unforgeable and entity links cross org boundaries. List/search methods have org_id scoping; get-by-ID does not. This is consistent with how GitHub handles issue numbers (public URL if you know it, but not enumerable without access).

### 9.3 Pre-Auth Entities Remain Visible After Auth

**Problem**: Entities created before authentication have `org_id IS NULL`. After the user authenticates and joins an org, the filter `AND (org_id = ? OR org_id IS NULL)` returns both team entities AND pre-auth local entities. This means the user sees their old local findings alongside team findings.

**Impact**: This is intentional — pre-auth work shouldn't disappear when the user authenticates. However, it means NULL-org entities from OTHER users on the same machine (if any) would also be visible. This is unlikely in practice (single-user CLI tool).

**Resolution**: Accepted behavior. A future "claim entities" command could migrate `org_id IS NULL` entities to the user's org.

### 9.4 Catalog Visibility Filter Uses Dynamic SQL

**Problem**: `visibility_filter_sql()` builds SQL dynamically with parameter indices. The number of parameters varies based on whether the identity has an org_id. This makes the code more complex than static SQL.

**Impact**: Parameter index calculation must be correct for each callsite. Off-by-one errors would cause SQL parameter binding failures (immediately caught by tests).

**Resolution**: The helper function returns both the SQL fragment and the corresponding parameter values. Callers extend their param vec with the returned params. All variants are tested.

### 9.5 `znt index .` Uses `'local'` Ecosystem — Search Requires Explicit Filters

**Problem**: Private code indexed via `znt index .` uses `ecosystem='local'` and a date-based version (`20260220`). This doesn't match any registry package. The `IndexingPipeline` was designed for registry packages with proper version strings.

**Impact**: The pipeline works fine with any ecosystem/version strings — they're just identifiers. But `znt search --ecosystem rust --package tokio` won't find locally indexed code. **Critically, `search.rs` `try_cloud_vector_search()` returns `Ok(None)` immediately when `--ecosystem` or `--package` are omitted** (lines 241–244), so users **cannot** "omit filters for a broad search" — cloud search is skipped entirely without both arguments. Users must explicitly provide `--ecosystem local --package <project-name>` to search their private indexed code.

**Resolution**: The `znt index .` output must clearly show the ecosystem and package name used, e.g., `"Indexed as local/<project-name>. Search with: znt search --ecosystem local --package <project-name> <query>"`. Future enhancement: update `try_cloud_vector_search()` to allow filterless broad search across all visibility tiers (requires querying the catalog for all packages visible to the identity, then searching each).

### 9.6 Clerk Backend API Rate Limits

**Problem**: Clerk rate-limits Backend API calls. `znt team list` and `znt team invite` call the Clerk API directly. Rapid repeated calls could trigger rate limiting (429 responses).

**Impact**: Team commands are interactive (human-speed), so rate limiting is unlikely in practice. Automated scripts calling `znt team invite` in a loop could hit limits.

**Resolution**: Parse 429 responses and return a clear error message: "Clerk API rate limited — try again in a few seconds". No automatic retry for MVP.

### 9.7 `register_catalog_data_file` Signature Change Is Breaking

**Problem**: Adding `visibility`, `org_id`, `owner_sub` parameters to `register_catalog_data_file()` changes the method signature. All callers must be updated simultaneously.

**Impact**: Only two callers exist: `install.rs` (public indexing) and the new `index.rs` (private indexing). The existing test in `catalog.rs` also needs updating.

**Resolution**: Update all callers in PR 2. The test uses `Visibility::Public, None, None` to match the previous hardcoded behavior.

### 9.8 `auth_token` and `identity` Have Different Lifetimes

**Problem**: `auth_token` is the raw JWT string. `identity` is the parsed claims. They come from the same `resolve_auth()` call but are stored as separate fields in AppContext. If one is stale, the other may not be.

**Impact**: The token could expire mid-session while the identity struct still shows valid claims. This is the same issue Phase 8 documented (libsql doesn't support hot-swap). `rebuild_synced()` exists for recovery.

**Resolution**: Both are set once at startup and are immutable for the lifetime of a CLI command (single-shot execution). For long-running operations, the Phase 8 expiry detection + `rebuild_synced()` path handles refresh.

### 9.9 Team Visibility Requires Both org_id in JWT AND Turso Sync

**Problem**: Team visibility only works when: (1) the user has `org_id` in their Clerk JWT, AND (2) the database is synced via Turso (so team members share data). Local-only mode with an org JWT provides identity but no data sharing.

**Impact**: A user who authenticates but doesn't configure Turso gets identity without team features. This is the expected "free tier" behavior — team features require cloud sync.

**Resolution**: `znt auth status` should indicate whether team mode is active (identity + sync). When identity has org_id but service is not synced, emit `tracing::info!` noting that team features require Turso configuration.

---

## 10. Milestone 9 Validation

### Validation Command

```bash
cargo test --workspace
```

### Acceptance Criteria

| # | Criterion | Validated By |
|---|-----------|-------------|
| M9.1 | Migration 003 applies `org_id` column to all 10 entity tables | `PRAGMA table_info` test |
| M9.2 | Migration 003 is idempotent (re-run doesn't fail) | Unit test |
| M9.3 | Entity creation writes `org_id` from identity | Unit test |
| M9.4 | Entity list/search respects org_id filter | Unit test |
| M9.5 | Entities with `org_id IS NULL` visible to all (backward compat) | Unit test |
| M9.6 | Entities with `org_id='org_a'` NOT visible to identity with `org_id='org_b'` | Unit test |
| M9.7 | `register_catalog_data_file()` writes correct visibility/org_id/owner_sub | Unit test |
| M9.8 | `catalog_paths_for_package_scoped()` returns visibility-filtered paths | Unit test |
| M9.9 | Public catalog entries visible to unauthenticated users | Unit test |
| M9.10 | Team catalog entries visible only to same-org members | Unit test |
| M9.11 | Private catalog entries visible only to owner | Unit test |
| M9.12 | `catalog_check_before_index()` detects existing public packages | Unit test |
| M9.13 | `znt team invite` requires auth + org_id + secret_key | CLI error test |
| M9.14 | `znt team list` requires auth + org_id + secret_key | CLI error test |
| M9.15 | `znt index .` requires auth | CLI error test |
| M9.16 | `znt index .` registers catalog with `visibility='private'` | Integration flow |
| M9.17 | `znt install` uses crowdsource dedup when synced | Integration flow |
| M9.18 | `znt search` uses resolved auth token (not config token) | Code inspection + test |
| M9.19 | R2 path includes visibility prefix (`public/`, `private/`, `team/`) | Unit test |
| M9.20 | All existing Phase 2–8 tests pass with `identity=None` | `cargo test --workspace` |
| M9.21 | `whats_next()` respects org_id scoping | Unit test |

---

## 11. Validation Traceability Matrix

Every implementation decision traces back to a spike test, upstream validated API, or established convention.

| Implementation | Validated By | Evidence |
|----------------|-------------|----------|
| `dl_data_file` visibility/org_id/owner_sub columns | 002_catalog.sql | Already exists in production schema |
| `ON CONFLICT DO NOTHING` dedup | catalog.rs | Already used in `register_catalog_data_file()` |
| `ux_dl_data_file_triplet_path` unique index | 002_catalog.sql | Already exists — dedup constraint |
| Visibility-scoped catalog SQL | Spike 0.20 J3 | `test_clerk_jwt_visibility_scoping` — real org_id filtering |
| Concurrent dedup first-writer-wins | Spike 0.20 L1 | `test_concurrent_dedup_first_writer_wins` — SQLITE_CONSTRAINT handled |
| Private code isolation | Spike 0.20 K2 | `test_private_code_isolation` — only owner discovers |
| Three-tier search (public + team visible) | Spike 0.20 K1 | `test_three_tier_search` — merged results |
| Turso accepts Clerk JWT for catalog ops | Spike 0.17 | `test_turso_jwks_accepts_clerk_jwt` |
| Embedded replica sync with Clerk JWT | Spike 0.17 | `test_turso_embedded_replica_with_clerk_jwt` |
| `lancedb` R2 writes via serde_arrow | Spike 0.19 M1 | Full roundtrip with `serde_arrow::to_record_batch()` |
| `lance_vector_search()` on R2 datasets | Spike 0.18 | `test_lance_vector_search_on_r2` |
| `Option<T>` in libsql `params!` macro | Spike 0.2g | Maps to SQL NULL natively |
| `AppContext.identity` populated from Phase 8 | Phase 8 | `resolve_auth()` → `claims.to_identity()` |
| Pre-context dispatch pattern | Phase 5/8 | Init, Hook, Schema, Auth dispatched before `AppContext::init()` |
| `output()` formatter for responses | Phase 5 | `crate::output::output` — JSON/table/raw |
| Graceful degradation to local mode | Phase 5/7/8 | Auth failure → local-only, warn but don't block |
| Clerk Backend API organization endpoints | Spike 0.20 J0 | `test_programmatic_org_scoped_jwt` — org API works |
| `reqwest` for Clerk API calls | zen-auth/api_key.rs | Same pattern as Phase 8 API key flow |
| `org_permissions: []` in JWT template | Spike 0.20 | Shortcode doesn't resolve; static empty array required |
| SQLite ALTER TABLE no IF NOT EXISTS | SQLite docs | Standard limitation; per-statement error handling required |
| `ZenService` field pattern | Phase 2 | `db`, `trail`, `schema` fields set at construction |
| Identity immutable for command lifetime | Phase 8 | CLI is single-shot; token/identity set once |

---

## 12. Plan Review — Mismatch Log

*To be filled after internal and oracle review. Follows the same categorized format as Phase 8 §12.*

### Categories

- **F** = Factual (code listing disagrees with actual source)
- **E** = Editorial (prose is imprecise or ambiguous)
- **S** = Semantic (architectural/behavioral claim is misleading or overstated)

### Round 1 — Pre-Implementation Review (plan vs. codebase audit)

| # | Category | Description | Severity | Resolution |
|---|----------|-------------|----------|------------|
| F1 | F | §3.2 visibility SQL example used `?auth_org_id` / `?auth_user_id` — invalid SQLite placeholder syntax (SQLite uses `?NNN` numeric or `:name` named params) | **High** | ✅ Fixed: Changed to `?1` / `?2` with comment explaining parameter mapping |
| S1 | S | §A3 migration runner used `MIGRATION_003.lines()` — breaks if any SQL statement spans multiple lines (e.g., CREATE INDEX) | **High** | ✅ Fixed: Changed to `MIGRATION_003.split(';')` with comment-stripping. Handles multi-line statements correctly |
| F2 | F | "Depends on" phrasing implied `write_to_r2()` and `search_cloud_vector()` already support visibility — they don't, that's what Phase 9 adds | **Medium** | ✅ Fixed: Clarified "basic `write_to_r2()` for public packages" and "without visibility scoping" |
| S2 | S | `org_id_filter(param_index)` approach requires every query to correctly compute `param_index` — off-by-one bugs possible across 13 repo modules | **Medium** | Accepted: Each usage is tested. The alternative (named params like `:org_id`) is cleaner but changes the parameterization style used throughout the codebase. Current codebase consistently uses `?N` style. Implementers should verify param index matches for each query. |
| S3 | S | Get-by-ID without org_id filtering means any ID holder can read cross-org entities. IDs are 8-char random hex (4 bytes entropy = ~4B combinations), not brute-forceable, but leaked IDs bypass org scoping | Low | Accepted: Intentional design — entity links reference by ID cross-org. Documented in Gotcha 9.2. |
| E1 | E | Plan should explicitly note that `catalog.rs` tests (3 existing tests) must be updated when `register_catalog_data_file()` signature changes | Low | ✅ Noted in Gotcha 9.7 |

### Round 2 — Code Review Pass (code-review skill)

| # | Category | Description | Severity | Resolution |
|---|----------|-------------|----------|------------|
| F3 | F | Plan listed "Crates touched" as zen-core, zen-db, zen-lake, zen-cli, zen-auth but **missed 8 call sites** of `ZenService::new_local`/`from_db` that break on signature change: `init.rs`, `rebuild/handle.rs`, `hook/rebuild_trigger.rs` (all zen-cli), `graph.rs`, `lib.rs` (zen-search), `test_support.rs` (zen-db internal), and `tests/pr1_infrastructure.rs` (zen-db integration tests) | **Critical** | ✅ Fixed: Added all missing files to Module Structure §4, added step 7b to Execution Order listing all callers needing `None` identity |
| S4 | S | §3.4 said catalog methods gain identity param (breaking change) but §B3 introduced new `_scoped` methods while leaving originals unchanged — originals returning ALL entries (including private) is a security footgun if used accidentally | **High** | ✅ Fixed: §3.4 now specifies originals are updated to default to `visibility = 'public'` only. New `catalog_paths_for_package()` in §B1 adds `AND visibility = 'public'`. Scoped variants required for multi-tier |
| F4 | F | Gotcha 9.5 claimed users can "omit filters for a broad search" but `search.rs` `try_cloud_vector_search()` returns `Ok(None)` immediately when ecosystem/package args are missing (lines 241–244) — cloud search is skipped entirely | **Medium** | ✅ Fixed: Gotcha 9.5 now states explicit `--ecosystem local --package <name>` is required; `znt index .` output must show the search command |

---

## Cross-References

- Architecture overview: [03-architecture-overview.md](./03-architecture-overview.md)
- Data architecture: [02-data-architecture.md](./02-data-architecture.md)
- CLI API design: [04-cli-api-design.md](./04-cli-api-design.md)
- Crate designs: [05-crate-designs.md](./05-crate-designs.md)
- Implementation plan: [07-implementation-plan.md](./07-implementation-plan.md) (§11 Phase 9)
- Phase 8 plan (format reference + dependency): [28-phase8-authentication-identity-plan.md](./28-phase8-authentication-identity-plan.md)
- Clerk auth spike: [15-clerk-auth-turso-jwks-spike-plan.md](./15-clerk-auth-turso-jwks-spike-plan.md)
- Catalog visibility spike: [18-catalog-visibility-spike-plan.md](./18-catalog-visibility-spike-plan.md)
- R2 Lance export spike: [16-r2-parquet-export-spike-plan.md](./16-r2-parquet-export-spike-plan.md)
- Native Lance spike: [17-native-lance-spike-plan.md](./17-native-lance-spike-plan.md)
- Spike 0.17 source: `crates/zen-db/src/spike_clerk_auth.rs`
- Spike 0.20 source: `crates/zen-db/src/spike_catalog_visibility.rs`
- Spike 0.18 source: `crates/zen-lake/src/spike_r2_parquet.rs`
- Spike 0.19 source: `crates/zen-lake/src/spike_native_lance.rs`
