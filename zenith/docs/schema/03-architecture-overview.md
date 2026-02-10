# Zenith: Architecture Overview

**Version**: 2026-02-08 (v2)
**Status**: Design Document
**Purpose**: System architecture, tech stack, data flow, and design decisions

**Changes from v1**: Added AgentFS as workspace layer, `turso` crate replaces `libsql`, added `issues` entity, Turso-native ID generation.

**Changes from v3**: Switched from `turso` crate (Limbo-based, pre-release) back to `libsql` crate (C SQLite fork, stable v0.9.29). Spike 0.2 revealed that `turso` 0.5.0-pre.8 lacks FTS support — its tantivy-backed FTS is gated behind an unexposed experimental flag. `libsql` provides native FTS5, stable API, and Turso Cloud embedded replica support. Plan: re-evaluate `turso` crate once it stabilizes.

**Changes from v2**: Replaced direct tree-sitter + individual grammar crates with `ast-grep-core` + `ast-grep-language`. Reduced initial language scope from 16 to 10 (ast-grep's built-in languages only). Removed `grammars/` directory.

---

## Table of Contents

1. [What Is Zenith](#1-what-is-zenith)
2. [System Architecture](#2-system-architecture)
3. [Tech Stack](#3-tech-stack)
4. [Data Flow](#4-data-flow)
5. [Directory Structure](#5-directory-structure)
6. [Design Decisions](#6-design-decisions)
7. [ast-grep Integration](#7-ast-grep-integration)
8. [Prior Art & Influences](#8-prior-art--influences)

---

## 1. What Is Zenith

Zenith is a **developer toolbox CLI** that an LLM (any LLM, any chat interface) calls as a tool to manage project knowledge. The LLM is the brain; Zenith is the memory and the filing cabinet.

**Zenith is:**
- A Rust CLI binary (`znt`) with structured JSON input/output
- A state management system for research, findings, hypotheses, tasks, and audit trails
- A package documentation indexer (clone, parse with ast-grep, embed with fastembed, store as Lance on R2 via lancedb, catalog in Turso)
- A search engine over indexed documentation (vector + FTS)
- A session tracker that lets the LLM pick up where it left off

**Zenith is NOT:**
- An AI agent system (no embedded LLM calls, no agent orchestration)
- A replacement for the LLM (the LLM reasons, Zenith stores and retrieves)
- A build system or package manager (it indexes docs, it doesn't compile code)

### The Fundamental Loop

```
User talks to LLM
  → LLM calls `zen <command>` as a tool
    → Zenith reads/writes state, returns structured JSON
  → LLM reasons over the response
  → LLM calls another `zen <command>`
    → ...
User says "wrap up"
  → LLM calls `znt wrap-up`
    → Zenith syncs to cloud, generates summary
Next session:
  → LLM calls `znt whats-next`
    → Zenith returns project state, pending work
  → Continues where it left off
```

---

## 2. System Architecture

```
┌─────────────────────────────────────────────────────────────────────────┐
│                           USER + LLM                                     │
│                                                                          │
│  Any chat interface (OpenCode, Cursor, Amp, Claude, ChatGPT, etc.)      │
│  LLM calls `znt` commands as tools                                       │
└──────────────────────────────┬───────────────────────────────────────────┘
                               │ CLI invocation (JSON in/out)
                               ▼
┌──────────────────────────────────────────────────────────────────────────┐
│                          ZENITH CLI (znt)                                 │
│                                                                          │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐  ┌────────────┐  │
│  │  Project Mgmt │  │  Knowledge   │  │  Doc Indexer │  │  Search    │  │
│  │              │  │  Tracker     │  │              │  │  Engine    │  │
│  │ init         │  │              │  │              │  │            │  │
│  │ onboard      │  │ research     │  │ install      │  │ search     │  │
│  │ session      │  │ finding      │  │ onboard      │  │            │  │
│  │ whats-next   │  │ hypothesis   │  │              │  │            │  │
│  │ wrap-up      │  │ insight      │  │              │  │            │  │
│  │ audit        │  │ study        │  │              │  │            │  │
│  │              │  │ task         │  │              │  │            │  │
│  │              │  │ compat       │  │              │  │            │  │
│  │              │  │ link         │  │              │  │            │  │
│  │              │  │ log          │  │              │  │            │  │
│  └──────┬───────┘  └──────┬───────┘  └──────┬───────┘  └──────┬─────┘  │
│         │                 │                 │                 │          │
│         ▼                 ▼                 ▼                 ▼          │
│  ┌─────────────────────────────────┐  ┌────────────────────────────┐    │
│  │       Turso / libSQL            │  │   Lance on R2              │    │
│  │       (State + Catalog)         │  │   (Documentation Lake)     │    │
│  │                                 │  │                            │    │
│  │  • Research, Findings, Tasks    │  │  • API symbols (ast-grep)  │    │
│  │  • Hypotheses, Insights         │  │  • Doc chunks (markdown)   │    │
│  │  • Implementation Log           │  │  • Embeddings (fastembed)  │    │
│  │  • Audit Trail, Entity Links    │  │  • Vector/FTS indexes      │    │
│  │  • FTS5 search indexes          │  │                            │    │
│  │  • DuckLake-inspired catalog    │  │  Written by: lancedb crate │    │
│  │    (dl_data_file, dl_snapshot)  │  │  Read by: DuckDB lance ext │    │
│  │  • Visibility (pub/team/priv)   │  │                            │    │
│  │  Sync: wrap-up (embedded repl)  │  │  Clerk JWT auth (JWKS)     │    │
│  └────────────┬────────────────────┘  └────────────────────────────┘    │
│               │                                                          │
│               ▼                                                          │
│  ┌─────────────────────────────────┐                                     │
│  │    JSONL Trail (Git-tracked)    │                                     │
│  │  • Per-session .jsonl files     │                                     │
│  │  • Append-only operations       │                                     │
│  │  • DB rebuildable: znt rebuild  │                                     │
│  │  .zenith/trail/ses-xxx.jsonl    │                                     │
│  └─────────────────────────────────┘                                     │
└──────────────────────────────────────────────────────────────────────────┘
```

---

## 3. Tech Stack

### Core Dependencies

| Crate | Version | Purpose |
|-------|---------|---------|
| **clap** | 4.x | CLI parsing with derive macros, subcommands |
| **tokio** | 1.x | Async runtime for I/O, networking |
| **libsql** | 0.9.x | libSQL database client (local, remote, embedded replicas, native FTS5) |
| **serde-jsonlines** | 0.7.x | JSONL trail: append-only per-session files, batch read/write, DB rebuild |
| **agentfs** | git | Workspace isolation, file-level audit, CoW cloning (from `tursodatabase/agentfs`) |
| **duckdb** | 1.4.x | Analytical database (bundled) |
| **fastembed** | 5.x | Local embedding generation (ONNX, 384-dim) |
| **ast-grep-core** | 0.40.x | Pattern-based AST matching, traversal, composable matchers (wraps tree-sitter) |
| **ast-grep-language** | 0.40.x | Bundled tree-sitter grammars for 26 languages via feature flags |
| **reqwest** | 0.13.x | HTTP client (registry APIs) |
| **serde** + **serde_json** | 1.x | Serialization |
| **object_store** | 0.13.x | S3/R2 object storage |
| **chrono** | 0.4.x | Date/time handling |
| **thiserror** | 2.x | Error types |
| **anyhow** | 1.x | Error handling |
| **figment** | 0.10.x | Layered configuration |

### Supported Languages (26 built-in via ast-grep)

All grammars are bundled by `ast-grep-language` behind feature flags. No manual grammar management needed. All 26 are supported for parsing; extractors are tiered by richness.

| Language | Extractor tier | Notes |
|----------|---------------|-------|
| Rust | Rich | Generics, lifetimes, doc sections, attributes, error detection, impl blocks |
| Python | Rich | Decorators, docstrings (Google/NumPy/Sphinx), dataclass/pydantic/protocol detection |
| TypeScript | Rich | Exports, interfaces, type aliases, type parameters |
| TSX | Rich | Shares TypeScript extractor |
| JavaScript | Rich | Shares TypeScript extractor |
| Go | Rich | Exported functions/types/methods, doc comments |
| Elixir | Rich | defmodule, def/defp, defmacro |
| Bash | Basic | Function definitions, exports |
| C | Basic | Functions, structs, typedefs, macros |
| C++ | Basic | Classes, methods, templates, namespaces |
| C# | Basic | Classes, methods, interfaces, namespaces |
| CSS | Basic | Selectors, properties |
| Haskell | Basic | Type signatures, function definitions |
| HCL | Basic | Blocks, attributes |
| HTML | Basic | Tags, attributes |
| Java | Basic | Classes, methods, interfaces |
| JSON | Basic | Keys, values |
| Kotlin | Basic | Classes, functions, data classes |
| Lua | Basic | Functions, tables |
| Nix | Basic | Attribute sets, functions |
| PHP | Basic | Classes, functions, methods |
| Ruby | Basic | Classes, modules, methods |
| Scala | Basic | Classes, objects, traits, defs |
| Solidity | Basic | Contracts, functions, events |
| Swift | Basic | Classes, structs, protocols, funcs |
| YAML | Basic | Keys, values |

**Rich extractors** produce full `ParsedItem` metadata (signatures, doc comments, generics, visibility, error detection, etc.).
**Basic extractors** use a generic kind-based extraction that captures function/class/type definitions with names and signatures, but without language-specific metadata enrichment.

**Not built-in** (can be added later via ast-grep's `Language` trait for custom grammars):
Zig, Svelte, Astro, Gleam, Mojo, Markdown, TOML

### NOT Used

| Library | Why Not |
|---------|---------|
| tree-sitter (direct) | Replaced by `ast-grep-core` + `ast-grep-language` which wrap tree-sitter with pattern matching, composable matchers, and bundled grammar management |
| rig | Zenith has no embedded LLM calls -- the user's LLM is the brain |
| candle | fastembed handles embeddings via ONNX |
| tonic/tower | No gRPC server (yet -- may revisit for daemon mode) |
| graphflow | No workflow orchestration -- the LLM orchestrates |
| beads | Replaced by Zenith's own data model |
| tempolite | No durable workflow engine needed |
| turso (Limbo) | Pre-release, FTS gated behind unexposed experimental flag. Will re-evaluate when stable |
| sha2/uuid/base32 | IDs generated by Turso natively via `hex(randomblob(4))` |

---

## 4. Data Flow

### Project Initialization

```
znt init
  │
  ├─► Detect project type (Cargo.toml? package.json? go.mod?)
  ├─► Parse manifest → extract dependencies
  ├─► Create .zenith/ directory
  ├─► Initialize Turso embedded replica (main.db)
  ├─► Store project_meta and project_dependencies
  ├─► Create initial session
  └─► Return: project info, dependency count, ready state
```

### Package Indexing

```
znt install <package> [--ecosystem rust]
  │
  ├─► Check Turso catalog (already indexed? crowdsource dedup)
  ├─► Resolve package → find repo URL via registry API
  ├─► git clone --depth 1 → temp directory
  ├─► Walk source files
  │     ├─► Skip: tests, vendor, node_modules, build artifacts
  │     ├─► Detect language from extension
  │     ├─► Parse with ast-grep (SupportLang detection)
  │     ├─► Extract symbols (klaw-style rich ParsedItem)
  │     └─► Extract doc comments, attributes, metadata
  ├─► Chunk documentation files (README, docs/*)
  ├─► Generate fastembed vectors (batch)
  ├─► Write Lance to R2 via lancedb (serde_arrow → RecordBatch → lance)
  ├─► Register in Turso catalog (dl_data_file + dl_snapshot)
  ├─► Update project_dependencies (indexed = TRUE)
  ├─► Write audit trail entry
  ├─► Cleanup temp directory
  └─► Return: symbol count, file count, doc chunk count
```

### Search

```
znt search <query> [--package tokio] [--kind function] [--limit 10]
  │
  ├─► Generate fastembed vector for query
  ├─► Query DuckDB:
  │     ├─► Vector similarity search (HNSW)
  │     ├─► Apply filters (package, kind, ecosystem)
  │     └─► Return ranked results
  └─► Return: JSON array of {package, name, signature, doc, score}
```

### Knowledge Tracking

```
znt finding create --content "..." --tag deps-conflict [--research res-xxx]
  │
  ├─► Generate finding ID
  ├─► INSERT INTO findings
  ├─► INSERT INTO finding_tags (for each tag)
  ├─► INSERT INTO entity_links (if research_id provided)
  ├─► INSERT INTO audit_trail
  └─► Return: finding ID, tags, linked entities
```

### Session Lifecycle

```
znt session start
  │
  ├─► Check for orphaned active sessions → mark abandoned
  ├─► Create new session
  ├─► INSERT INTO audit_trail (session_start)
  └─► Return: session ID

znt whats-next [--limit 10]
  │
  ├─► Find last wrapped_up session → read snapshot
  ├─► Query open tasks, pending hypotheses, recent findings
  ├─► Query last N audit trail entries
  └─► Return: structured project state summary + raw entries

znt wrap-up
  │
  ├─► Generate session summary (counts, key events)
  ├─► Create session_snapshot
  ├─► Mark session as wrapped_up
  ├─► Export audit trail to JSONL (for git)
  ├─► Sync Turso embedded replica to cloud
  ├─► Optionally auto-commit (if configured)
  └─► Return: summary, sync status
```

---

## 5. Directory Structure

### Project-Level (`.zenith/`)

Created by `znt init` at the project root, alongside `.git/`:

```
.zenith/
├── config.toml            # Project configuration
├── db/
│   └── main.db            # Turso embedded replica
├── lake/
│   └── cache.duckdb       # Local DuckDB cache (optional)
├── audit/
│   └── trail.jsonl        # Append-only JSONL (git-tracked)
└── cache/
    └── clones/            # Temp directory for repo clones during indexing
```

### Rust Crate Structure

```
zenith/
├── Cargo.toml             # Workspace root
├── crates/
│   ├── zen-cli/           # CLI binary (clap)
│   ├── zen-core/          # Core types, ID generation, error types
│   ├── zen-db/            # Turso/libSQL operations
│   ├── zen-lake/          # Lance writes (lancedb) + DuckDB query engine
│   ├── zen-auth/          # Clerk auth, JWKS validation, token management
│   ├── zen-parser/        # ast-grep-based parsing + extraction
│   ├── zen-embeddings/    # fastembed integration
│   ├── zen-registry/      # Package registry HTTP clients
│   ├── zen-search/        # Search orchestration (vector + FTS)
│   ├── zen-config/        # Configuration (figment)
│   ├── zen-hooks/         # Git hooks, gix integration, session-git
│   └── zen-schema/        # JSON Schema generation and validation
└── docs/
    └── schema/            # This documentation
```

---

## 6. Design Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| **Language** | Rust | Type safety, performance, single binary distribution, aligns with reference library |
| **CLI name** | `znt` | Short, from "zenith", avoids collision with zen-browser (spike 0.13) |
| **No embedded LLM** | The user's LLM is the orchestrator | Zenith is a tool, not an agent. Any LLM can call it |
| **Turso for state** | Embedded replicas + cloud sync | Offline-first, sync only at wrap-up to avoid conflicts |
| **Lance on R2** | Turso catalog + lancedb writes + DuckDB lance ext reads | Validated in spikes 0.18-0.20. Native vector/FTS/hybrid search. Crowdsourced. MotherDuck removed. |
| **Clerk auth** | clerk-rs JWKS + org claims for visibility | No custom RBAC. JWT sub/org_id drive public/team/private scoping. |
| **fastembed** | Local ONNX embeddings | No API keys needed. 384-dim vectors. Fast batch generation |
| **ast-grep** | Pattern-based AST matching (wraps tree-sitter) | Code-like patterns, composable matchers, jQuery-like traversal, 26 built-in languages. 12k+ stars, actively maintained |
| **FTS5 in Turso** | Porter stemming search | Fast full-text search over findings, tasks, audit trail without vector overhead |
| **Lance search** | lance_vector_search + lance_fts + lance_hybrid_search | Persistent indexes, BM25 FTS, hybrid search. Replaces HNSW (crashes on persistence) |
| **JSONL audit trail** | Append-only in git | Git-friendly merge format (from beads pattern), complete history |
| **JSON I/O** | Structured CLI responses | LLM can parse structured data. No natural language parsing needed |
| **IDs** | SHA256-based short hashes | Deterministic for lake data, collision-resistant for state data, human-readable prefixes |
| **Sync strategy** | Wrap-up only | Avoids conflicts, data corruption. Single sync point per session |

### ID Format

All entity IDs use a type prefix + short hash:

| Entity | Prefix | Example |
|--------|--------|---------|
| Session | `ses-` | `ses-a3f8b2` |
| Research | `res-` | `res-c4e2d1` |
| Finding | `fnd-` | `fnd-b7a3f9` |
| Hypothesis | `hyp-` | `hyp-e1c4b2` |
| Insight | `ins-` | `ins-d2f5a8` |
| Task | `tsk-` | `tsk-f3b7c1` |
| Implementation Log | `imp-` | `imp-a8d3e2` |
| Compatibility Check | `cmp-` | `cmp-c1f4b7` |
| Study | `stu-` | `stu-a1b2c3` |
| Entity Link | `lnk-` | `lnk-e5a2d9` |
| Audit Entry | `aud-` | `aud-b3c8f1` |
| Decision | `dec-` | `dec-a1b2c3` |

---

## 7. ast-grep Integration

### Why ast-grep instead of raw tree-sitter

Zenith uses `ast-grep-core` + `ast-grep-language` instead of direct tree-sitter bindings. This gives us:

1. **Pattern-based matching**: Write code-like patterns (`fn $NAME($$$PARAMS) -> $RET { $$$ }`) instead of manual AST cursor walking
2. **jQuery-like Node API**: `node.find()`, `node.field("name")`, `node.children()`, `node.prev()`, `node.dfs()`, `node.ancestors()`
3. **Composable matchers**: `All`, `Any`, `And`, `Or`, `Not` combinators for building complex extraction rules
4. **MetaVariable capture**: Like regex capture groups but for AST nodes (`$NAME` captures single nodes, `$$$ARGS` captures multiple)
5. **Bundled grammars**: 26 languages managed via feature flags, no manual grammar version tracking
6. **Actively maintained**: 12k+ stars, 170 releases, used by CodeRabbit and Vercel

### Extraction Strategy

Zenith uses a **two-tier extraction fallback** (adapted from klaw-effect-tracker):

1. **ast-grep pattern matching + Node traversal** (preferred, most accurate): Parse source code with `SupportLang::ast_grep()`, use pattern matching and Node API to extract structured symbols with full metadata
2. **Regex** (last resort): For edge cases where ast-grep extraction returns empty results

Extractors are tiered by richness:
- **Rich extractors** (Rust, Python, TypeScript/TSX/JS, Go, Elixir): Full `ParsedItem` metadata with language-specific features
- **Generic extractor** (all other 19 built-in languages): Kind-based extraction capturing function/class/type definitions

### ParsedItem Structure

Each extracted symbol produces a rich `ParsedItem` (Rust struct, ported from klaw's TypeScript):

```rust
pub struct ParsedItem {
    pub kind: SymbolKind,         // function, struct, enum, trait, class, ...
    pub name: String,
    pub signature: String,        // Full signature line, no body
    pub source: Option<String>,   // Full source (up to 50 lines, optional)
    pub doc_comment: String,
    pub start_line: u32,
    pub end_line: u32,
    pub visibility: Visibility,
    pub metadata: SymbolMetadata, // Language-specific rich metadata
}

pub struct SymbolMetadata {
    pub is_async: bool,
    pub is_unsafe: bool,
    pub return_type: Option<String>,
    pub generics: Option<String>,
    pub attributes: Vec<String>,
    pub lifetimes: Vec<String>,
    pub where_clause: Option<String>,
    pub trait_name: Option<String>,
    pub for_type: Option<String>,
    pub variants: Vec<String>,
    pub fields: Vec<String>,
    pub methods: Vec<String>,
    pub associated_types: Vec<String>,
    pub base_classes: Vec<String>,
    pub decorators: Vec<String>,
    pub parameters: Vec<String>,
    pub is_pyo3: bool,
    pub is_pydantic: bool,
    pub is_protocol: bool,
    pub is_dataclass: bool,
    pub doc_sections: DocSections,
}

pub struct DocSections {
    pub errors: Option<String>,
    pub panics: Option<String>,
    pub safety: Option<String>,
    pub examples: Option<String>,
    pub args: HashMap<String, String>,
    pub returns: Option<String>,
    pub raises: HashMap<String, String>,
}
```

### Test File Detection

Zenith skips test files and test directories by default during indexing (configurable):

**Test directories:** `test`, `tests`, `spec`, `specs`, `__tests__`, `__mocks__`, `__snapshots__`, `testdata`, `fixtures`, `e2e`, `integration_tests`, `unit_tests`, `benches`, `benchmarks`, `examples`

**Test files:** `*_test.go`, `*_test.rs`, `*.test.{js,ts,tsx}`, `*.spec.{js,ts,tsx}`, `test_*.py`, `*_test.py`, `*_test.exs`

---

## 8. Prior Art & Influences

### klaw-effect-tracker (TypeScript)

**What we take:**
- ~40 CLI tools with JSON in/out as the LLM interface
- Extraction fallback strategy (tree-sitter WASM → CLI → regex, now replaced by ast-grep → regex)
- Rich `ParsedItem` metadata structure (async, unsafe, generics, attributes, doc sections)
- FTS5 with porter stemming for documentation search
- Findings with tags and file:line links
- Separate databases for docs (FTS) and knowledge (findings)

**What we change:**
- Rust instead of TypeScript/Bun
- ast-grep instead of raw tree-sitter WASM/CLI (pattern matching + composable matchers replace manual AST walking)
- DuckDB+DuckLake instead of SQLite for docs (vector search, cloud queryable)
- Turso instead of plain SQLite for state (cloud sync)
- fastembed instead of no embeddings (semantic search capability)
- Richer data model (research items, hypotheses, insights, compatibility checks, entity links)

### beads / btcab (Git-backed issue tracking)

**What we take:**
- JSONL append-only audit trail stored in git
- Collision-resistant ID generation (SHA256 + base32)
- Git hooks for JSONL validation and sync

**What we change:**
- Zenith is not an issue tracker -- it's a knowledge management system
- The JSONL is the audit trail only, not the primary data store (Turso is)
- No agent coordination via git branches

### aether (Rust monorepo)

**What we take:**
- Validated DuckDB + DuckLake + MotherDuck + R2 storage stack
- Validated Turso/libSQL embedded replicas
- Validated figment configuration loading (zen-config spike: `Env::prefixed("ZENITH_").split("__")`, `figment::Jail` for tests, 46/46 pass)
- object_store crate for S3/R2
- Cargo workspace structure patterns

**What we change:**
- No Arrow Flight (Zenith is a CLI, not a server)
- No gpui desktop app
- No kameo actors or dataflow-rs DAGs
- Simpler crate architecture (9 crates vs 24)

### workflow-tool-plan (Original plan)

**What we take:**
- Context management research (observation masking, AGENTS.md style, retrieval-led reasoning)
- Tree-sitter multi-language parsing strategy
- Git integration patterns
- Session lifecycle design

**What we change:**
- No embedded agents (ResearchAgent, POCAgent, etc.)
- No GraphFlow orchestration
- No Candle local LLMs
- Go implementation discarded, Rust from scratch
- Simpler architecture aligned with "tool, not agent" philosophy

---

## Cross-References

- Turso data model: [01-turso-data-model.md](./01-turso-data-model.md)
- Data architecture: [02-data-architecture.md](./02-data-architecture.md) (supersedes 02-ducklake-data-model.md)
- CLI API design: [04-cli-api-design.md](./04-cli-api-design.md)
- Original plan: `workflow-tool-plan/` directory
- Aether storage validation: `~/projects/aether/crates/aether-storage/`
- klaw-effect-tracker: `~/projects/klaw/.agents/skills/klaw-effect-tracker/`
