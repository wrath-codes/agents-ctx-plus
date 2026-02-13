# Zenith Design Documents

**Project**: Zenith (`znt` CLI)
**Purpose**: Developer toolbox CLI that an LLM calls to manage project knowledge, index package documentation, and track research/findings/hypotheses/tasks
**Language**: Rust
**Created**: 2026-02-07

---

## Document Map

| # | Document | Purpose |
|---|----------|---------|
| 1 | [01-turso-data-model.md](./01-turso-data-model.md) | Complete Turso/libSQL schema: 13 tables + 6 FTS5 virtual tables + indexes + triggers |
| 2 | [02-data-architecture.md](./02-data-architecture.md) | **Data architecture**: Turso catalog (DuckLake-inspired) + Lance on R2 + DuckDB query engine. Three-tier visibility (public/team/private). Supersedes 02-ducklake-data-model.md |
| 3 | [03-architecture-overview.md](./03-architecture-overview.md) | System architecture, tech stack, data flow, design decisions, prior art |
| 4 | [04-cli-api-design.md](./04-cli-api-design.md) | Complete CLI command reference with input/output JSON formats |
| 5 | [05-crate-designs.md](./05-crate-designs.md) | Per-crate implementation guide: 9 crates with dependencies, module structure, validated patterns, key types, tests |
| 6 | [06-prd-workflow.md](./06-prd-workflow.md) | PRD workflow adapted from ai-dev-tasks: create PRD, generate tasks, execute one-by-one, integrated with Zenith data model |
| 7 | [07-implementation-plan.md](./07-implementation-plan.md) | Phased implementation: 9 phases, dependency graph, risk register, validation checkpoints, MVP acceptance test |
| 8 | [08-studies-spike-plan.md](./08-studies-spike-plan.md) | Studies feature spike plan: Approach A (compose) vs Approach B (hybrid), evaluation criteria, 15 tests — **DONE**: Approach B selected |
| 9 | [09-studies-workflow.md](./09-studies-workflow.md) | Studies workflow: structured learning lifecycle, CLI commands, data flow, multi-session persistence |
| 10 | [10-git-jsonl-strategy.md](./10-git-jsonl-strategy.md) | Git & JSONL strategy: JSONL as source of truth (beads pattern), per-session trail files, rebuild from JSONL, git hooks — **DONE**: Approach B selected, `serde-jsonlines` confirmed |
| 11 | [11-git-hooks-spike-plan.md](./11-git-hooks-spike-plan.md) | Git hooks spike plan: hook implementation (shell vs Rust vs wrapper), installation strategy (`core.hooksPath` vs symlink vs chain), post-checkout rebuild (auto vs warn), `gix` validation, session-git integration — 22 tests |
| 12 | [12-schema-spike-plan.md](./12-schema-spike-plan.md) | Schema spike plan (schemars + jsonschema) — **DONE**: 22/22 tests, zen-schema crate validated |
| 13 | [13-zen-grep-design.md](./13-zen-grep-design.md) | `znt grep` feature design: two-engine hybrid grep (DuckDB for packages, `grep`+`ignore` crates for local), `source_files` table, symbol correlation, `znt cache` — **DONE**: spike 0.14 validated, 26/26 tests |
| 14 | [14-trail-versioning-spike-plan.md](./14-trail-versioning-spike-plan.md) | Trail versioning spike plan (Approach D hybrid) — `v` field, additive evolution, version-dispatch migration, `additionalProperties` convention — **DONE**: 10/10 tests |
| 15 | [15-clerk-auth-turso-jwks-spike-plan.md](./15-clerk-auth-turso-jwks-spike-plan.md) | Clerk Auth + Turso JWKS spike — clerk-rs validation, browser callback, keyring storage, Turso JWKS integration — **DONE**: 14/14 tests |
| 16 | [16-r2-parquet-export-spike-plan.md](./16-r2-parquet-export-spike-plan.md) | R2 Lance Export spike — Parquet + Lance on R2, vector/FTS/hybrid search — **DONE**: 18/18 tests |
| 17 | [17-native-lance-spike-plan.md](./17-native-lance-spike-plan.md) | Native lancedb writes spike — lancedb Rust crate, serde_arrow production path, arrow_serde adapters — **DONE**: 10/10 tests |
| 18 | [18-catalog-visibility-spike-plan.md](./18-catalog-visibility-spike-plan.md) | Turso Catalog + Clerk Visibility spike — DuckLake-inspired catalog, three-tier search, concurrent dedup, org JWT — **DONE**: 9/9 tests |
| 21 | [21-rlm-recursive-query-spike-plan.md](./21-rlm-recursive-query-spike-plan.md) | Recursive context query spike (RLM-style) on Arrow monorepo — AST/doc symbolic recursion, categorized ref graph, external DataFusion refs — **DONE**: 17/17 tests |
| 22 | [22-decision-graph-rustworkx-spike-plan.md](./22-decision-graph-rustworkx-spike-plan.md) | Decision traces + context graph spike — decisions as first-class entities, precedent search, graph algorithms via rustworkx-core — **DONE**: 54/54 tests |
| 23 | [23-phase3-parsing-indexing-plan.md](./23-phase3-parsing-indexing-plan.md) | **Phase 3 delta plan**: zen-parser **Stream A COMPLETE** (PR1 merged — 25 dedicated extractors, `extract_api()`, `test_files.rs`, `doc_chunker.rs`, 1328 tests), zen-embeddings (pending PR2), zen-lake (pending PR3), indexing pipeline (pending PR4) |

---

## Quick Reference

```
|NAME|zenith
|CLI|znt
|LANGUAGE|rust
|STORAGE_STATE|libsql (embedded replica, sync on wrap-up only via Turso Cloud)
|STORAGE_LAKE|turso catalog (ducklake-inspired) + lance on r2 (lancedb writes) + duckdb query engine (lance ext reads)
|WORKSPACE|agentfs (from git: tursodatabase/agentfs, fallback: tempdir-based)
|EMBEDDINGS|fastembed (ONNX, 384-dim, local)
|PARSING|ast-grep (ast-grep-core + ast-grep-language, 26 built-in languages)
|JSONL_TRAIL|serde-jsonlines 0.7 (append-only per-session trail, DB rebuildable via replay)
|SEARCH|fts5 (libsql) + lance_vector_search + lance_fts + lance_hybrid_search (duckdb lance ext) + grep (ripgrep library for local) + recursive context query (rlm-style symbolic recursion) + graph analytics (rustworkx-core: toposort, centrality, shortest path, connected components)
|GREP|grep 0.4 (ripgrep library) + ignore 0.4 (gitignore-aware walking)
|ID_GENERATION|libsql/SQLite native: hex(randomblob(4)), prefixed in app layer
|ENTITIES|research_items, findings, hypotheses, insights, issues, tasks, implementation_log, compatibility_checks, studies, entity_links, audit_trail, sessions, session_snapshots, project_meta, project_dependencies, decisions
|PREFIXES|ses-, res-, fnd-, hyp-, ins-, iss-, tsk-, imp-, cmp-, stu-, lnk-, aud-, dec-
|STUDY_STATES|active, concluding, completed, abandoned
|STUDY_METHODOLOGY|explore, test-driven, compare
|ISSUE_TYPES|bug, feature, spike, epic, request
|HYPOTHESIS_STATES|unverified, analyzing, confirmed, debunked, partially_confirmed, inconclusive
|ISSUE_STATES|open, in_progress, done, blocked, abandoned
|TASK_STATES|open, in_progress, done, blocked
|RESEARCH_STATES|open, in_progress, resolved, abandoned
|SESSION_STATES|active, wrapped_up, abandoned
|CRATES|zen-cli, zen-core, zen-db, zen-lake, zen-parser, zen-embeddings, zen-registry, zen-search, zen-config, zen-hooks, zen-schema, zen-auth
|DB_CRATE|libsql 0.9.x (stable C SQLite fork; turso crate planned for future switch)
```

---

## Key Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Tool, not agent | No embedded LLM | The user's LLM orchestrates; Zenith stores and retrieves |
| libSQL for state | `libsql` crate, embedded replica + Turso Cloud | Offline-first, sync only at wrap-up |
| AgentFS for workspaces | From git (`tursodatabase/agentfs`) | CoW isolation, file-level audit, session workspaces |
| ~~DuckLake for docs~~ | ~~MotherDuck + R2~~ | **RETIRED**: Replaced by Turso catalog + Lance on R2 + DuckDB query engine. See [02-data-architecture.md](./02-data-architecture.md) |
| Turso catalog + Lance | Turso (DuckLake-inspired) + lancedb + DuckDB lance ext | Validated in spikes 0.19 (10/10) + 0.20 (9/9). Crowdsourced, three-tier visibility |
| Clerk auth | clerk-rs JWKS + org claims | No custom RBAC. JWT sub/org_id drive visibility. Validated in spikes 0.17 + 0.20 |
| serde_arrow | Rust structs → Arrow → Lance | Production write path. No DuckDB in write chain. arrow_serde adapters for DateTime |
| fastembed | Local ONNX | No API keys. 384-dim. Fast batch |
| ast-grep | 26 built-in languages | Pattern-based AST matching, composable matchers, jQuery-like traversal. 12k+ stars, wraps tree-sitter |
| SQLite-native IDs | `hex(randomblob(4))` | No external deps (sha2, uuid, base32 removed) |
| Issues as entity | Separate from research + tasks | Bugs/features/spikes/epics with parent-child hierarchy |
| JSONL as source of truth | Beads 3-layer pattern | Append-only per-session trail files, DB rebuildable from JSONL, git-friendly, no Turso Cloud required for durability |
| Git ops not our job | User/LLM handles git | We produce JSONL files + provide hooks, user commits them |
| Hypothesis lifecycle | 6 states | unverified → analyzing → confirmed/debunked/partial/inconclusive |
| Recursive context query | RLM-style symbolic recursion + reference graph categories | Spike 0.21 validated 17/17 tests on Arrow monorepo with external DataFusion evidence |
| Decision traces + context graph | Decisions as first-class entities + rustworkx-core graph algorithms | Spike 0.22 validated 54/54 tests. Precedent search, centrality, toposort, deterministic tie-break, visibility safety |

---

## Prior Art

| Project | What We Take |
|---------|-------------|
| klaw-effect-tracker | CLI tools with JSON I/O, FTS5 docs, findings with tags, two-tier extraction (ast-grep + regex), rich ParsedItem metadata |
| aether | ~~DuckDB + DuckLake + MotherDuck + R2~~ (retired), Turso embedded replicas (validated), Clerk auth patterns (Claims, JwksValidator, arrow_serde adapters — ported to zen-core), figment config (adapted for zenith), object_store |
| beads/btcab | JSONL append-only audit trail, collision-resistant IDs, git hooks |
| workflow-tool-plan | Context management research, session lifecycle, parsing strategy |
