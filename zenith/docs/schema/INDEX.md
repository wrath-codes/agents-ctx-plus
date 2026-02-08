# Zenith Design Documents

**Project**: Zenith (`zen` CLI)
**Purpose**: Developer toolbox CLI that an LLM calls to manage project knowledge, index package documentation, and track research/findings/hypotheses/tasks
**Language**: Rust
**Created**: 2026-02-07

---

## Document Map

| # | Document | Purpose |
|---|----------|---------|
| 1 | [01-turso-data-model.md](./01-turso-data-model.md) | Complete Turso/libSQL schema: 13 tables + 6 FTS5 virtual tables + indexes + triggers |
| 2 | [02-ducklake-data-model.md](./02-ducklake-data-model.md) | DuckLake schema: 3 tables (indexed_packages, api_symbols, doc_chunks) + HNSW indexes |
| 3 | [03-architecture-overview.md](./03-architecture-overview.md) | System architecture, tech stack, data flow, design decisions, prior art |
| 4 | [04-cli-api-design.md](./04-cli-api-design.md) | Complete CLI command reference with input/output JSON formats |
| 5 | [05-crate-designs.md](./05-crate-designs.md) | Per-crate implementation guide: 9 crates with dependencies, module structure, validated patterns, key types, tests |
| 6 | [06-prd-workflow.md](./06-prd-workflow.md) | PRD workflow adapted from ai-dev-tasks: create PRD, generate tasks, execute one-by-one, integrated with Zenith data model |
| 7 | [07-implementation-plan.md](./07-implementation-plan.md) | Phased implementation: 9 phases, dependency graph, risk register, validation checkpoints, MVP acceptance test |

---

## Quick Reference

```
|NAME|zenith
|CLI|zen
|LANGUAGE|rust
|STORAGE_STATE|libsql (embedded replica, sync on wrap-up only via Turso Cloud)
|STORAGE_LAKE|duckdb + ducklake + motherduck + cloudflare r2
|WORKSPACE|agentfs (from git: tursodatabase/agentfs, fallback: tempdir-based)
|EMBEDDINGS|fastembed (ONNX, 384-dim, local)
|PARSING|ast-grep (ast-grep-core + ast-grep-language, 26 built-in languages)
|SEARCH|fts5 (libsql) + hnsw vector (duckdb vss)
|ID_GENERATION|libsql/SQLite native: hex(randomblob(4)), prefixed in app layer
|ENTITIES|research_items, findings, hypotheses, insights, issues, tasks, implementation_log, compatibility_checks, entity_links, audit_trail, sessions, session_snapshots, project_meta, project_dependencies
|PREFIXES|ses-, res-, fnd-, hyp-, ins-, iss-, tsk-, imp-, cmp-, lnk-, aud-
|ISSUE_TYPES|bug, feature, spike, epic, request
|HYPOTHESIS_STATES|unverified, analyzing, confirmed, debunked, partially_confirmed, inconclusive
|ISSUE_STATES|open, in_progress, done, blocked, abandoned
|TASK_STATES|open, in_progress, done, blocked
|RESEARCH_STATES|open, in_progress, resolved, abandoned
|SESSION_STATES|active, wrapped_up, abandoned
|CRATES|zen-cli, zen-core, zen-db, zen-lake, zen-parser, zen-embeddings, zen-registry, zen-search, zen-config
|DB_CRATE|libsql 0.9.x (stable C SQLite fork; turso crate planned for future switch)
```

---

## Key Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Tool, not agent | No embedded LLM | The user's LLM orchestrates; Zenith stores and retrieves |
| libSQL for state | `libsql` crate, embedded replica + Turso Cloud | Offline-first, sync only at wrap-up |
| AgentFS for workspaces | From git (`tursodatabase/agentfs`) | CoW isolation, file-level audit, session workspaces |
| DuckLake for docs | MotherDuck + R2 | Validated in aether. Parquet native, vector search |
| fastembed | Local ONNX | No API keys. 384-dim. Fast batch |
| ast-grep | 26 built-in languages | Pattern-based AST matching, composable matchers, jQuery-like traversal. 12k+ stars, wraps tree-sitter |
| SQLite-native IDs | `hex(randomblob(4))` | No external deps (sha2, uuid, base32 removed) |
| Issues as entity | Separate from research + tasks | Bugs/features/spikes/epics with parent-child hierarchy |
| JSONL audit | Git-friendly | Append-only, merge-safe (from beads) |
| Hypothesis lifecycle | 6 states | unverified → analyzing → confirmed/debunked/partial/inconclusive |

---

## Prior Art

| Project | What We Take |
|---------|-------------|
| klaw-effect-tracker | CLI tools with JSON I/O, FTS5 docs, findings with tags, two-tier extraction (ast-grep + regex), rich ParsedItem metadata |
| aether | DuckDB + DuckLake + MotherDuck + R2 (validated), Turso embedded replicas (validated), figment config, object_store |
| beads/btcab | JSONL append-only audit trail, collision-resistant IDs, git hooks |
| workflow-tool-plan | Context management research, session lifecycle, parsing strategy |
