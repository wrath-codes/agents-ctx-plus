# Zenith: Implementation Plan

**Version**: 2026-02-08
**Status**: Active Planning Document
**Purpose**: Phased implementation roadmap with milestones, dependencies, validation criteria, and risk mitigations

---

## Table of Contents

1. [Principles](#1-principles)
2. [Phase 0: Workspace Setup & Dependency Validation](#2-phase-0-workspace-setup--dependency-validation)
3. [Phase 1: Foundation](#3-phase-1-foundation)
4. [Phase 2: Storage Layer](#4-phase-2-storage-layer)
5. [Phase 3: Parsing & Indexing Pipeline](#5-phase-3-parsing--indexing-pipeline)
6. [Phase 4: Search & Registry](#6-phase-4-search--registry)
7. [Phase 5: CLI Shell](#7-phase-5-cli-shell)
8. [Phase 6: PRD Workflow](#8-phase-6-prd-workflow)
9. [Phase 7: AgentFS Integration](#9-phase-7-agentfs-integration)
10. [Phase 8: Cloud & Polish](#10-phase-8-cloud--polish)
11. [Phase 9: Team & Pro](#11-phase-9-team--pro)
12. [Dependency Graph](#12-dependency-graph)
13. [Risk Register](#13-risk-register)
14. [Validation Checkpoints](#14-validation-checkpoints)

---

## 1. Principles

- **Validate risky dependencies early** (Phase 0) -- AgentFS, DuckDB+Lance, fastembed, Turso+Clerk
- **Working CLI at every phase** -- after Phase 5 we have a usable tool, everything after is enhancement
- **Tests at every step** -- no moving to the next phase without tests passing
- **Each phase produces a milestone** -- a commit that compiles, tests pass, and does something demonstrably useful
- **Reference implementations consulted** -- aether patterns for storage, klaw patterns for parsing, ai-dev-tasks for PRD workflow

---

## 2. Phase 0: Workspace Setup & Dependency Validation

**Goal**: Prove that all risky dependencies compile and work together before writing any application code.

### Tasks

| ID | Task | Validates | Blocks |
|----|------|-----------|--------|
| 0.1 | Create Cargo workspace with all 11 crate stubs (9 original + zen-hooks + zen-schema) | Rust 2024 edition, workspace structure | Everything |
| 0.2 | ~~Add `turso` crate~~ → Add `libsql` crate, write spike: create local DB, execute SQL, query rows, FTS5 | **DONE** — libsql 0.9.29 works locally (turso crate FTS blocked) | Phase 1 |
| 0.3 | ~~Add `libsql` embedded replica spike: connect to Turso Cloud, sync~~ | **DONE** — `Builder::new_remote_replica()` + `db.sync().await` works. Validated: connect, write-forward, two-replica roundtrip, FTS5 through replica, transactions, deferred batch sync. Requires `tokio multi_thread` runtime. | Phase 8 |
| 0.4 | ~~Add `duckdb` crate (bundled), write spike: create table, insert, query~~ | **DONE** — `duckdb` 1.4 (bundled) compiles and works. Validated: CRUD, Appender bulk insert (1000 rows), transactions, JSON columns, `FLOAT[384]` arrays with `array_cosine_similarity()`, `execute_batch`, file persistence. DuckDB is synchronous; async strategy documented (prefer `spawn_blocking`, `async-duckdb` as alternative). `FLOAT[N]` enforces dimension at insert time. | Phase 2 |
| 0.5 | ~~Add `duckdb` VSS extension spike: create HNSW index, vector search~~ | **DONE** — Full stack validated: (1) VSS HNSW works in-memory but crashes on persistence (DuckDB 1.4 bug). (2) MotherDuck cloud works (`md:` protocol). (3) R2 Parquet roundtrip works (`httpfs`). (4) **Lance community extension validated** as superior alternative: `lance_vector_search()`, `lance_fts()` (BM25), `lance_hybrid_search()` all work locally and on R2 (`s3://`). Lance has persistent vector indexes, no HNSW crash. **Decision**: Use Lance format for documentation lake instead of Parquet + HNSW. See `02-ducklake-data-model.md` §10. **Gotchas**: Lance FTS is term-exact (no stemming). Lance uses AWS env vars for S3 creds, not DuckDB `CREATE SECRET`. | Phase 4 |
| 0.6 | ~~Add `fastembed` crate, write spike: embed text, verify 384 dimensions~~ | **DONE** — fastembed 5.8.1 works locally. Validated: `BGESmallENV15` (default, CLS pooling, 384-dim, ~100MB) and `AllMiniLML6V2` (design model, Mean pooling, 384-dim, ~80MB). Both produce correct 384-dim vectors. Confirmed: single/batch embed, determinism, cosine similarity sanity (similar texts cluster, dissimilar don't), query/passage prefix behavior (BGE), edge cases (empty/short/long text), batch size control. API is synchronous (`&mut self`); use `spawn_blocking` from async code. Models cache to `~/.zenith/cache/fastembed/`. **Gotcha**: fastembed default cache is `.fastembed_cache` (relative CWD) — use `with_cache_dir()` for stable path. `embed()` takes `&mut self`, not `&self`. Dynamic quantized models reject sub-total batch sizes. | Phase 3 |
| 0.7 | ~~Add `agentfs` from git~~ → Add `agentfs-sdk` from crates.io, write spike: KV CRUD, filesystem ops, tool tracking | **DONE** — `agentfs-sdk` 0.6.0 works (crates.io, not git). Validated: ephemeral + persistent modes, KV (set/get/delete/keys, serde structs), filesystem (mkdir/create_file/pwrite/read_file/stat/remove), tool tracking (start/success/error, record, recent, stats). **Note**: Turso docs say `agentfs = "0.1"` but correct crate is `agentfs-sdk`; docs show simplified API that doesn't match v0.6.0 (POSIX-level FS, `&V` refs for KV, positional args for tools). Task 0.10 (fallback) not needed. | Phase 7 |
| 0.8 | ~~Add `ast-grep-core` + `ast-grep-language`, write spike: parse Rust file, pattern match, walk AST nodes~~ | **DONE** — `ast-grep-core` 0.40.5 + `ast-grep-language` 0.40.5 work. Validated 19 tests across 7 sections: (1) Core parsing works for all 7 rich languages + all 26 built-in grammars. (2) Pattern matching with metavariables works (`$NAME` single, `$$$PARAMS` multi). (3) `KindMatcher` + `Any`/`All`/`Not` composable matchers work. (4) Node traversal: `field("name")`, `field("parameters")`, `field("return_type")`, `field("body")`, `field("trait")` for impl discrimination, `prev()` sibling walking for doc comments, `children()` for enum variants/struct fields/methods, `parent()`/`ancestors()` for nesting detection. (5) Position: `start_pos().line()` zero-based, `column()` takes `&Node` arg (O(n)). (6) Raw tree-sitter fallback via `LanguageExt::get_ts_language()` + `Query`/`QueryCursor` works (uses `StreamingIterator`). **Key findings**: (a) Pattern matching is fragile for Rust — `fn $NAME() { $$$ }` does NOT match functions with return types or generics; **use `KindMatcher` as primary extraction strategy** (klaw approach), patterns only for specific structural queries. (b) `async`/`unsafe` appear as children of `function_modifiers` node, not as direct children — walk into modifiers for detection. (c) `All::new()` requires homogeneous matcher types; use `ops::Op` for mixed types. (d) `get_match()` returns `None` for `$$$` multi-metavars — must use `get_multiple_matches()`. (e) `Position::column()` requires `&Node` argument unlike `line()`. (f) `text()`/`kind()` return `Cow<str>`. (g) Smart strictness only matches `fn foo()` (not `pub fn` or `pub async fn`) — confirms KindMatcher-first approach. (h) `tree-sitter` 0.26 `QueryMatches` uses `StreamingIterator`, not `Iterator`. | Phase 3 |
| 0.9 | ~~Add `clap` derive, write spike: parse subcommands, output JSON~~ | **DONE** — clap 4.5 derive works. Validated: `Parser`/`Subcommand`/`ValueEnum` derive macros, global flags with `global = true` (work before AND after subcommand), `OutputFormat` enum restricting `--format` to json/table/raw, nested two-level subcommands (`znt finding create`), positional + optional arg mixing, default values, error rejection for missing args and unknown subcommands, JSON serialization of response structs via serde. Representative subset covers all patterns needed for the full 16-domain command tree. No gotchas found — clap derive works exactly as documented. | Phase 5 |
| 0.10 | ~~If 0.7 fails: design `Workspace` trait, implement `TempDirWorkspace` fallback~~ | **CANCELLED** — 0.7 passed, AgentFS works from crates.io | N/A |
| 0.11 | ~~Write studies feature spike: test Approach A vs Approach B~~ | **DONE** — Approach B (hybrid) selected. One new `studies` table + reuse existing entities. 15/15 tests pass. Type-safe filtering, purpose-built fields (`topic`, `library`, `methodology`), dedicated lifecycle. See [08-studies-spike-plan.md](./08-studies-spike-plan.md) | Phase 2 (StudyRepo), Phase 5 (CLI) |
| 0.12 | ~~Write JSONL trail spike: test Approach A (export only) vs Approach B (source of truth), evaluate `serde-jsonlines` crate~~ | **DONE** — Approach B (source of truth) selected. 15/15 tests pass. DB is rebuildable from JSONL (FTS5 + entity_links survive). `serde-jsonlines` confirmed (1-line batch read/write/append). Per-session files concurrent-safe (4 agents, 100 ops). Replay logic ~60 LOC. See [10-git-jsonl-strategy.md](./10-git-jsonl-strategy.md) | Phase 2 (JSONL writer + replayer), Phase 5 (`znt rebuild` CLI) |
| 0.13 | ~~Write git hooks spike: test hook implementation, installation strategy, post-checkout rebuild, `gix` for repo discovery + config + index + tree diff + session tags~~ | **DONE** — 22/22 tests pass. Decisions: (1) Hook implementation: thin shell wrapper calling `znt hook` (Rust validation via `serde_json` + `jsonschema` for schema enforcement, graceful skip if `znt` not in PATH). (2) Installation: symlink for MVP (coexists with existing hooks). (3) Post-checkout: threshold-based auto-rebuild (JSONL parse <25ms for 5K ops). (4) `gix` 0.70 adopted with `max-performance-safe` + `index` + `blob-diff`. (5) Session tags: adopt lightweight `zenith/ses-xxx` tags. (6) CLI renamed from `zen` to `znt` (zen-browser collision). **Gotchas**: `gix` `MustNotExist` doesn't reject duplicate refs — use `find_reference()` first; `config_snapshot_mut()` is in-memory only — `forget()` + `write_to()` to persist; `jq` not default-installed — Rust is the only reliable JSON validation path. See [11-git-hooks-spike-plan.md](./11-git-hooks-spike-plan.md) | Phase 5 (tasks 5.18a-e), session-git integration |
| 0.14 | ~~Write zen grep spike: validate `grep` crate (ripgrep library), `ignore` crate (gitignore-aware walking), DuckDB `source_files` table, symbol correlation~~ | **DONE** — 26/26 tests pass. Validated: (1) `grep` 0.4 — `RegexMatcher` compiles patterns with flags (case-insensitive, word, literal, smart-case), `Searcher` + `UTF8` sink with line numbers, custom `Sink` for context lines, binary detection, `search_path` for files. (2) `ignore` 0.4 — `WalkBuilder` respects `.gitignore`, override globs for include/exclude, `filter_entry` for test file skipping, custom ignore filename (`.zenithignore`), hidden file skipping, combined grep+ignore workflow. (3) DuckDB `source_files` table — CRUD, Appender bulk insert, `regexp_matches()` with flags, `string_split()`+`unnest()` line-level grep with line numbers, language filtering, cache management (DELETE, stats). (4) Symbol correlation — `idx_symbols_file_lines` composite index, batch symbol lookup per file, binary search matches line→symbol range, `SymbolRef` population with all fields (id, kind, name, signature). (5) Combined pipeline — store source during indexing → grep with `RegexMatcher`+`Searcher` over stored content → correlate with `api_symbols` → `CorrelatedHit` with all fields validated. **Key findings**: (a) DuckDB fetch + Rust regex is faster than SQL-level line splitting; use DuckDB as compressed storage, Rust for line matching. (b) `grep` crate's `RegexMatcher` and DuckDB's RE2 are both linear-time; no semantic differences for common patterns. (c) `ignore` crate's `filter_entry` is evaluated before file I/O — test file skipping is free. (d) Appender bulk insert for source files adds negligible time to indexing. See [13-zen-grep-design.md](./13-zen-grep-design.md) | Phase 3 (3.16-3.18), Phase 4 (4.10-4.12), Phase 5 (5.19-5.20) |
| 0.15 | ~~Write schema generation & validation spike: validate `schemars` 1.x + `jsonschema` 0.28 full integration, per-entity data dispatch, SchemaRegistry~~ | **DONE** — 22/22 tests pass. Validated: (1) `schemars` 1.x `#[derive(JsonSchema)]` works with all entity structs, serde attributes (`rename_all`, `Option`, `DateTime<Utc>`, `serde_json::Value`), and `chrono04` feature. (2) All 12 entity types + 8 enums generate correct schemas; roundtrip (serialize → validate → deserialize) passes for every entity. (3) Per-entity `data` dispatch works: correct entity data passes, wrong entity data fails with descriptive errors. (4) Trail envelope schema matches spike 0.13 hand-written schema; schemars-generated version is strictly superior (validates `data` sub-schemas). (5) Config schema generation works for all 6 sections. (6) Audit detail per-action schemas work (StatusChanged, Linked, Tagged, Indexed). (7) DuckDB metadata schemas for Rust/Python/TypeScript all generate correctly including nested `Option<Vec<String>>` and `HashMap<String,String>`. (8) SchemaRegistry prototype: 39 schemas, construction <50ms, validation sub-microsecond. (9) Both Draft 2020-12 (schemars default) and Draft 7 (via `SchemaSettings`) work with `jsonschema` 0.28. **Gotcha**: schemars does NOT add `additionalProperties: false` by default — convention decision needed. **Decision**: Use schemars-generated schemas everywhere; retire hand-written schema from spike 0.13. See [12-schema-spike-plan.md](./12-schema-spike-plan.md) | Phase 1 (entity structs get `#[derive(JsonSchema)]`), Phase 2 (trail + audit validation), Phase 5 (`znt schema` command, pre-commit uses generated schema) |
| 0.16 | ~~Write JSONL trail schema versioning spike: validate Approach D (Hybrid) — `v` field with `#[serde(default)]`, additive evolution, version-dispatch migration, `additionalProperties` convention, `serde-jsonlines` roundtrip~~ | **DONE** — 10/10 tests pass. Validated: (1) `#[serde(default = "fn")]` on `v: u32` — old trails without `v` field deserialize as v1. schemars does NOT include `v` in `required` array. (2) Additive evolution — `Option<T>` and `#[serde(default)]` fields work for both serde deserialization AND schema validation (schemars excludes default fields from `required`). (3) `#[serde(alias)]` — serde deserialization works (old field names map to new), BUT schemars schema uses Rust field name only (schema validation rejects old names). (4) Version-dispatch migration — transform `serde_json::Value` in-place, validate against target schema, dispatch by `op.v`, reject unsupported versions. (5) `additionalProperties` convention confirmed — trail (no `deny_unknown_fields`) accepts unknowns; config (`#[serde(deny_unknown_fields)]`) generates `additionalProperties: false` and rejects unknowns. (6) `serde-jsonlines` roundtrip preserves `v` field; old-format files (no `v`) read back with `v == 1`; mixed old+new files work. **Decision**: Approach D adopted. Trail envelope gets `v: u32` with `#[serde(default)]`. Evolution rules: additive by default, version bump for breaking changes. `additionalProperties`: permissive for trails, strict for config. `serde(alias)` is serde-safe but schema-unsafe. See [14-trail-versioning-spike-plan.md](./14-trail-versioning-spike-plan.md) | Phase 2 (tasks 2.15-2.17 trail writer/replayer/versioning) |
| 0.17 | ~~Write Clerk Auth + Turso JWKS spike: validate `clerk-rs` JWT validation, `tiny_http` browser callback, `keyring` token storage, Turso JWKS integration (Clerk JWT as libsql auth token), API key fallback~~ | **DONE** — 14/14 tests pass. Validated: (1) `clerk-rs` 0.4.2 `MemoryCacheJwksProvider` + `validate_jwt()` work standalone without web framework. (2) `tiny_http` localhost callback captures JWT from redirect URL. (3) `keyring` v3 store/retrieve/delete works on macOS Keychain. File fallback with 0600 permissions works. (4) JWT `exp` claim decoding and near-expiry detection (60s buffer) work. (5) **Turso JWKS accepts Clerk JWT**: `Builder::new_remote()` with Clerk JWT as auth token — `SELECT 1` succeeds. (6) **Embedded replicas work**: `Builder::new_remote_replica()` with Clerk JWT — sync, write, read all succeed. (7) **Write-forwarding**: replica 1 writes, syncs; replica 2 syncs, sees the data. (8) **Expired token behavior**: auth validated at builder time (not deferred) — `Sync("Unauthorized")` error immediately. (9) JWKS public endpoint returns 1 RSA key (RS256). **Key finding**: Turso JWKS = zero runtime token minting. Clerk JWT is the auth token. `open` crate opens browser on macOS. **Gotchas**: (a) Auth is validated at `Builder::new_remote_replica().build()` time — expired tokens fail immediately. (b) No hot-swapping auth tokens on embedded replicas — must recreate client. (c) Local reads continue working after token expiry. See [15-clerk-auth-turso-jwks-spike-plan.md](./15-clerk-auth-turso-jwks-spike-plan.md) | Phase 9 (zen-auth crate, Turso JWKS wiring, team DB) |
| 0.18 | ~~Write R2 Lance Export spike: validate Lance format on R2 for shared team index — vector search, FTS, hybrid search, JSON metadata roundtrip, incremental export, manifest lifecycle~~ | **DONE** — 18/18 tests pass (13 Parquet + 5 Lance). Validated: (1) **Parquet export/read**: all 3 tables (api_symbols, doc_chunks, indexed_packages) export to R2 and read back correctly. `FLOAT[]` embeddings need `::FLOAT[384]` cast for `array_cosine_similarity()`. JSON metadata survives Parquet roundtrip (JSON operators work on Parquet-read data). DuckDB manifest returns Struct type — need `to_json()::VARCHAR`. (2) **Performance**: 10K symbols insert 769ms, export to R2 5.1s, vector search 3.0s, text filter 310ms. (3) **Incremental**: delta export by timestamp works, multi-file merge via `read_parquet([...])` works. (4) **Lance on R2**: `COPY TO (FORMAT lance)` works, `lance_vector_search()` with 384-dim returns correct nearest neighbors (distance=0.0 for self-match). `lance_fts()` BM25 works. `lance_hybrid_search()` combines vector + text (alpha=0.5). (5) **Lance vs Parquet**: at 100 rows Parquet is 2x faster (brute-force cheaper than Lance overhead). At scale, Lance's persistent indexes will dominate. **Decision**: Use Lance format for both shared team index and local index. Lance provides native vector search, BM25 FTS, and hybrid search without brute-force scan. **Gotchas**: (a) Lance uses AWS credential chain (not DuckDB secrets) — must set `AWS_ACCESS_KEY_ID`, `AWS_SECRET_ACCESS_KEY`, `AWS_ENDPOINT_URL` env vars. (b) Lance FTS is term-exact, not stemmed. (c) Parquet `FLOAT[]` needs explicit cast to `FLOAT[384]` for cosine ops. See [16-r2-parquet-export-spike-plan.md](./16-r2-parquet-export-spike-plan.md) | Phase 9 (R2 Lance export, shared reader, `znt export`) |
| 0.19 | ~~Write native lancedb writes spike: validate `lancedb` Rust crate for writing Lance to R2, Arrow version bridge (duckdb arrow-56 vs lancedb arrow-57), serde_arrow production path, arrow_serde chrono adapters~~ | **DONE** — 10/10 tests pass. Validated: (1) **Arrow version bridge**: Value-based reconstruction converts arrow-56 → 57 (FFI doesn't work — Rust treats same `#[repr(C)]` layout as different types across crate versions). (2) **lancedb local + R2 writes**: api_symbols schema (19 cols, `FixedSizeList(Float32, 384)`) writes and reads back via DuckDB lance extension. (3) **Explicit indexes**: IVF-PQ vector + BM25 FTS via `create_index()`. PQ needs >= 256 rows. (4) **Incremental add**: `tbl.add()` for delta updates (100 + 50 = 150, search works across both). (5) **DuckDB → lancedb pipeline**: `query_arrow()` → value bridge → `create_table()` → local + R2 (EXPLORATORY — not production code, see Part I docs). (6) **Cross-process index read**: indexes survive handle drop, fresh DuckDB connection reads them. (7) **exist_ok**: `CreateTableMode::exist_ok()` returns existing table without data loss. (8) **serde_arrow production path** (test M1): Rust structs → `serde_arrow::to_record_batch()` → `lancedb::create_table()` → DuckDB reads → `serde_arrow::from_record_batch()` → Rust structs. Full round-trip with `DateTime<Utc>` via `arrow_serde` adapter. Vector search distance=0.000000. **Key decisions**: (a) `serde_arrow` is the production bridge (no DuckDB extraction needed). (b) `arrow_serde` ported from aether to zen-core (timestamp_micros_utc, date32 adapters). (c) `FixedSizeList(384)` override needed (serde_arrow defaults `Vec<f32>` to `LargeList`). (d) `unsafe_code = "forbid"` preserved — value bridge is safe. (e) `object_store` downgraded 0.13 → 0.12 for lance 2.0. (f) `protoc` required by `lance-encoding`. See [17-native-lance-spike-plan.md](./17-native-lance-spike-plan.md) | Phase 2 (zen-lake writes), Phase 9 (R2 upload) |
| 0.20 | ~~Write Turso catalog + Clerk visibility spike: validate DuckLake-inspired catalog tables in Turso, Clerk JWT org claims for visibility scoping, embedded replica sync, three-tier search, concurrent write dedup, programmatic org-scoped JWT generation~~ | **DONE** — 9/9 tests pass. Validated: (1) **Programmatic org-scoped JWT** (J0): create session → get JWT from `zenith_cli` template → `clerk-rs` validates → `ActiveOrganization { id, slug, role, permissions }` extracted. (2) **indexed_packages schema in Turso** (J1): visibility columns work, public/team/private scoping correct. (3) **Embedded replica sync** (J2): catalog replicates, offline reads work. (4) **Clerk JWT visibility scoping** (J3): real `org_id` from JWT drives team visibility, `sub` drives private. No custom RBAC. (5) **End-to-end catalog → search** (J4): Turso catalog → lance path → DuckDB `lance_vector_search()` → distance=0.0. (6) **Three-tier search** (K1): public + team visible, private excluded, results merged. (7) **Private code isolation** (K2): only owner discovers private packages. (8) **Concurrent dedup** (L1): PRIMARY KEY → `SQLITE_CONSTRAINT`, first writer wins, concurrent race resolved correctly. (9) **Two Turso replicas** (L3): separate DBs coexist in same process, no interference. **Key findings**: (a) `org_permissions` must be `[]` in JWT template (shortcode doesn't resolve, breaks clerk-rs deserialization). (b) Turso "shared lock on node" errors are infrastructure-level (DB creation/deletion), not application concurrency. (c) Created Clerk org `zenith-dev` (`org_39PSbEI9mVoLgBQWuASKeltV7S9`), user `zenith_dev` is admin. See [18-catalog-visibility-spike-plan.md](./18-catalog-visibility-spike-plan.md) | Phase 8 (Turso catalog), Phase 9 (visibility, team, crowdsource) |

### Milestone 0

- All spikes compile and pass
- `cargo build` succeeds for the workspace
- `cargo test` passes for all spikes
- Decision made: AgentFS from git works (proceed) or doesn't (use fallback)

### Validation

```bash
cargo build --workspace
cargo test --workspace
```

---

## 3. Phase 1: Foundation

**Goal**: Core types, error handling, configuration, and database schema.

**Depends on**: Phase 0 (libsql spike works)

### Tasks

| ID | Task | Crate | Blocks |
|----|------|-------|--------|
| 1.1 | Define all entity structs (Finding, Hypothesis, Issue, Task, etc.) with `#[derive(JsonSchema)]` | zen-core | 1.4 |
| 1.2 | Define all enums (status types, entity types, relations, actions) with `#[derive(JsonSchema)]` | zen-core | 1.4 |
| 1.3 | Define error hierarchy (`ZenError`, sub-errors per crate) | zen-core | 1.4 |
| 1.4 | Implement ID prefix constants and `gen_id_sql()` helper | zen-core | 1.6 |
| 1.5 | ~~Implement `ZenConfig` with figment (turso, motherduck, r2, general sections)~~ | zen-config | **DONE** — 46/46 tests pass. Figment `Env::prefixed("ZENITH_").split("__")` handles env vars (no manual `std::env::var()`). `String` fields with empty defaults (not `Option<String>`). Added Clerk + Axiom config sections. Storage wiring helpers: `R2Config::create_secret_sql()`, `MotherDuckConfig::connection_string()`, `TursoConfig::db_name()` / `can_mint_tokens()`. All `.env` vars renamed to `ZENITH_*__*` format, existing spikes updated. `figment::Jail` for safe test isolation (Rust 2024 `set_var` is unsafe). Real `.env` values flow through figment and match spike `std::env::var()` reads. See `05-crate-designs.md` §4 for gotchas. |
| 1.6 | Write full SQL migration file (all 14 tables + 7 FTS5 + indexes + triggers) from `01-turso-data-model.md` | zen-db | 1.7 |
| 1.7 | Implement `ZenDb::open_local()`, run migrations, verify schema | zen-db | 1.8 |
| 1.8 | Implement `ZenDb::generate_id()` using Turso's `randomblob()` | zen-db | Phase 2 |

### Tests

- zen-core: Serde roundtrip for every entity, enum string representation, ID prefix correctness, `JsonSchema` generation validates against serde output
- zen-config: **DONE** — 46 tests (26 unit + 10 TOML/Jail + 9 dotenv + 1 doctest). Default loads, TOML per-section, env overrides TOML, typo gotcha documented, full provider chain, real `.env` values, spike compatibility
- zen-db: Schema creation, `generate_id()` produces correct prefix format, basic INSERT+SELECT for each table
- zen-schema: SchemaRegistry loads all entity + trail + audit + config schemas, `validate()` accepts valid data and rejects invalid data with descriptive errors

### Milestone 1

- `cargo test -p zen-core -p zen-config -p zen-db -p zen-schema` all pass
- Database opens, schema created, IDs generate correctly
- Every entity can be inserted and queried back
- SchemaRegistry available with all entity and trail operation schemas

---

## 4. Phase 2: Storage Layer

**Goal**: CRUD operations for every entity, FTS5 search, audit trail, session management.

**Depends on**: Phase 1

### Tasks

| ID | Task | Crate | Blocks |
|----|------|-------|--------|
| 2.1 | Implement `SessionRepo`: start, end, list, snapshot, orphan detection | zen-db | 2.8 |
| 2.2 | Implement `ProjectRepo`: meta CRUD, dependency CRUD | zen-db | Phase 5 |
| 2.3 | Implement `ResearchRepo`: CRUD + FTS search | zen-db | 2.8 |
| 2.4 | Implement `FindingRepo`: CRUD + tag/untag + FTS search | zen-db | 2.8 |
| 2.5 | Implement `HypothesisRepo`: CRUD + status transitions (validate allowed transitions) | zen-db | 2.8 |
| 2.6 | Implement `InsightRepo`: CRUD + FTS search | zen-db | 2.8 |
| 2.7 | Implement `IssueRepo`: CRUD + FTS + parent-child queries | zen-db | 2.8 |
| 2.8 | Implement `TaskRepo`: CRUD + FTS + issue linkage | zen-db | Phase 5 |
| 2.9 | Implement `ImplLogRepo`: CRUD + file path queries | zen-db | Phase 5 |
| 2.10 | Implement `CompatRepo`: CRUD + package pair queries | zen-db | Phase 5 |
| 2.14 | Implement `StudyRepo`: CRUD + FTS + progress tracking + conclude lifecycle | zen-db | Phase 5 |
| 2.15 | Implement JSONL trail writer: append operations to per-session `.zenith/trail/ses-xxx.jsonl` on every mutation. Validate `Operation.data` against per-entity schema from zen-schema before writing. | zen-db | Phase 5 |
| 2.16 | Implement JSONL replayer: read all trail files, replay operations to rebuild DB (`znt rebuild`). Support `--strict` flag for schema validation on replay. | zen-db | Phase 5 |
| 2.17 | Implement JSONL schema versioning (Approach D, validated in spike 0.16): add `v: u32` with `#[serde(default)]` to `TrailOperation` envelope, implement replay version dispatch with `match op.v`, implement first migration function when needed. Evolution rules: additive changes (Option/default) don't bump version; type changes and required-field additions bump version. `additionalProperties`: permissive for trails, strict for config. | zen-db + zen-schema | Phase 5, Phase 8 |
| 2.11 | Implement `LinkRepo`: create, delete, query by source, query by target | zen-db | Phase 5 |
| 2.12 | Implement `AuditRepo`: append (every repo method calls this), query with filters. Validate audit `detail` payloads against per-action schemas from zen-schema. | zen-db | 2.13 |
| 2.13 | Implement `whats_next()` query: aggregate open tasks, pending hypotheses, recent audit | zen-db | Phase 5 |

### Tests

- CRUD roundtrip for every entity type
- FTS5 search: porter stemming ("spawning" matches "spawn")
- Hypothesis status: valid transitions succeed, invalid transitions return error
- Audit: every CRUD operation produces an audit entry
- Session: start → active, wrap-up → wrapped_up, orphan detection marks abandoned
- Entity links: bidirectional query (find all links FROM entity, find all links TO entity)
- `whats_next()`: returns correct aggregate counts
- Study: create, add assumptions, record test results, conclude, progress tracking
- Study FTS: searching studies by topic and summary
- JSONL trail: every mutation appends to per-session trail file
- JSONL rebuild: replay all trail files produces identical DB state (including FTS5)
- JSONL concurrent: multiple sessions write to separate files without corruption

### Milestone 2

- Complete CRUD layer with 13 repo modules (Session, Project, Research, Finding, Hypothesis, Insight, Issue, Task, ImplLog, Compat, Study, Link, Audit)
- JSONL trail writer validates and appends every mutation; replayer rebuilds DB from trail
- Every mutation writes to audit trail
- FTS5 search works across all searchable entities
- `whats_next()` returns structured project state

---

## 5. Phase 3: Parsing & Indexing Pipeline

**Goal**: ast-grep-based extraction across all 26 built-in languages (rich extractors for 7, generic for 19), fastembed integration, DuckDB lake storage.

**Depends on**: Phase 0 (ast-grep, fastembed, duckdb spikes), Phase 1 (zen-core types)

### Tasks

| ID | Task | Crate | Blocks |
|----|------|-------|--------|
| 3.1 | Implement `Parser`: language detection via `SupportLang`, parse file with `ast_grep()` | zen-parser | 3.2 |
| 3.2 | Implement `ParsedItem` struct with full metadata (port from klaw `rust-treesitter.ts`) | zen-parser | 3.3 |
| 3.3 | Implement Rust rich extractor using ast-grep patterns + Node traversal (13 node types, doc comments, attributes, generics, lifetimes, error detection, impl blocks) | zen-parser | 3.10 |
| 3.4 | Implement Python rich extractor using ast-grep patterns (classes, decorators, docstrings with Google/NumPy/Sphinx support, pydantic/protocol/dataclass detection) | zen-parser | 3.10 |
| 3.5 | Implement TypeScript/JavaScript/TSX rich extractor (exports, classes, interfaces, type aliases) | zen-parser | 3.10 |
| 3.6 | Implement Go rich extractor (exported functions/types/methods, doc comments) | zen-parser | 3.10 |
| 3.7 | Implement Elixir rich extractor (defmodule, def/defp, defmacro) | zen-parser | 3.10 |
| 3.8 | Implement generic kind-based extractor for all remaining 19 built-in languages | zen-parser | 3.10 |
| 3.9 | Implement `IsTestFile()`, `IsTestDir()` for all supported languages | zen-parser | 3.10 |
| 3.10 | Implement two-tier extraction fallback: ast-grep → regex | zen-parser | 3.14 |
| 3.11 | Implement `EmbeddingEngine`: init fastembed, `embed_batch()`, `embed_single()` | zen-embeddings | 3.14 |
| 3.12 | Implement `ZenLake::open_local()`: DuckDB connection, extension loading, table creation | zen-lake | 3.13 |
| 3.13 | Implement `ZenLake::store_symbols()`, `store_doc_chunks()`, `register_package()`. **Note**: DuckLake does not support `FLOAT[N]` — store embeddings as `FLOAT[]` and cast to `FLOAT[384]` at query time. | zen-lake | 3.14 |
| 3.14 | Implement full indexing pipeline: clone repo → walk files → parse → extract → embed → store in lake | zen-lake + zen-parser + zen-embeddings | Phase 4 |
| 3.15 | Implement doc chunking: split README/docs by section headings, chunk to ~512 tokens | zen-parser or zen-lake | 3.14 |
| 3.16 | Add `source_files` table to DuckDB schema, add `source_cached` to `indexed_packages` | zen-lake | 3.17 |
| 3.17 | Store source file contents during indexing pipeline (step 6.5) | zen-lake | 4.10 |
| 3.18 | Implement `walk.rs` walker factory (`WalkMode::LocalProject`, `Raw`) with `ignore` crate | zen-search | 4.10, 3.14 |

### Tests

- Parse real Rust, Python, TypeScript, Go source files (fixture files in `tests/fixtures/`)
- Verify `ParsedItem` metadata: async detection, visibility, generics, doc comments, error types
- Verify signature extraction (no body leaks)
- Verify test file detection for all languages
- Verify generic extractor produces usable output for non-rich languages (C, Java, Ruby, etc.)
- Verify ast-grep pattern matching captures metavariables correctly
- Embedding: generates 384-dim vectors, similar texts have high cosine similarity
- Lake: round-trip insert + query for symbols and doc chunks
- Full pipeline: index a small real crate (e.g., `anyhow`), verify symbols and chunks stored

### Milestone 3

- `zen-parser` extracts rich API symbols from 7 languages, basic symbols from 19 more
- `zen-embeddings` generates local vectors
- `zen-lake` stores and retrieves indexed packages
- Full pipeline: clone → parse → embed → store works end-to-end

---

## 6. Phase 4: Search & Registry

**Goal**: Vector search over the lake, FTS over knowledge entities, registry API clients.

**Depends on**: Phase 2 (zen-db FTS), Phase 3 (zen-lake with vectors)

### Tasks

| ID | Task | Crate | Blocks |
|----|------|-------|--------|
| 4.1 | Implement vector search: embed query → Lance `lance_vector_search()` or brute-force `array_cosine_similarity()` in DuckDB. **Note**: embeddings stored as `FLOAT[]`, cast to `FLOAT[384]` for cosine ops. | zen-search | 4.4 |
| 4.2 | Implement FTS search: query zen-db FTS5 tables (findings, tasks, audit, etc.) | zen-search | 4.4 |
| 4.3 | Implement hybrid search: combine vector + FTS scores. **Note**: Lance FTS is term-exact (no stemming) vs libSQL FTS5 (porter stemming) — vector should be primary signal, FTS as boost. Validate `alpha` parameter with real queries. | zen-search | 4.4 |
| 4.4 | Implement `SearchEngine` orchestrator with filters (package, kind, ecosystem, limit, context-budget) | zen-search | Phase 5 |
| 4.5 | Implement crates.io client | zen-registry | Phase 5 |
| 4.6 | Implement npm registry client (+ api.npmjs.org for downloads) | zen-registry | Phase 5 |
| 4.7 | Implement PyPI client | zen-registry | Phase 5 |
| 4.8 | Implement hex.pm client | zen-registry | Phase 5 |
| 4.9 | Implement `search_all()`: concurrent search across all registries | zen-registry | Phase 5 |
| 4.10 | Implement `GrepEngine::grep_package()` — DuckDB fetch + Rust regex + symbol correlation | zen-search | 5.19 |
| 4.11 | Implement `GrepEngine::grep_local()` — `grep` + `ignore` crates, custom `Sink` | zen-search | 5.19 |
| 4.12 | Add `idx_symbols_file_lines` index to `api_symbols` | zen-lake | 4.10 |

### Tests

- Vector search: insert known vectors, verify nearest neighbor returns correct results
- FTS: porter-stemmed queries match expected results
- Hybrid: combined ranking produces better results than either alone
- Registry: parse real API response fixtures (recorded JSON), handle errors (404, rate limit)
- `search_all()` merges and sorts by downloads

### Milestone 4

- `znt search "async spawn"` returns ranked results from indexed packages
- `znt research registry "http client" --ecosystem rust` returns crates.io results
- Hybrid search combines vector similarity + FTS relevance

---

## 7. Phase 5: CLI Shell

**Goal**: Working `znt` binary with all commands wired up. This is the first fully usable milestone.

**Depends on**: Phase 2 (all repos), Phase 4 (search + registry)

### Tasks

| ID | Task | Crate | Blocks |
|----|------|-------|--------|
| 5.1 | Implement clap `Cli` struct with all subcommands and global flags | zen-cli | 5.2 |
| 5.2 | Implement `main.rs`: load config, init tracing, open database, dispatch commands | zen-cli | 5.3 |
| 5.3 | Implement `znt init`: detect project, parse manifest, create `.zenith/`, init DB | zen-cli | 5.4 |
| 5.4 | Implement `znt session start/end/list` | zen-cli | 5.5 |
| 5.5 | Implement knowledge commands: `znt research`, `znt finding`, `znt hypothesis`, `znt insight` (all CRUD) | zen-cli | 5.7 |
| 5.6 | Implement work commands: `znt issue`, `znt task`, `znt log`, `znt compat` | zen-cli | 5.7 |
| 5.7 | Implement linking: `znt link`, `znt unlink` | zen-cli | 5.8 |
| 5.16 | Implement study commands: `znt study create/assume/test/get/conclude/list` | zen-cli | Done |
| 5.17 | Implement `znt rebuild`: delete DB, replay all JSONL trail files, rebuild FTS5 | zen-cli | Done |
| 5.18a | Implement `znt init` `.gitignore` template (ignore DB files, track trail/ and hooks/) | zen-cli | 5.18b |
| 5.18b | Implement pre-commit hook: validate staged `.zenith/trail/*.jsonl` files via `znt hook pre-commit` (Rust validation with `serde_json`, schema checks). Thin shell wrapper with graceful fallback if `znt` not in PATH. | zen-hooks + zen-cli | 5.18e |
| 5.18c | Implement post-checkout hook: detect JSONL trail changes between old and new HEAD via `gix` tree diff, trigger `znt rebuild` or warn based on performance threshold from spike 0.13 | zen-hooks + zen-cli | 5.18e |
| 5.18d | Implement post-merge hook: detect conflict markers in JSONL files, trigger rebuild if clean merge changed trail files | zen-hooks + zen-cli | 5.18e |
| 5.18e | Implement hook installation in `znt init`: detect git repo via `gix`, detect existing hooks / `core.hooksPath`, install using strategy chosen by spike 0.13 (hookspath / symlink / chain), support `--skip-hooks` flag | zen-hooks + zen-cli | Done |
| 5.8 | Implement `znt audit` with all filters | zen-cli | 5.9 |
| 5.9 | Implement `znt whats-next` (both JSON and raw formats) | zen-cli | 5.11 |
| 5.10 | Implement `znt search` wired to SearchEngine | zen-cli | 5.11 |
| 5.11 | Implement `znt install`: clone repo, run indexing pipeline, update project_dependencies | zen-cli | 5.12 |
| 5.12 | Implement `znt onboard`: detect project, parse manifest, batch index all deps | zen-cli | 5.13 |
| 5.13 | Implement `znt wrap-up`: session summary, snapshot, audit export | zen-cli | 5.14 |
| 5.14 | Implement `znt research registry` wired to RegistryClient | zen-cli | 5.15 |
| 5.15 | Implement JSON/table/raw output formatting for all commands | zen-cli | Done |
| 5.19 | Implement `znt grep` CLI command (package mode + local mode, all flags) | zen-cli | Done |
| 5.20 | Implement `znt cache` CLI command (list, clean, stats) | zen-cli | Done |
| 5.21 | Implement `warn_unconfigured()` at CLI startup: detect figment config sections with all-default values, warn user about possible typo'd env var keys (confirmed gotcha from zen-config spike) | zen-cli | Done |
| 5.22 | Implement `znt schema <type>` CLI command: dump JSON Schema for any registered type from SchemaRegistry. Uses `SchemaRegistry.get()` + pretty print. | zen-cli | Done |
| 5.23 | Update pre-commit hook (task 5.18b) to use schemars-generated schemas from zen-schema instead of hand-written schema from spike 0.13 | zen-hooks + zen-schema | Done |

### Tests

- Integration tests: build the binary, run commands as subprocesses, verify JSON output
- `znt init` creates `.zenith/` with valid DB
- `znt session start` → `znt finding create` → `znt audit` shows the finding creation
- `znt install <small-crate>` → `znt search` returns results from it
- `znt whats-next` returns correct state after a sequence of operations
- Error cases: invalid command, missing args, entity not found

### Milestone 5

**This is the MVP.** The `znt` binary is functional:
- Initialize a project, start sessions, track knowledge
- Install and index packages, search documentation
- Query registries, manage issues/tasks
- View audit trail, get project state with `whats-next`
- Wrap up sessions

---

## 8. Phase 6: PRD Workflow

**Goal**: Full ai-dev-tasks PRD workflow via `znt prd` commands.

**Depends on**: Phase 5 (working CLI with issues and tasks)

### Tasks

| ID | Task | Crate | Blocks |
|----|------|-------|--------|
| 6.1 | Implement `znt prd create`: creates epic issue, returns ID | zen-cli | 6.2 |
| 6.2 | Implement `znt prd update`: stores PRD markdown in issue description | zen-cli | 6.3 |
| 6.3 | Implement `znt prd tasks`: creates parent tasks linked to epic, returns list with "confirm" message | zen-cli | 6.4 |
| 6.4 | Implement `znt prd subtasks`: creates sub-tasks linked to parent via entity_links | zen-cli | 6.5 |
| 6.5 | Implement `znt prd get`: returns full PRD with tasks, progress, findings, open questions | zen-cli | 6.6 |
| 6.6 | Implement `znt prd complete`: marks epic done, creates summary audit entry | zen-cli | 6.7 |
| 6.7 | Implement `znt prd list`: lists all epic issues with progress percentages | zen-cli | Done |

### Tests

- Full PRD lifecycle: create → update → tasks → subtasks → execute → complete
- `znt prd get` returns correct progress counts (done/total tasks)
- Multi-session PRD: start PRD in session 1, complete half tasks, wrap-up, start session 2, `znt prd get` shows correct state
- Task execution: `in_progress` → `done` with implementation log entries

### Milestone 6

- Complete PRD workflow matches ai-dev-tasks behavior
- PRDs persist across sessions
- Multiple PRDs can run in parallel

---

## 9. Phase 7: AgentFS Integration

**Goal**: Workspace isolation for sessions and package indexing via AgentFS (or fallback).

**Depends on**: Phase 0 (AgentFS spike result), Phase 5 (working CLI)

### If AgentFS Works (0.7 passed)

| ID | Task | Crate | Blocks |
|----|------|-------|--------|
| 7.1 | Create `Workspace` trait in zen-core | zen-core | 7.2 |
| 7.2 | Implement `AgentFsWorkspace` wrapping the AgentFS Rust SDK | zen-cli or zen-lake | 7.3 |
| 7.3 | Wire session start to create AgentFS workspace per session | zen-cli | 7.4 |
| 7.4 | Wire `znt install` to use AgentFS workspace for cloning | zen-lake | 7.5 |
| 7.5 | Wire `znt wrap-up` to snapshot AgentFS workspace | zen-cli | 7.6 |
| 7.6 | Wire `znt audit --files` to query AgentFS audit log | zen-cli | Done |

### If AgentFS Doesn't Work (0.7 failed, 0.10 executed)

| ID | Task | Crate | Blocks |
|----|------|-------|--------|
| 7.1 | Create `Workspace` trait in zen-core | zen-core | 7.2b |
| 7.2b | Implement `TempDirWorkspace` using `tempfile::TempDir` | zen-core or zen-lake | 7.4b |
| 7.4b | Wire `znt install` to use TempDirWorkspace for cloning | zen-lake | Done |

Note: without AgentFS, we skip session workspaces and file-level audit. These become future enhancements when AgentFS stabilizes.

### Tests

- Workspace creation, file read/write, deletion
- Package indexing through workspace (clone → parse → cleanup)
- Session workspace snapshot (AgentFS path only)

### Milestone 7

- Package indexing uses isolated workspaces (crash-safe)
- Session file-level audit available via `znt audit --files` (AgentFS path)

---

## 10. Phase 8: Cloud & Catalog

**Goal**: Turso Cloud sync for entities, Turso catalog for package index (DuckLake-inspired), Lance on R2 for search data. MotherDuck removed from architecture — replaced by Turso catalog + lancedb writes.

**Depends on**: Phase 5 (working local tool), Spikes 0.17-0.20

### Tasks

| ID | Task | Crate | Blocks |
|----|------|-------|--------|
| 8.1 | Implement `ZenDb::open_synced()` with Turso Cloud (Clerk JWT via JWKS) | zen-db | 8.2 |
| 8.2 | Wire `znt wrap-up` to call `ZenDb::sync()` | zen-cli | 8.5 |
| 8.3 | Implement DuckLake-inspired catalog tables in Turso (`dl_metadata`, `dl_snapshot`, `dl_data_file`) | zen-db | 8.5 |
| 8.4 | Implement `ZenLake::write_to_r2()` using lancedb Rust crate + serde_arrow (production path from spike 0.19 M1) | zen-lake | 8.5 |
| 8.5 | Implement `ZenLake::search()` — query Turso catalog for lance paths → DuckDB lance extension search | zen-lake | 8.6 |
| 8.6 | Implement `znt onboard` cloud mode: check Turso catalog for already-indexed packages, skip if exists | zen-cli | 8.7 |
| 8.7 | Implement config validation: check R2/Turso/Clerk credentials at startup | zen-config | Done |
| 8.8 | ~~Implement `ZenLake::open_cloud()` with MotherDuck + R2~~ | **REMOVED** — MotherDuck dropped from architecture. Turso catalog + Lance on R2 replaces DuckLake. See [02-data-architecture.md](./02-data-architecture.md) | N/A |

### Tests

- Cloud sync: create entities locally, sync, verify they appear in Turso Cloud
- Catalog: register Lance dataset in Turso, query back, verify paths correct
- Lance write: serde_arrow → lancedb → R2, read back via DuckDB lance extension
- Config validation: missing/invalid credentials produce clear error messages
- Onboard cloud mode: already-indexed packages in Turso catalog are skipped

### Milestone 8

- Full cloud sync at wrap-up (Turso embedded replica)
- Package index catalog in Turso with DuckLake-inspired snapshots
- Lance datasets on R2 written via lancedb, read via DuckDB lance extension
- Config validation catches credential issues at startup

---

## 11. Phase 9: Team & Pro (Crowdsourced Index)

**Goal**: Multi-user crowdsourced package index via Turso catalog (DuckLake-inspired) + Lance on R2. Three-tier visibility (public/team/private). Clerk authentication with org claims. No custom server for MVP — CLI authenticates directly with Clerk, Turso validates JWTs via JWKS. R2 temp credentials via CF Worker for production.

**Depends on**: Phase 8 (cloud catalog working), Spikes 0.17-0.20

### Architecture

```
Any Authenticated User                   Team Member (Pro)
     │                                        │
     ├── znt auth login                       ├── znt auth login
     │   (browser → Clerk → JWT)              │   (browser → Clerk → JWT with org claims)
     │                                        │
     ├── znt install tokio                    ├── znt install internal-sdk
     │   Check Turso: already indexed?        │   Write Lance → R2 (team visibility)
     │   NO → parse, embed, write Lance→R2    │   Register in Turso: visibility='team'
     │   Register in Turso: visibility='pub'  │
     │                                        │
     ├── znt search "spawn task"              ├── znt search "auth handler"
     │   Turso: WHERE visibility='public'     │   Turso: WHERE vis='public' OR
     │   DuckDB: lance_vector_search(paths)   │     (vis='team' AND team_id=jwt.org_id)
     │                                        │   Results from tokio + internal-sdk
     └── Turso catalog (embedded replica)     └── Turso catalog (embedded replica)

                    Turso Cloud (DuckLake-inspired catalog)
                    ┌──────────────────────────────┐
                    │ dl_data_file (Lance paths)    │
                    │ dl_snapshot (versioned history)│
                    │ dl_metadata (config)          │
                    │ Clerk JWT auth (JWKS)         │
                    │ Visibility: public/team/priv  │
                    └──────────────────────────────┘
                                   │
                                   ▼
                    Cloudflare R2 (Lance datasets)
                    ┌──────────────────────────────┐
                    │ s3://zenith/lance/            │
                    │   rust/tokio/1.49.0/*.lance   │ public
                    │   acme/internal-sdk/*.lance   │ team
                    │   jdoe/my-app/*.lance         │ private
                    └──────────────────────────────┘
```

### New Crate: `zen-auth`

```
zen-auth/src/
├── lib.rs              # Public API: login(), verify(), get_claims(), logout()
├── claims.rs           # Claims struct (clerk-rs ActiveOrganization integration)
├── error.rs            # AuthError enum (port from aether)
├── jwks.rs             # JwksValidator (clerk-rs JWKS validation)
├── browser_flow.rs     # Localhost callback: tiny_http + open browser
├── api_key.rs          # CI fallback: programmatic session + JWT via Clerk Backend API
├── token_store.rs      # keyring (OS keychain) + file fallback (~/.zenith/credentials)
└── refresh.rs          # Token lifecycle: check expiry, recreate libsql client
```

### Tasks

| ID | Task | Crate | Blocks |
|----|------|-------|--------|
| **Auth core** | | | |
| 9.1 | Implement `Claims` struct wrapping `clerk-rs::ClerkJwt` + `ActiveOrganization` | zen-auth | 9.2 |
| 9.2 | Implement `AuthError` enum (port from aether, replace `tonic::Status`) | zen-auth | 9.3 |
| 9.3 | Implement JWKS validation via `clerk-rs` (MemoryCacheJwksProvider + validate_jwt) | zen-auth | 9.5 |
| 9.4 | Implement token store: `keyring` primary, `~/.zenith/credentials` fallback (0600), `ZENITH_AUTH__TOKEN` env var for CI | zen-auth | 9.5 |
| 9.5 | Implement browser login flow: `tiny_http` on `127.0.0.1:0`, `open` browser to Clerk sign-in page, capture JWT from redirect, store token | zen-auth | 9.9 |
| 9.6 | Implement programmatic auth: create session + get JWT from `zenith_cli` template via Clerk Backend API (for CI/headless) | zen-auth | 9.9 |
| 9.7 | Implement token refresh: decode JWT `exp`, detect near-expiry (60s buffer), trigger browser re-auth or fail with message. Recreate libsql client with fresh token. | zen-auth | 9.10 |
| **Catalog & visibility** | | | |
| 9.8 | Add `org_id` column to sessions, findings, hypotheses, insights, issues, tasks, studies, impl_logs, compat_checks, audit_log, entity_links. Migration `002_team.sql`. NULL = personal/local. | zen-db | 9.10 |
| 9.9 | Implement `AuthContext` struct (user_id, org_id, org_role from `ClerkJwt.org`) populated from Claims. Thread through repo methods. | zen-auth + zen-db | 9.10 |
| 9.10 | Update all entity repos to accept optional `AuthContext`. When present, scope queries with `WHERE org_id = ?`. When absent, scope to `WHERE org_id IS NULL`. | zen-db | 9.11 |
| 9.11 | Implement visibility-scoped catalog queries: `SELECT path FROM dl_data_file WHERE visibility='public' OR (visibility='team' AND team_id=?) OR (visibility='private' AND owner_id=?)` | zen-lake | 9.12 |
| 9.12 | Implement crowdsource dedup: check catalog before indexing, skip if exists, handle `SQLITE_CONSTRAINT` on concurrent write race | zen-lake | 9.14 |
| **Turso JWKS** | | | |
| 9.13 | Update `ZenDb::open_synced()` to accept Clerk JWT as auth token (via JWKS, not Platform API minting). | zen-db | 9.14 |
| 9.14 | Implement libsql client recreation on token expiry: detect `Sync("Unauthorized")` errors, get fresh Clerk token, rebuild client. | zen-db | 9.16 |
| **R2 Lance writes** | | | |
| 9.15 | Implement `ZenLake::upload_to_r2()`: serde_arrow → lancedb → R2 Lance datasets. Create vector + FTS indexes. Register in Turso catalog. | zen-lake | 9.16 |
| 9.16 | Implement `ZenLake::search_federated()`: query Turso for visible paths → DuckDB `lance_vector_search()` / `lance_fts()` / `lance_hybrid_search()` across multiple paths → merge results | zen-lake | 9.17 |
| **CLI commands** | | | |
| 9.17 | Implement `znt auth login` (browser flow → store token → print user/org) | zen-cli | 9.18 |
| 9.18 | Implement `znt auth logout` (delete from keyring, clear credentials file) | zen-cli | 9.19 |
| 9.19 | Implement `znt auth status` (show current user, org, token expiry, Turso connection state) | zen-cli | 9.20 |
| 9.20 | Implement `znt auth switch-org` (re-authenticate with different Clerk org) | zen-cli | 9.21 |
| 9.21 | Wire team mode into startup: if authenticated, use `open_synced()` with Clerk JWT + visibility-scoped search. If not, local mode. | zen-cli | 9.22 |
| 9.22 | Implement `znt team invite` / `znt team list` using clerk-rs organization APIs | zen-cli | Done |
| 9.23 | Implement `znt index .` (private code indexing: parse current project → Lance → R2 with `visibility='private'`) | zen-cli | Done |

### Three-Tier Index Model

| Tier | Visibility | Who Writes | Who Reads |
|------|-----------|------------|-----------|
| **Global** | `public` | Any authenticated user (crowdsource) | Everyone |
| **Team** | `team` | Team members (org_id from JWT) | Team members |
| **Private** | `private` | Package owner (sub from JWT) | Owner only |

### Clerk JWT Template (`zenith_cli`)

Name: `zenith_cli` | Lifetime: 7 days | Algorithm: RS256

```json
{
  "org_id": "{{org.id}}",
  "org_slug": "{{org.slug}}",
  "org_role": "{{org.role}}",
  "org_permissions": [],
  "p": {
    "rw": {
      "ns": ["{org_slug}.zenith-{env}"],
      "tables": {
        "all": {
          "data_read": true, "data_add": true,
          "data_update": true, "data_delete": true,
          "schema_add": true, "schema_update": true, "schema_delete": true
        }
      }
    }
  }
}
```

**Critical**: `org_permissions` must be `[]` (static), not `{{org.permissions}}`. See spike 0.20 findings.

### Free vs Pro Boundary

| Feature | Free (local) | Pro (team) |
|---------|-------------|------------|
| Local indexing + search | Yes | Yes |
| Global public index (read) | Yes (with auth) | Yes |
| Contribute to global index | Yes (with auth) | Yes |
| Team visibility | -- | Yes |
| Private code indexing (`znt index .`) | -- | Yes |
| Turso Cloud sync | -- | Yes |
| `znt team` commands | -- | Yes |

No license checks. No credentials = local mode. Valid Clerk JWT = authenticated mode. `org_id` in JWT = team mode.

### Tests

- Browser login mock (test localhost callback without real browser)
- Token store: keyring write/read/delete, file fallback, env var override
- Claims: `ClerkJwt.org` → `ActiveOrganization` extraction, expiry detection
- Turso JWKS: connect with Clerk JWT, verify queries succeed
- Visibility scoping: team member sees public + team, not private
- Crowdsource dedup: concurrent INSERT → first writer wins
- Private code: owner sees it, others don't
- Federated search: results from multiple Lance datasets merged and ranked
- Token refresh: detect expiry, recreate client, verify queries resume

### Milestone 9

- `znt auth login` → browser opens → user authenticates → JWT stored in keyring
- `znt install tokio` → check catalog → index → upload Lance → register in Turso
- `znt search "spawn task"` → visibility-scoped catalog query → federated lance search
- `znt index .` → private code indexed, searchable only by owner
- Team members share indexed packages via Turso catalog + R2 Lance

---

## 12. Dependency Graph

```
Phase 0: Spikes (all parallel)
    │
    ├─► Phase 1: Foundation (zen-core + zen-schema, zen-config, zen-db schema)
    │       │
    │       ├─► Phase 2: Storage Layer (zen-db repos, all 13 modules + JSONL trail with schema validation)
    │       │       │
    │       │       └─► Phase 4: Search & Registry (zen-search, zen-registry)
    │       │               │
    │       │               └─► Phase 5: CLI Shell (zen-cli + zen-hooks, MVP)
    │       │                       │
    │       │                       ├─► Phase 6: PRD Workflow
    │       │                       ├─► Phase 7: AgentFS Integration
    │       │                       └─► Phase 8: Cloud & Polish
    │       │
    │       └─► Phase 3: Parsing & Indexing (zen-parser, zen-embeddings, zen-lake)
    │               │
    │               └─► Phase 4 (needs zen-lake for vector search)
    │
    ├─► Phase 5 tasks 5.18a-e (needs spike 0.13 git hooks result)
    │
    └─► Phase 7 (needs AgentFS spike result from 0.7)

Critical path: 0 → 1 → 2 → 4 → 5 (MVP)
Parallel path: 0 → 3 (can run alongside 1+2)
Parallel path: 0.13 → 5.18a-e (git hooks, can run alongside Phase 1-4)
Parallel path: 0.14 → 3.16-3.18 → 4.10-4.12 → 5.19-5.20 (zen grep, can run alongside other Phase 3-5 tasks)
Parallel path: 0.15 → 1.1-1.2 (zen-schema, entity structs get #[derive(JsonSchema)])
Parallel path: 0.16 → 2.15-2.17 (trail versioning, envelope v field + migration dispatch)
Parallel path: 0.17 + 0.18 + 0.19 + 0.20 → Phase 8 (catalog) + Phase 9 (team & pro)
```

---

## 13. Risk Register

| Risk | Impact | Likelihood | Mitigation |
|------|--------|-----------|------------|
| AgentFS doesn't compile from git | Lose workspace isolation and file audit | **Mitigated** | Spike 0.7 confirmed: `agentfs-sdk` 0.6.0 works from crates.io (no git dep needed). KV, filesystem, tool tracking all validated. **New risk**: Turso docs (`agentfs = "0.1"`) don't match actual crate name (`agentfs-sdk`) or API surface (POSIX-level vs high-level). May need thin wrapper. Task 0.10 (fallback) cancelled. |
| `turso` crate API differs from docs | Blocks all DB work | **Realized** | Spike 0.2 confirmed: `turso` 0.5.0-pre.8 lacks FTS (experimental flag not exposed). **Mitigated**: switched to `libsql` 0.9.29 which has native FTS5. Plan to re-evaluate `turso` when stable. |
| DuckDB VSS extension doesn't work in Rust | Lose vector search in lake | **Partially realized** | Spike 0.5 confirmed: VSS loads and works in-memory (HNSW + cosine similarity + hybrid search). **However**: HNSW persistence is experimental and causes SIGABRT on DB reopen (DuckDB 1.4 bug). **Mitigation**: Use in-memory HNSW only; store embeddings in Parquet on R2; rebuild HNSW at query time or use brute-force `array_cosine_similarity()` (acceptable for <100K symbols). Also: Parquet `FLOAT[384]` → `FLOAT[]` requires explicit cast back. |
| fastembed model download fails or is slow | Blocks indexing pipeline | Low | Phase 0 spike (0.6). Fallback: skip embeddings, use FTS only |
| ~~DuckLake + MotherDuck requires features not in duckdb crate~~ | ~~Lose cloud lake~~ | **RETIRED** | MotherDuck/DuckLake removed from architecture. Replaced by Turso catalog (DuckLake-inspired tables) + Lance on R2 (native lancedb writes) + DuckDB as read-only query engine. Validated in spikes 0.19 (10/10) + 0.20 (9/9). See [02-data-architecture.md](./02-data-architecture.md). |
| Tree-sitter grammar incompatibility (local grammars for Astro/Gleam/Mojo/Markdown) | Lose 4 of 16 languages | Low | Focus on core languages (Rust, Python, TS, Go) first. Local grammars are Phase 3 stretch |
| Turso Cloud sync is slow or unreliable | Poor wrap-up experience | Low | Sync is manual (wrap-up only), can retry. Local DB always works |
| User has existing git hooks (husky, lefthook, pre-commit) | Zenith hooks fail to install or overwrite user's hooks | Medium | Spike 0.13 evaluates three installation strategies (`core.hooksPath`, symlink, chain-append). Detect existing hooks before installing. Support `--skip-hooks` flag. See [11-git-hooks-spike-plan.md](./11-git-hooks-spike-plan.md) |
| `gix` adds significant compile time | Slower builds for all developers | Medium | `gix` isolated in `zen-hooks` crate — only rebuilds when hooks code changes. Spike 0.13 measures compile time delta and identifies minimal feature flags. |
| `znt rebuild` too slow for post-checkout hook | Branch switches become sluggish | Low (< 5K ops) | Spike 0.13 measures rebuild at 100/1000/5000 ops. Threshold-based decision: auto below threshold, warn above. Configurable via `.zenith/config.toml`. |
| `znt` binary not in PATH when hooks run | Hooks skip validation silently | Medium | Wrapper approach: graceful fallback with guidance message. Pre-commit skips validation rather than blocking commit. |
| Figment silently ignores typo'd env var keys | Config appears loaded but values are defaults; hard to debug | Medium | **Confirmed** in zen-config spike. `ZENITH_TURSO__URLL` (typo) is silently ignored. Mitigation: `is_configured()` checks on every sub-config, `warn_unconfigured()` at CLI startup (task 5.21). Test `typo_env_var_silently_ignored` documents the behavior. |
| schemars `additionalProperties` convention undecided | Validation strictness mismatch between generated and hand-written schemas | Medium | **Confirmed** in spike 0.15: schemars does NOT add `additionalProperties: false` by default. Must decide convention: strict (reject unknown fields) vs permissive (allow forward-compat). Recommend: permissive for trail operations (forward-compat), strict for config (catch typos). Decision needed before Phase 2 trail writer. |
| JSONL trail schema versioning | Old trail files become unreplayable after entity shape changes | **Mitigated** | Spike 0.16 validated Approach D (Hybrid): `v: u32` field with `#[serde(default)]` in trail envelope, additive evolution by default, version-dispatch migration for breaking changes. 10/10 tests pass. **Gotcha**: `serde(alias)` is serde-safe but schema-unsafe (schemars uses Rust field name, not alias). Field renames should use alias for serde + skip schema validation for aliased fields. |
| Clerk browser flow fails in SSH/containers | Can't authenticate headless | Medium | **Mitigated**: API key fallback (task 9.6), `ZENITH_AUTH__TOKEN` env var for CI |
| Turso JWKS beta (Clerk + Auth0 only) | Locked to Clerk | Low | **Mitigated**: spike 0.17 validated. Fallback: Platform API minting (spike 0.3) |
| Clerk JWT 60s default lifetime too short for CLI | Constant re-auth | **Mitigated** | Custom JWT template `zenith_cli` with 7-day TTL validated in spike 0.17 |
| Embedded replica auth token expires mid-session | Sync/writes fail | Medium | Spike 0.17 confirmed: `Sync("Unauthorized")` error, local reads survive. Task 9.12: detect error, recreate client with fresh token. |
| Lance AWS credential chain differs from DuckDB secrets | R2 access fails for Lance | **Mitigated** | Spike 0.18 validated: set `AWS_ACCESS_KEY_ID`, `AWS_SECRET_ACCESS_KEY`, `AWS_ENDPOINT_URL` env vars from R2 config. Lance reads/writes work. |
| `keyring` crate fails on headless Linux (no Secret Service) | Token storage fails | Medium | Spike 0.17 validated file fallback with 0600 permissions (task 9.4) |
| Lance FTS is term-exact (no stemming) | "spawning" won't match "spawn" | Low | Vector search is primary signal. Lance FTS is boost only. libSQL FTS5 (porter stemming) remains for knowledge entity search. |

---

## 14. Validation Checkpoints

At each milestone, verify:

| Milestone | Validation | Command |
|-----------|-----------|---------|
| 0 | All spikes compile and pass | `cargo test --workspace` |
| 1 | DB opens, schema created, entities insertable, SchemaRegistry validates all entity types | `cargo test -p zen-core -p zen-config -p zen-db -p zen-schema` |
| 2 | All 13 repos work, FTS search works, audit trail logs everything, JSONL trail validates on write | `cargo test -p zen-db` |
| 3 | Parse Rust/Python/TS files via ast-grep, extract symbols, embed, store in DuckDB | `cargo test -p zen-parser -p zen-embeddings -p zen-lake` |
| 4 | Vector search returns relevant results, registry clients work | `cargo test -p zen-search -p zen-registry` |
| **5 (MVP)** | **`znt init` → `znt install tokio` → `znt search "spawn"` returns results. Git hooks install correctly, pre-commit validates JSONL, post-checkout detects trail changes.** | **Build binary, run e2e** |
| 6 | Full PRD lifecycle works across sessions | E2E test with sequential commands |
| 7 | Package indexing uses isolated workspaces | `cargo test -p zen-cli` (workspace tests) |
| 8 | Cloud sync works, indexed packages accessible from another machine | Manual test with Turso Cloud + MotherDuck |
| **9 (Team)** | **`znt auth login` → browser → JWT stored. `znt session start` creates org-scoped session. `znt export` writes Lance to R2. Team member queries shared index via `lance_vector_search()`.** | **E2E with two Clerk users** |

### MVP Acceptance Test (Milestone 5)

This is the sequence that must work end-to-end:

```bash
# 1. Initialize
znt init

# 2. Start working
znt session start

# 3. Research
znt research create --title "Evaluate HTTP clients"
znt research registry "http client" --ecosystem rust

# 4. Install and index a package
znt install reqwest --ecosystem rust

# 5. Search indexed docs
znt search "connection pool" --package reqwest

# 6. Track knowledge
znt finding create --content "reqwest supports connection pooling" --tag verified
znt hypothesis create --content "reqwest works with tower middleware"

# 7. Create an issue
znt issue create --type feature --title "Add HTTP client layer" --priority 2

# 8. Create tasks
znt task create --title "Implement retry logic" --issue <issue-id>
znt task update <task-id> --status in_progress
znt task complete <task-id>
znt log src/http/retry.rs#1-45 --task <task-id>

# 9. Check state
znt whats-next
znt audit --limit 10

# 10. Wrap up
znt wrap-up
```

Every command must return valid JSON. Every mutation must appear in the audit trail. `znt whats-next` must reflect the current state accurately.

---

## Cross-References

- Turso data model: [01-turso-data-model.md](./01-turso-data-model.md)
- Data architecture: [02-data-architecture.md](./02-data-architecture.md) (supersedes 02-ducklake-data-model.md)
- Native lancedb spike: [17-native-lance-spike-plan.md](./17-native-lance-spike-plan.md)
- Catalog visibility spike: [18-catalog-visibility-spike-plan.md](./18-catalog-visibility-spike-plan.md)
- Architecture overview: [03-architecture-overview.md](./03-architecture-overview.md)
- CLI API design: [04-cli-api-design.md](./04-cli-api-design.md)
- Crate designs: [05-crate-designs.md](./05-crate-designs.md)
- PRD workflow: [06-prd-workflow.md](./06-prd-workflow.md)
- Git hooks spike plan: [11-git-hooks-spike-plan.md](./11-git-hooks-spike-plan.md)
- Git & JSONL strategy: [10-git-jsonl-strategy.md](./10-git-jsonl-strategy.md)
- Schema spike plan: [12-schema-spike-plan.md](./12-schema-spike-plan.md)
- Trail versioning spike plan: [14-trail-versioning-spike-plan.md](./14-trail-versioning-spike-plan.md)
- Clerk Auth + Turso JWKS spike: [15-clerk-auth-turso-jwks-spike-plan.md](./15-clerk-auth-turso-jwks-spike-plan.md)
- R2 Lance Export spike: [16-r2-parquet-export-spike-plan.md](./16-r2-parquet-export-spike-plan.md)
