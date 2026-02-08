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
11. [Dependency Graph](#11-dependency-graph)
12. [Risk Register](#12-risk-register)
13. [Validation Checkpoints](#13-validation-checkpoints)

---

## 1. Principles

- **Validate risky dependencies early** (Phase 0) -- AgentFS, DuckDB+DuckLake, fastembed
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
| 0.1 | Create Cargo workspace with all 10 crate stubs (9 original + zen-hooks) | Rust 2024 edition, workspace structure | Everything |
| 0.2 | ~~Add `turso` crate~~ → Add `libsql` crate, write spike: create local DB, execute SQL, query rows, FTS5 | **DONE** — libsql 0.9.29 works locally (turso crate FTS blocked) | Phase 1 |
| 0.3 | ~~Add `libsql` embedded replica spike: connect to Turso Cloud, sync~~ | **DONE** — `Builder::new_remote_replica()` + `db.sync().await` works. Validated: connect, write-forward, two-replica roundtrip, FTS5 through replica, transactions, deferred batch sync. Requires `tokio multi_thread` runtime. | Phase 8 |
| 0.4 | ~~Add `duckdb` crate (bundled), write spike: create table, insert, query~~ | **DONE** — `duckdb` 1.4 (bundled) compiles and works. Validated: CRUD, Appender bulk insert (1000 rows), transactions, JSON columns, `FLOAT[384]` arrays with `array_cosine_similarity()`, `execute_batch`, file persistence. DuckDB is synchronous; async strategy documented (prefer `spawn_blocking`, `async-duckdb` as alternative). `FLOAT[N]` enforces dimension at insert time. | Phase 2 |
| 0.5 | ~~Add `duckdb` VSS extension spike: create HNSW index, vector search~~ | **DONE** — Full stack validated: (1) VSS HNSW works in-memory but crashes on persistence (DuckDB 1.4 bug). (2) MotherDuck cloud works (`md:` protocol). (3) R2 Parquet roundtrip works (`httpfs`). (4) **Lance community extension validated** as superior alternative: `lance_vector_search()`, `lance_fts()` (BM25), `lance_hybrid_search()` all work locally and on R2 (`s3://`). Lance has persistent vector indexes, no HNSW crash. **Decision**: Use Lance format for documentation lake instead of Parquet + HNSW. See `02-ducklake-data-model.md` §10. **Gotchas**: Lance FTS is term-exact (no stemming). Lance uses AWS env vars for S3 creds, not DuckDB `CREATE SECRET`. | Phase 4 |
| 0.6 | ~~Add `fastembed` crate, write spike: embed text, verify 384 dimensions~~ | **DONE** — fastembed 5.8.1 works locally. Validated: `BGESmallENV15` (default, CLS pooling, 384-dim, ~100MB) and `AllMiniLML6V2` (design model, Mean pooling, 384-dim, ~80MB). Both produce correct 384-dim vectors. Confirmed: single/batch embed, determinism, cosine similarity sanity (similar texts cluster, dissimilar don't), query/passage prefix behavior (BGE), edge cases (empty/short/long text), batch size control. API is synchronous (`&mut self`); use `spawn_blocking` from async code. Models cache to `~/.zenith/cache/fastembed/`. **Gotcha**: fastembed default cache is `.fastembed_cache` (relative CWD) — use `with_cache_dir()` for stable path. `embed()` takes `&mut self`, not `&self`. Dynamic quantized models reject sub-total batch sizes. | Phase 3 |
| 0.7 | ~~Add `agentfs` from git~~ → Add `agentfs-sdk` from crates.io, write spike: KV CRUD, filesystem ops, tool tracking | **DONE** — `agentfs-sdk` 0.6.0 works (crates.io, not git). Validated: ephemeral + persistent modes, KV (set/get/delete/keys, serde structs), filesystem (mkdir/create_file/pwrite/read_file/stat/remove), tool tracking (start/success/error, record, recent, stats). **Note**: Turso docs say `agentfs = "0.1"` but correct crate is `agentfs-sdk`; docs show simplified API that doesn't match v0.6.0 (POSIX-level FS, `&V` refs for KV, positional args for tools). Task 0.10 (fallback) not needed. | Phase 7 |
| 0.8 | ~~Add `ast-grep-core` + `ast-grep-language`, write spike: parse Rust file, pattern match, walk AST nodes~~ | **DONE** — `ast-grep-core` 0.40.5 + `ast-grep-language` 0.40.5 work. Validated 19 tests across 7 sections: (1) Core parsing works for all 7 rich languages + all 26 built-in grammars. (2) Pattern matching with metavariables works (`$NAME` single, `$$$PARAMS` multi). (3) `KindMatcher` + `Any`/`All`/`Not` composable matchers work. (4) Node traversal: `field("name")`, `field("parameters")`, `field("return_type")`, `field("body")`, `field("trait")` for impl discrimination, `prev()` sibling walking for doc comments, `children()` for enum variants/struct fields/methods, `parent()`/`ancestors()` for nesting detection. (5) Position: `start_pos().line()` zero-based, `column()` takes `&Node` arg (O(n)). (6) Raw tree-sitter fallback via `LanguageExt::get_ts_language()` + `Query`/`QueryCursor` works (uses `StreamingIterator`). **Key findings**: (a) Pattern matching is fragile for Rust — `fn $NAME() { $$$ }` does NOT match functions with return types or generics; **use `KindMatcher` as primary extraction strategy** (klaw approach), patterns only for specific structural queries. (b) `async`/`unsafe` appear as children of `function_modifiers` node, not as direct children — walk into modifiers for detection. (c) `All::new()` requires homogeneous matcher types; use `ops::Op` for mixed types. (d) `get_match()` returns `None` for `$$$` multi-metavars — must use `get_multiple_matches()`. (e) `Position::column()` requires `&Node` argument unlike `line()`. (f) `text()`/`kind()` return `Cow<str>`. (g) Smart strictness only matches `fn foo()` (not `pub fn` or `pub async fn`) — confirms KindMatcher-first approach. (h) `tree-sitter` 0.26 `QueryMatches` uses `StreamingIterator`, not `Iterator`. | Phase 3 |
| 0.9 | ~~Add `clap` derive, write spike: parse subcommands, output JSON~~ | **DONE** — clap 4.5 derive works. Validated: `Parser`/`Subcommand`/`ValueEnum` derive macros, global flags with `global = true` (work before AND after subcommand), `OutputFormat` enum restricting `--format` to json/table/raw, nested two-level subcommands (`zen finding create`), positional + optional arg mixing, default values, error rejection for missing args and unknown subcommands, JSON serialization of response structs via serde. Representative subset covers all patterns needed for the full 16-domain command tree. No gotchas found — clap derive works exactly as documented. | Phase 5 |
| 0.10 | ~~If 0.7 fails: design `Workspace` trait, implement `TempDirWorkspace` fallback~~ | **CANCELLED** — 0.7 passed, AgentFS works from crates.io | N/A |
| 0.11 | ~~Write studies feature spike: test Approach A vs Approach B~~ | **DONE** — Approach B (hybrid) selected. One new `studies` table + reuse existing entities. 15/15 tests pass. Type-safe filtering, purpose-built fields (`topic`, `library`, `methodology`), dedicated lifecycle. See [08-studies-spike-plan.md](./08-studies-spike-plan.md) | Phase 2 (StudyRepo), Phase 5 (CLI) |
| 0.12 | ~~Write JSONL trail spike: test Approach A (export only) vs Approach B (source of truth), evaluate `serde-jsonlines` crate~~ | **DONE** — Approach B (source of truth) selected. 15/15 tests pass. DB is rebuildable from JSONL (FTS5 + entity_links survive). `serde-jsonlines` confirmed (1-line batch read/write/append). Per-session files concurrent-safe (4 agents, 100 ops). Replay logic ~60 LOC. See [10-git-jsonl-strategy.md](./10-git-jsonl-strategy.md) | Phase 2 (JSONL writer + replayer), Phase 5 (`zen rebuild` CLI) |
| 0.13 | ~~Write git hooks spike: test hook implementation, installation strategy, post-checkout rebuild, `gix` for repo discovery + config + index + tree diff + session tags~~ | **DONE** — 22/22 tests pass. Decisions: (1) Hook implementation: thin shell wrapper calling `znt hook` (Rust validation via `serde_json` + `jsonschema` for schema enforcement, graceful skip if `znt` not in PATH). (2) Installation: symlink for MVP (coexists with existing hooks). (3) Post-checkout: threshold-based auto-rebuild (JSONL parse <25ms for 5K ops). (4) `gix` 0.70 adopted with `max-performance-safe` + `index` + `blob-diff`. (5) Session tags: adopt lightweight `zenith/ses-xxx` tags. (6) CLI renamed from `zen` to `znt` (zen-browser collision). **Gotchas**: `gix` `MustNotExist` doesn't reject duplicate refs — use `find_reference()` first; `config_snapshot_mut()` is in-memory only — `forget()` + `write_to()` to persist; `jq` not default-installed — Rust is the only reliable JSON validation path. See [11-git-hooks-spike-plan.md](./11-git-hooks-spike-plan.md) | Phase 5 (tasks 5.18a-e), session-git integration |
| 0.14 | ~~Write zen grep spike: validate `grep` crate (ripgrep library), `ignore` crate (gitignore-aware walking), DuckDB `source_files` table, symbol correlation~~ | **DONE** — 26/26 tests pass. Validated: (1) `grep` 0.4 — `RegexMatcher` compiles patterns with flags (case-insensitive, word, literal, smart-case), `Searcher` + `UTF8` sink with line numbers, custom `Sink` for context lines, binary detection, `search_path` for files. (2) `ignore` 0.4 — `WalkBuilder` respects `.gitignore`, override globs for include/exclude, `filter_entry` for test file skipping, custom ignore filename (`.zenithignore`), hidden file skipping, combined grep+ignore workflow. (3) DuckDB `source_files` table — CRUD, Appender bulk insert, `regexp_matches()` with flags, `string_split()`+`unnest()` line-level grep with line numbers, language filtering, cache management (DELETE, stats). (4) Symbol correlation — `idx_symbols_file_lines` composite index, batch symbol lookup per file, binary search matches line→symbol range, `SymbolRef` population with all fields (id, kind, name, signature). (5) Combined pipeline — store source during indexing → grep with `RegexMatcher`+`Searcher` over stored content → correlate with `api_symbols` → `CorrelatedHit` with all fields validated. **Key findings**: (a) DuckDB fetch + Rust regex is faster than SQL-level line splitting; use DuckDB as compressed storage, Rust for line matching. (b) `grep` crate's `RegexMatcher` and DuckDB's RE2 are both linear-time; no semantic differences for common patterns. (c) `ignore` crate's `filter_entry` is evaluated before file I/O — test file skipping is free. (d) Appender bulk insert for source files adds negligible time to indexing. See [13-zen-grep-design.md](./13-zen-grep-design.md) | Phase 3 (3.16-3.18), Phase 4 (4.10-4.12), Phase 5 (5.19-5.20) |

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
| 1.1 | Define all entity structs (Finding, Hypothesis, Issue, Task, etc.) | zen-core | 1.4 |
| 1.2 | Define all enums (status types, entity types, relations, actions) | zen-core | 1.4 |
| 1.3 | Define error hierarchy (`ZenError`, sub-errors per crate) | zen-core | 1.4 |
| 1.4 | Implement ID prefix constants and `gen_id_sql()` helper | zen-core | 1.6 |
| 1.5 | ~~Implement `ZenConfig` with figment (turso, motherduck, r2, general sections)~~ | zen-config | **DONE** — 46/46 tests pass. Figment `Env::prefixed("ZENITH_").split("__")` handles env vars (no manual `std::env::var()`). `String` fields with empty defaults (not `Option<String>`). Added Clerk + Axiom config sections. Storage wiring helpers: `R2Config::create_secret_sql()`, `MotherDuckConfig::connection_string()`, `TursoConfig::db_name()` / `can_mint_tokens()`. All `.env` vars renamed to `ZENITH_*__*` format, existing spikes updated. `figment::Jail` for safe test isolation (Rust 2024 `set_var` is unsafe). Real `.env` values flow through figment and match spike `std::env::var()` reads. See `05-crate-designs.md` §4 for gotchas. |
| 1.6 | Write full SQL migration file (all 14 tables + 7 FTS5 + indexes + triggers) from `01-turso-data-model.md` | zen-db | 1.7 |
| 1.7 | Implement `ZenDb::open_local()`, run migrations, verify schema | zen-db | 1.8 |
| 1.8 | Implement `ZenDb::generate_id()` using Turso's `randomblob()` | zen-db | Phase 2 |

### Tests

- zen-core: Serde roundtrip for every entity, enum string representation, ID prefix correctness
- zen-config: **DONE** — 46 tests (26 unit + 10 TOML/Jail + 9 dotenv + 1 doctest). Default loads, TOML per-section, env overrides TOML, typo gotcha documented, full provider chain, real `.env` values, spike compatibility
- zen-db: Schema creation, `generate_id()` produces correct prefix format, basic INSERT+SELECT for each table

### Milestone 1

- `cargo test -p zen-core -p zen-config -p zen-db` all pass
- Database opens, schema created, IDs generate correctly
- Every entity can be inserted and queried back

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
| 2.15 | Implement JSONL trail writer: append operations to per-session `.zenith/trail/ses-xxx.jsonl` on every mutation | zen-db | Phase 5 |
| 2.16 | Implement JSONL replayer: read all trail files, replay operations to rebuild DB (`zen rebuild`) | zen-db | Phase 5 |
| 2.11 | Implement `LinkRepo`: create, delete, query by source, query by target | zen-db | Phase 5 |
| 2.12 | Implement `AuditRepo`: append (every repo method calls this), query with filters | zen-db | 2.13 |
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

- Complete CRUD layer with 14 repo modules (including StudyRepo)
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
| 3.13 | Implement `ZenLake::store_symbols()`, `store_doc_chunks()`, `register_package()` | zen-lake | 3.14 |
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
| 4.1 | Implement vector search: embed query → HNSW similarity search in DuckDB | zen-search | 4.4 |
| 4.2 | Implement FTS search: query zen-db FTS5 tables (findings, tasks, audit, etc.) | zen-search | 4.4 |
| 4.3 | Implement hybrid search: combine vector + FTS scores | zen-search | 4.4 |
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

- `zen search "async spawn"` returns ranked results from indexed packages
- `zen research registry "http client" --ecosystem rust` returns crates.io results
- Hybrid search combines vector similarity + FTS relevance

---

## 7. Phase 5: CLI Shell

**Goal**: Working `zen` binary with all commands wired up. This is the first fully usable milestone.

**Depends on**: Phase 2 (all repos), Phase 4 (search + registry)

### Tasks

| ID | Task | Crate | Blocks |
|----|------|-------|--------|
| 5.1 | Implement clap `Cli` struct with all subcommands and global flags | zen-cli | 5.2 |
| 5.2 | Implement `main.rs`: load config, init tracing, open database, dispatch commands | zen-cli | 5.3 |
| 5.3 | Implement `zen init`: detect project, parse manifest, create `.zenith/`, init DB | zen-cli | 5.4 |
| 5.4 | Implement `zen session start/end/list` | zen-cli | 5.5 |
| 5.5 | Implement knowledge commands: `zen research`, `zen finding`, `zen hypothesis`, `zen insight` (all CRUD) | zen-cli | 5.7 |
| 5.6 | Implement work commands: `zen issue`, `zen task`, `zen log`, `zen compat` | zen-cli | 5.7 |
| 5.7 | Implement linking: `zen link`, `zen unlink` | zen-cli | 5.8 |
| 5.16 | Implement study commands: `zen study create/assume/test/get/conclude/list` | zen-cli | Done |
| 5.17 | Implement `zen rebuild`: delete DB, replay all JSONL trail files, rebuild FTS5 | zen-cli | Done |
| 5.18a | Implement `zen init` `.gitignore` template (ignore DB files, track trail/ and hooks/) | zen-cli | 5.18b |
| 5.18b | Implement pre-commit hook: validate staged `.zenith/trail/*.jsonl` files via `zen hook pre-commit` (Rust validation with `serde_json`, schema checks). Thin shell wrapper with graceful fallback if `zen` not in PATH. | zen-hooks + zen-cli | 5.18e |
| 5.18c | Implement post-checkout hook: detect JSONL trail changes between old and new HEAD via `gix` tree diff, trigger `zen rebuild` or warn based on performance threshold from spike 0.13 | zen-hooks + zen-cli | 5.18e |
| 5.18d | Implement post-merge hook: detect conflict markers in JSONL files, trigger rebuild if clean merge changed trail files | zen-hooks + zen-cli | 5.18e |
| 5.18e | Implement hook installation in `zen init`: detect git repo via `gix`, detect existing hooks / `core.hooksPath`, install using strategy chosen by spike 0.13 (hookspath / symlink / chain), support `--skip-hooks` flag | zen-hooks + zen-cli | Done |
| 5.8 | Implement `zen audit` with all filters | zen-cli | 5.9 |
| 5.9 | Implement `zen whats-next` (both JSON and raw formats) | zen-cli | 5.11 |
| 5.10 | Implement `zen search` wired to SearchEngine | zen-cli | 5.11 |
| 5.11 | Implement `zen install`: clone repo, run indexing pipeline, update project_dependencies | zen-cli | 5.12 |
| 5.12 | Implement `zen onboard`: detect project, parse manifest, batch index all deps | zen-cli | 5.13 |
| 5.13 | Implement `zen wrap-up`: session summary, snapshot, audit export | zen-cli | 5.14 |
| 5.14 | Implement `zen research registry` wired to RegistryClient | zen-cli | 5.15 |
| 5.15 | Implement JSON/table/raw output formatting for all commands | zen-cli | Done |
| 5.19 | Implement `zen grep` CLI command (package mode + local mode, all flags) | zen-cli | Done |
| 5.20 | Implement `zen cache` CLI command (list, clean, stats) | zen-cli | Done |

### Tests

- Integration tests: build the binary, run commands as subprocesses, verify JSON output
- `zen init` creates `.zenith/` with valid DB
- `zen session start` → `zen finding create` → `zen audit` shows the finding creation
- `zen install <small-crate>` → `zen search` returns results from it
- `zen whats-next` returns correct state after a sequence of operations
- Error cases: invalid command, missing args, entity not found

### Milestone 5

**This is the MVP.** The `zen` binary is functional:
- Initialize a project, start sessions, track knowledge
- Install and index packages, search documentation
- Query registries, manage issues/tasks
- View audit trail, get project state with `whats-next`
- Wrap up sessions

---

## 8. Phase 6: PRD Workflow

**Goal**: Full ai-dev-tasks PRD workflow via `zen prd` commands.

**Depends on**: Phase 5 (working CLI with issues and tasks)

### Tasks

| ID | Task | Crate | Blocks |
|----|------|-------|--------|
| 6.1 | Implement `zen prd create`: creates epic issue, returns ID | zen-cli | 6.2 |
| 6.2 | Implement `zen prd update`: stores PRD markdown in issue description | zen-cli | 6.3 |
| 6.3 | Implement `zen prd tasks`: creates parent tasks linked to epic, returns list with "confirm" message | zen-cli | 6.4 |
| 6.4 | Implement `zen prd subtasks`: creates sub-tasks linked to parent via entity_links | zen-cli | 6.5 |
| 6.5 | Implement `zen prd get`: returns full PRD with tasks, progress, findings, open questions | zen-cli | 6.6 |
| 6.6 | Implement `zen prd complete`: marks epic done, creates summary audit entry | zen-cli | 6.7 |
| 6.7 | Implement `zen prd list`: lists all epic issues with progress percentages | zen-cli | Done |

### Tests

- Full PRD lifecycle: create → update → tasks → subtasks → execute → complete
- `zen prd get` returns correct progress counts (done/total tasks)
- Multi-session PRD: start PRD in session 1, complete half tasks, wrap-up, start session 2, `zen prd get` shows correct state
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
| 7.4 | Wire `zen install` to use AgentFS workspace for cloning | zen-lake | 7.5 |
| 7.5 | Wire `zen wrap-up` to snapshot AgentFS workspace | zen-cli | 7.6 |
| 7.6 | Wire `zen audit --files` to query AgentFS audit log | zen-cli | Done |

### If AgentFS Doesn't Work (0.7 failed, 0.10 executed)

| ID | Task | Crate | Blocks |
|----|------|-------|--------|
| 7.1 | Create `Workspace` trait in zen-core | zen-core | 7.2b |
| 7.2b | Implement `TempDirWorkspace` using `tempfile::TempDir` | zen-core or zen-lake | 7.4b |
| 7.4b | Wire `zen install` to use TempDirWorkspace for cloning | zen-lake | Done |

Note: without AgentFS, we skip session workspaces and file-level audit. These become future enhancements when AgentFS stabilizes.

### Tests

- Workspace creation, file read/write, deletion
- Package indexing through workspace (clone → parse → cleanup)
- Session workspace snapshot (AgentFS path only)

### Milestone 7

- Package indexing uses isolated workspaces (crash-safe)
- Session file-level audit available via `zen audit --files` (AgentFS path)

---

## 10. Phase 8: Cloud & Polish

**Goal**: Turso Cloud sync, DuckLake with MotherDuck + R2, JSONL audit export, auto-commit.

**Depends on**: Phase 5 (working local tool), Phase 0 (cloud spikes)

### Tasks

| ID | Task | Crate | Blocks |
|----|------|-------|--------|
| 8.1 | Implement `ZenDb::open_synced()` with Turso Cloud | zen-db | 8.2 |
| 8.2 | Wire `zen wrap-up` to call `ZenDb::sync()` | zen-cli | 8.5 |
| 8.3 | Implement `ZenLake::open_cloud()` with MotherDuck + R2 | zen-lake | 8.5 |
| 8.4 | ~~Implement JSONL audit trail export at wrap-up (for git)~~ | **MOVED** — JSONL trail is now real-time append in Phase 2 (tasks 2.15-2.16), not wrap-up-only export. See [10-git-jsonl-strategy.md](./10-git-jsonl-strategy.md) | N/A |
| 8.5 | ~~Implement `--auto-commit` flag on `zen wrap-up`: git add + commit~~ | **DESCOPED** — git operations are the user's/LLM's responsibility, not Zenith's. Zenith produces JSONL files; user commits them. | N/A |
| 8.6 | Implement `zen onboard` cloud mode: check DuckLake for already-indexed packages | zen-cli | 8.7 |
| 8.7 | Implement config validation: check R2/MotherDuck/Turso credentials at startup | zen-config | Done |

### Tests

- Cloud sync: create entities locally, sync, verify they appear in Turso Cloud
- DuckLake: write parquet to R2 via MotherDuck, query back
- Config validation: missing/invalid credentials produce clear error messages
- Onboard cloud mode: already-indexed packages are skipped, not re-indexed

### Milestone 8

- Full cloud sync at wrap-up
- Indexed packages shared across machines via MotherDuck + R2
- Config validation catches credential issues at startup

---

## 11. Dependency Graph

```
Phase 0: Spikes (all parallel)
    │
    ├─► Phase 1: Foundation (zen-core, zen-config, zen-db schema)
    │       │
    │       ├─► Phase 2: Storage Layer (zen-db repos, all 13 modules)
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
```

---

## 12. Risk Register

| Risk | Impact | Likelihood | Mitigation |
|------|--------|-----------|------------|
| AgentFS doesn't compile from git | Lose workspace isolation and file audit | **Mitigated** | Spike 0.7 confirmed: `agentfs-sdk` 0.6.0 works from crates.io (no git dep needed). KV, filesystem, tool tracking all validated. **New risk**: Turso docs (`agentfs = "0.1"`) don't match actual crate name (`agentfs-sdk`) or API surface (POSIX-level vs high-level). May need thin wrapper. Task 0.10 (fallback) cancelled. |
| `turso` crate API differs from docs | Blocks all DB work | **Realized** | Spike 0.2 confirmed: `turso` 0.5.0-pre.8 lacks FTS (experimental flag not exposed). **Mitigated**: switched to `libsql` 0.9.29 which has native FTS5. Plan to re-evaluate `turso` when stable. |
| DuckDB VSS extension doesn't work in Rust | Lose vector search in lake | **Partially realized** | Spike 0.5 confirmed: VSS loads and works in-memory (HNSW + cosine similarity + hybrid search). **However**: HNSW persistence is experimental and causes SIGABRT on DB reopen (DuckDB 1.4 bug). **Mitigation**: Use in-memory HNSW only; store embeddings in Parquet on R2; rebuild HNSW at query time or use brute-force `array_cosine_similarity()` (acceptable for <100K symbols). Also: Parquet `FLOAT[384]` → `FLOAT[]` requires explicit cast back. |
| fastembed model download fails or is slow | Blocks indexing pipeline | Low | Phase 0 spike (0.6). Fallback: skip embeddings, use FTS only |
| DuckLake + MotherDuck requires features not in duckdb crate | Lose cloud lake | **Mitigated** | Spike 0.5 confirmed: DuckLake works on MotherDuck with R2 custom DATA_PATH (`s3://zenith/lake/`). Created `zenith` R2 bucket (us-east-1, same region as MotherDuck). Snapshots, ACID, schema evolution all work. **Gotcha**: DuckLake does NOT support `FLOAT[N]` (fixed-size arrays). Must use `FLOAT[]` for embeddings and cast to `FLOAT[384]` at query time. Fully managed mode also works (no R2 needed). |
| Tree-sitter grammar incompatibility (local grammars for Astro/Gleam/Mojo/Markdown) | Lose 4 of 16 languages | Low | Focus on core languages (Rust, Python, TS, Go) first. Local grammars are Phase 3 stretch |
| Turso Cloud sync is slow or unreliable | Poor wrap-up experience | Low | Sync is manual (wrap-up only), can retry. Local DB always works |
| User has existing git hooks (husky, lefthook, pre-commit) | Zenith hooks fail to install or overwrite user's hooks | Medium | Spike 0.13 evaluates three installation strategies (`core.hooksPath`, symlink, chain-append). Detect existing hooks before installing. Support `--skip-hooks` flag. See [11-git-hooks-spike-plan.md](./11-git-hooks-spike-plan.md) |
| `gix` adds significant compile time | Slower builds for all developers | Medium | `gix` isolated in `zen-hooks` crate — only rebuilds when hooks code changes. Spike 0.13 measures compile time delta and identifies minimal feature flags. |
| `zen rebuild` too slow for post-checkout hook | Branch switches become sluggish | Low (< 5K ops) | Spike 0.13 measures rebuild at 100/1000/5000 ops. Threshold-based decision: auto below threshold, warn above. Configurable via `.zenith/config.toml`. |
| `zen` binary not in PATH when hooks run | Hooks skip validation silently | Medium | Wrapper approach: graceful fallback with guidance message. Pre-commit skips validation rather than blocking commit. |
| Figment silently ignores typo'd env var keys | Config appears loaded but values are defaults; hard to debug | Medium | **Confirmed** in zen-config spike. `ZENITH_TURSO__URLL` (typo) is silently ignored. Mitigation: `is_configured()` checks on every sub-config, `warn_unconfigured()` planned for CLI startup. Test `typo_env_var_silently_ignored` documents the behavior. |

---

## 13. Validation Checkpoints

At each milestone, verify:

| Milestone | Validation | Command |
|-----------|-----------|---------|
| 0 | All spikes compile and pass | `cargo test --workspace` |
| 1 | DB opens, schema created, entities insertable | `cargo test -p zen-core -p zen-config -p zen-db` |
| 2 | All 13 repos work, FTS search works, audit trail logs everything | `cargo test -p zen-db` |
| 3 | Parse Rust/Python/TS files via ast-grep, extract symbols, embed, store in DuckDB | `cargo test -p zen-parser -p zen-embeddings -p zen-lake` |
| 4 | Vector search returns relevant results, registry clients work | `cargo test -p zen-search -p zen-registry` |
| **5 (MVP)** | **`zen init` → `zen install tokio` → `zen search "spawn"` returns results. Git hooks install correctly, pre-commit validates JSONL, post-checkout detects trail changes.** | **Build binary, run e2e** |
| 6 | Full PRD lifecycle works across sessions | E2E test with sequential commands |
| 7 | Package indexing uses isolated workspaces | `cargo test -p zen-cli` (workspace tests) |
| 8 | Cloud sync works, indexed packages accessible from another machine | Manual test with Turso Cloud + MotherDuck |

### MVP Acceptance Test (Milestone 5)

This is the sequence that must work end-to-end:

```bash
# 1. Initialize
zen init

# 2. Start working
zen session start

# 3. Research
zen research create --title "Evaluate HTTP clients"
zen research registry "http client" --ecosystem rust

# 4. Install and index a package
zen install reqwest --ecosystem rust

# 5. Search indexed docs
zen search "connection pool" --package reqwest

# 6. Track knowledge
zen finding create --content "reqwest supports connection pooling" --tag verified
zen hypothesis create --content "reqwest works with tower middleware"

# 7. Create an issue
zen issue create --type feature --title "Add HTTP client layer" --priority 2

# 8. Create tasks
zen task create --title "Implement retry logic" --issue <issue-id>
zen task update <task-id> --status in_progress
zen task complete <task-id>
zen log src/http/retry.rs#1-45 --task <task-id>

# 9. Check state
zen whats-next
zen audit --limit 10

# 10. Wrap up
zen wrap-up
```

Every command must return valid JSON. Every mutation must appear in the audit trail. `zen whats-next` must reflect the current state accurately.

---

## Cross-References

- Turso data model: [01-turso-data-model.md](./01-turso-data-model.md)
- DuckLake data model: [02-ducklake-data-model.md](./02-ducklake-data-model.md)
- Architecture overview: [03-architecture-overview.md](./03-architecture-overview.md)
- CLI API design: [04-cli-api-design.md](./04-cli-api-design.md)
- Crate designs: [05-crate-designs.md](./05-crate-designs.md)
- PRD workflow: [06-prd-workflow.md](./06-prd-workflow.md)
- Git hooks spike plan: [11-git-hooks-spike-plan.md](./11-git-hooks-spike-plan.md)
- Git & JSONL strategy: [10-git-jsonl-strategy.md](./10-git-jsonl-strategy.md)
