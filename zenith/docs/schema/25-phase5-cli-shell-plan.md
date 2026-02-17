# Phase 5: CLI Shell — Implementation Plan

**Version**: 2026-02-17
**Status**: Ready to Execute
**Depends on**: Phase 2 (zen-db — 15 repo modules, JSONL trail writer/replayer, `ZenService` — **DONE**), Phase 3 (zen-parser, zen-embeddings, zen-lake, indexing pipeline — **DONE**), Phase 4 (zen-search, zen-registry — **DONE**)
**Produces**: Milestone 5 — **The MVP.** `znt` binary is functional: initialize projects, track knowledge, search documentation, query registries, manage sessions, view audit trail, rebuild from JSONL.

> **⚠️ Scope**: Phase 5 is **CLI wiring only**. All business logic already exists in upstream crates (zen-db repos, zen-search engines, zen-registry clients, zen-lake pipeline). This phase connects clap-parsed commands to those implementations, adds output formatting, and implements git hook installation. No new algorithms or storage backends.

---

## Table of Contents

1. [Overview](#1-overview)
2. [Current State](#2-current-state-as-of-2026-02-17)
3. [Key Decisions](#3-key-decisions)
4. [Architecture](#4-architecture)
5. [PR 1 — Stream A: Core Infrastructure](#5-pr-1--stream-a-core-infrastructure)
6. [PR 2 — Stream B: Knowledge Commands](#6-pr-2--stream-b-knowledge-commands)
7. [PR 3 — Stream C: Work & Cross-Cutting Commands](#7-pr-3--stream-c-work--cross-cutting-commands)
8. [PR 4 — Stream D: Search, Registry & Indexing Commands](#8-pr-4--stream-d-search-registry--indexing-commands)
9. [PR 5 — Stream E: Git Hooks & Rebuild](#9-pr-5--stream-e-git-hooks--rebuild)
10. [PR 6 — Stream F: Workflow & Polish Commands](#10-pr-6--stream-f-workflow--polish-commands)
11. [Execution Order](#11-execution-order)
12. [Gotchas & Warnings](#12-gotchas--warnings)
13. [Milestone 5 Validation](#13-milestone-5-validation)
14. [Validation Traceability Matrix](#14-validation-traceability-matrix)

---

## 1. Overview

**Goal**: Working `znt` binary with all commands wired up to upstream crate implementations. The first fully usable milestone — after this phase, Zenith is a real tool.

**Crates touched**:
- `zen-cli` — **heavy**: all new command modules, clap structs, output formatting, `main.rs` bootstrap
- `zen-hooks` — **medium**: production modules for hook scripts, hook installation, session-git integration (promoted from spike 0.13)

**Dependency changes needed**:
- `zen-cli`: Add `zen-hooks.workspace = true` to `[dependencies]` (for `znt init` hook installation and `znt hook` subcommand)
- `zen-cli`: Add `zen-schema.workspace = true` to `[dependencies]` (for `znt schema` command via `SchemaRegistry`)
- `zen-cli`: Rename binary from `zen` to `znt` (zen-browser collision, decided in spike 0.13)
- `zen-cli`: Add `dirs.workspace = true` to `[dependencies]` (for `.zenith/` path resolution)
- `zen-cli`: Move `tempfile` from `[dev-dependencies]` to `[dependencies]` (for `install.rs` clone-to-temp directory)
- `zen-hooks`: Add `zen-schema.workspace = true` to `[dependencies]` (for pre-commit schema validation via `SchemaRegistry`)
- `zen-hooks`: Add `schemars.workspace = true` to `[dependencies]` (if needed for schema generation in hooks)

**Estimated deliverables**: ~30 new production files, ~4500–6000 LOC production code, ~1500 LOC tests

**PR strategy**: 6 PRs by stream. Streams A→B→C→D→E→F are sequential (each builds on prior), but B/C can partially overlap since they touch different command modules.

| PR | Stream | Contents | Depends On |
|----|--------|----------|------------|
| PR 1 | A: Core Infrastructure | `cli.rs` (clap structs), `main.rs` (bootstrap), `output.rs` (formatters), `AppContext` | None (clean start) |
| PR 2 | B: Knowledge Commands | `session.rs`, `research.rs`, `finding.rs`, `hypothesis.rs`, `insight.rs`, `study.rs` | Stream A |
| PR 3 | C: Work & Cross-Cutting | `issue.rs`, `task.rs`, `log.rs`, `compat.rs`, `link.rs`, `audit.rs` | Stream A |
| PR 4 | D: Search, Registry & Indexing | `search.rs`, `grep.rs`, `cache.rs`, `install.rs`, `onboard.rs`, `init.rs` (partial — project detection, `.zenith/` creation) | Streams A + B (needs session) |
| PR 5 | E: Git Hooks & Rebuild | zen-hooks production modules, `rebuild.rs`, `hook.rs` (CLI), `init.rs` (hook installation) | Streams A + D (needs init) |
| PR 6 | F: Workflow & Polish | `whats_next.rs`, `wrap_up.rs`, `schema.rs`, `warn_unconfigured()`, recursive search CLI mode | Streams A–E |

---

## 2. Current State (as of 2026-02-17)

### zen-cli — Stub Only

| Aspect | Status | Detail |
|--------|--------|--------|
| **`main.rs`** | Stub | `fn main() { println!("zen: not yet implemented") }` + spike module declarations |
| **`pipeline.rs`** | **DONE** (Phase 3) | Full indexing pipeline: walk → parse → embed → store. 487 LOC. `IndexingPipeline::index_directory()` works end-to-end. |
| **`cli.rs`** | Not started | No clap structs |
| **`commands/`** | Not started | No command handlers |
| **`output.rs`** | Not started | No formatting |
| **Cargo.toml** | Partial | Binary name is `zen` (needs rename to `znt`). Missing `zen-hooks`, `zen-schema`, `dirs`, `tempfile` (production) deps. Has `agentfs-sdk` (for Phase 7). |
| **Spikes** | 2 modules | `spike_agentfs.rs` (spike 0.7), `spike_clap.rs` (spike 0.9) |

### zen-hooks — Stub Only

| Aspect | Status | Detail |
|--------|--------|--------|
| **`lib.rs`** | Stub | Only `#[cfg(test)] mod spike_git_hooks;` |
| **Production code** | None | No production modules |
| **Spike** | Validated | `spike_git_hooks.rs` — 22/22 tests pass (spike 0.13). Hook impl, installation, gix ops, session tags all validated. |
| **Cargo.toml** | Ready | `gix`, `serde`, `serde_json`, `jsonschema`, `thiserror`, `anyhow`, `tracing` |

### Upstream Dependencies — All Ready

| Dependency | Crate | Status | Evidence |
|------------|-------|--------|----------|
| `ZenService` (all 15 repos) | zen-db | **DONE** | Session, Research, Finding, Hypothesis, Insight, Issue, Task, ImplLog, Compat, Study, Link, Audit, Project, WhatsNext repos all implemented |
| `TrailWriter` + `TrailReplayer` | zen-db | **DONE** | JSONL writer/replayer with schema validation |
| `ZenConfig` | zen-config | **DONE** | Figment-based config with env var support |
| `SchemaRegistry` | zen-schema | **DONE** | 26 schemas, validation via `jsonschema` 0.28 |
| `SearchEngine` (all modes) | zen-search | **DONE** | Vector, FTS, Hybrid, Grep, Recursive, Graph — 109+ tests |
| `RegistryClient` (11 ecosystems) | zen-registry | **DONE** | `search()`, `search_all()`, 42 tests |
| `IndexingPipeline` | zen-cli | **DONE** | `pipeline.rs` — walk → parse → embed → store |
| `ZenLake` + `SourceFileStore` | zen-lake | **DONE** | DuckDB local cache, source file storage |
| `EmbeddingEngine` | zen-embeddings | **DONE** | `embed_single()`, `embed_batch()`, 384-dim |
| `build_walker()` | zen-search | **DONE** | `WalkMode`, `.zenithignore`, skip_tests |

### Spike Code Inventory (patterns to promote)

| Spike File | Tests | Key Patterns for Production |
|------------|-------|----------------------------|
| `zen-cli/src/spike_clap.rs` | N/A | `Parser`/`Subcommand`/`ValueEnum` derive, global flags with `global = true`, nested two-level subcommands, `OutputFormat` enum |
| `zen-hooks/src/spike_git_hooks.rs` | 22 | Thin shell wrapper scripts, `serde_json` + `jsonschema` validation, gix repo discovery + config, symlink hook installation, session tag creation, tree diff for JSONL changes |

---

## 3. Key Decisions

All decisions are backed by validated spike results from Phase 0.

### 3.1 Binary Name: `znt` (Not `zen`)

**Decision**: Rename the CLI binary from `zen` to `znt`.

**Rationale**: Spike 0.13 discovered that `zen` collides with zen-browser. All hook scripts already reference `znt`. The name is short, unique, and memorable.

**Action**: Update `Cargo.toml` `[[bin]]` name from `zen` to `znt`.

**Validated in**: spike 0.13, spike 0.9 (clap derive uses `znt` as command name).

### 3.2 AppContext: Shared State Across All Commands

**Decision**: Create an `AppContext` struct that holds all initialized resources. Constructed once in `main.rs`, passed by reference to all command handlers.

**Rationale**: Multiple commands need the same resources (`ZenService`, `ZenConfig`, `ZenLake`, `SourceFileStore`, `EmbeddingEngine`, `RegistryClient`). Constructing them per-command wastes startup time and duplicates initialization logic. The context pattern is proven in aether (`aether-cli`).

```rust
pub struct AppContext {
    pub service: ZenService,
    pub config: ZenConfig,
    pub lake: ZenLake,
    pub source_store: SourceFileStore,
    pub embedder: EmbeddingEngine,
    pub registry: RegistryClient,
}
```

**Lazy initialization note**: Not all commands need all resources. `znt finding create` doesn't need `EmbeddingEngine` or `ZenLake`. Two strategies:

1. **Eager (simpler)**: Initialize everything at startup. Embedding model load is ~100ms (cached). DuckDB open is ~10ms. Total overhead ~150ms — acceptable for a CLI tool.
2. **Lazy (more complex)**: Use `OnceCell` or separate builder. More code, harder to test.

**Decision**: Eager initialization for MVP. If profiling shows >300ms startup penalty, switch to lazy.

### 3.3 Output Formatting: Trait-Based with Three Modes

**Decision**: Define an `OutputFormatter` that handles JSON, table, and raw output. All command handlers return a response struct implementing `serde::Serialize`. The formatter is applied in `main.rs` after command dispatch.

**Rationale**: Spike 0.9 validated `OutputFormat` as a `ValueEnum` enum. The CLI API design (`04-cli-api-design.md`) specifies three formats: `json` (default, for LLM consumption), `table` (human-readable), `raw` (minimal/NDJSON). Centralizing formatting avoids duplicated `serde_json::to_string_pretty()` calls across 20+ commands.

```rust
#[derive(Clone, Copy, ValueEnum)]
pub enum OutputFormat {
    Json,
    Table,
    Raw,
}

pub fn output<T: Serialize>(value: &T, format: OutputFormat) -> Result<()> {
    match format {
        OutputFormat::Json => println!("{}", serde_json::to_string_pretty(value)?),
        OutputFormat::Table => print_table(value),
        OutputFormat::Raw => println!("{}", serde_json::to_string(value)?),
    }
    Ok(())
}
```

**Table formatting**: Use manual column alignment (no heavy table library). Entities have few columns (ID, status, title/content). A simple `format!("{:<12} {:<15} {}", id, status, title)` pattern suffices.

### 3.4 Command Handler Signature: Consistent Parameters

**Decision**: All command handlers receive `&AppContext` and `&GlobalFlags` (or their needed subset). Return `Result<()>` — output is printed inside the handler via `output()`.

**Rationale**: Uniform signature makes dispatch simple. The handler is responsible for calling the appropriate repo method on `AppContext.service`, formatting the result, and printing it.

```rust
// commands/finding.rs
pub async fn handle_finding(
    action: FindingCommands,
    ctx: &AppContext,
    flags: &GlobalFlags,
) -> Result<()> {
    match action {
        FindingCommands::Create { content, source, confidence, tag, research } => {
            // Get active session (no active_session_id() helper — use list_sessions)
            let sessions = ctx.service.list_sessions(Some(SessionStatus::Active), 1).await?;
            let session_id = sessions.first().map(|s| &s.id)
                .ok_or_else(|| anyhow!("No active session. Run 'znt session start' first."))?;
            // ... create finding, call ctx.service.create_finding() ...
            output(&response, flags.format)?;
        }
        // ...
    }
    Ok(())
}
```

### 3.5 Global Flags: `global = true` with Extraction Struct

**Decision**: Global flags (`--format`, `--limit`, `--quiet`, `--verbose`, `--project`) use clap's `global = true` so they work before AND after subcommands. Extract into a `GlobalFlags` struct for ergonomic passing.

**Rationale**: Validated in spike 0.9 — global flags with `global = true` work in both positions (`znt --format table finding list` and `znt finding list --format table`).

```rust
pub struct GlobalFlags {
    pub format: OutputFormat,
    pub limit: Option<u32>,
    pub quiet: bool,
    pub verbose: bool,
    pub project: Option<String>,
}
```

### 3.6 Project Root Detection: `.zenith/` Directory Walking

**Decision**: On startup, walk up from CWD (or `--project` path) looking for a `.zenith/` directory. If not found, commands that require a project (most of them) fail with a clear error. `znt init` creates `.zenith/`.

**Rationale**: Standard CLI pattern (cargo looks for `Cargo.toml`, git looks for `.git/`). No global config needed — the project root is always discoverable.

```rust
fn find_project_root(start: &Path) -> Option<PathBuf> {
    let mut current = start.to_path_buf();
    loop {
        if current.join(".zenith").is_dir() {
            return Some(current);
        }
        if !current.pop() {
            return None;
        }
    }
}
```

### 3.7 `znt init` Creates Full `.zenith/` Structure

**Decision**: `znt init` creates the `.zenith/` directory tree, initializes the database (runs migrations), creates the `.gitignore` template, and optionally installs git hooks.

**Rationale**: Single-command setup. The `.zenith/` directory contains:

```
.zenith/
├── zenith.db           # SQLite database (gitignored)
├── zenith.db-wal       # WAL file (gitignored)
├── lake.duckdb         # DuckDB local cache (gitignored)
├── source_files.duckdb # Source file cache (gitignored)
├── config.toml         # User config (tracked)
├── trail/              # JSONL trail files (tracked)
│   └── ses-xxx.jsonl
├── hooks/              # Hook scripts (tracked)
│   ├── pre-commit
│   ├── post-checkout
│   └── post-merge
└── cache/              # Embedding model cache (gitignored)
    └── fastembed/
```

### 3.8 Error Handling: `anyhow` at CLI Boundary, Typed Errors Internally

**Decision**: Command handlers use `anyhow::Result<()>`. Internal crates use typed errors (`DatabaseError`, `SearchError`, `RegistryError`, `LakeError`). The CLI converts typed errors to `anyhow` at the boundary.

**Rationale**: `anyhow` is already in zen-cli's dependencies. CLI is the system boundary where errors become user-facing messages. Internal crate errors carry structured information; at the CLI boundary, `anyhow::Context` adds user-friendly messages.

### 3.9 Tracing: `tracing-subscriber` with `ZENITH_LOG` Env Filter

**Decision**: Initialize `tracing-subscriber` with `EnvFilter` using `ZENITH_LOG` env var. Default level: `warn`. `--verbose` bumps to `debug`. `--quiet` sets `error`.

**Rationale**: Standard Rust tracing pattern. `ZENITH_LOG=zen_db=debug,zen_search=trace` gives fine-grained control. All upstream crates already use `tracing::info!`/`debug!`/`warn!` internally.

### 3.10 Hook Implementation: Thin Shell Wrapper + `znt hook` Subcommand

**Decision**: Git hooks are thin shell scripts (2-3 lines) that call `znt hook <name>`. The Rust validation logic lives in the `znt hook` subcommand. Graceful fallback: if `znt` is not in `PATH`, the hook script exits 0 (does not block git operations).

**Rationale**: Validated in spike 0.13 (22/22 tests). Shell-only hooks can't validate JSON reliably (no `jq` by default). Rust with `serde_json` + `jsonschema` catches all edge cases. The wrapper pattern means:
- Hook scripts are trivial (version-controlled in `.zenith/hooks/`)
- All logic is in compiled Rust (testable, debuggable)
- Graceful degradation when `znt` is not installed

```bash
#!/bin/sh
# .zenith/hooks/pre-commit
command -v znt >/dev/null 2>&1 && znt hook pre-commit || true
```

### 3.11 Hook Installation: Symlink Strategy for MVP

**Decision**: `znt init` installs hooks by creating symlinks from `.git/hooks/<name>` to `.zenith/hooks/<name>`. Detects existing hooks and refuses with guidance (rather than silently overwriting).

**Rationale**: Validated in spike 0.13. Symlink strategy coexists with most setups. `core.hooksPath` available as future `--exclusive-hooks` option. Detects husky, lefthook, pre-commit framework via `core.hooksPath` config.

**Skip flag**: `znt init --skip-hooks` skips hook installation (for CI or users with incompatible hook managers).

### 3.12 `warn_unconfigured()`: Config Typo Detection at Startup

**Decision**: At CLI startup, check if figment config sections have all-default values. If so, warn the user that their environment variables may be mistyped.

**Rationale**: Validated gotcha from zen-config spike — figment silently uses defaults when env var keys don't match. A user typing `ZENITH_TURSO_URL` instead of `ZENITH_TURSO__URL` (double underscore) gets no error, just default values. The warning catches this common mistake.

```rust
fn warn_unconfigured(config: &ZenConfig) {
    if config.turso.url.is_empty() && std::env::vars().any(|(k, _)| k.starts_with("ZENITH_TURSO")) {
        tracing::warn!("Turso config has default values but ZENITH_TURSO* env vars detected. Did you use double underscores? Example: ZENITH_TURSO__URL");
    }
}
```

---

## 4. Architecture

### Module Structure (Final)

```
zen-cli/src/
├── main.rs             # Entry point: load config, init tracing, find project root,
│                       #   init AppContext, parse CLI, dispatch commands
├── cli.rs              # Clap derive structs: Cli, Commands, all subcommand enums,
│                       #   GlobalFlags extraction
├── context.rs          # AppContext: initialized resources shared across commands
├── output.rs           # OutputFormat, output(), print_table()
├── pipeline.rs         # IndexingPipeline (DONE — Phase 3)
├── commands/
│   ├── mod.rs          # Re-exports all command handlers
│   ├── init.rs         # znt init (project detect, .zenith/ creation, hook install)
│   ├── onboard.rs      # znt onboard (detect project, parse manifest, batch index)
│   ├── session.rs      # znt session {start,end,list}
│   ├── install.rs      # znt install <package> (clone, parse, index)
│   ├── search.rs       # znt search (vector/fts/hybrid/recursive/graph dispatch)
│   ├── grep.rs         # znt grep (package mode + local mode)
│   ├── cache.rs        # znt cache {list,clean,stats}
│   ├── research.rs     # znt research {create,update,list,get,registry}
│   ├── finding.rs      # znt finding {create,update,list,get,tag,untag}
│   ├── hypothesis.rs   # znt hypothesis {create,update,list,get}
│   ├── insight.rs      # znt insight {create,update,list,get}
│   ├── issue.rs        # znt issue {create,update,list,get}
│   ├── task.rs         # znt task {create,update,list,get,complete}
│   ├── log.rs          # znt log <file#lines> [--task id]
│   ├── compat.rs       # znt compat {check,list,get}
│   ├── study.rs        # znt study {create,assume,test,get,conclude,list}
│   ├── link.rs         # znt link, znt unlink
│   ├── audit.rs        # znt audit [filters]
│   ├── whats_next.rs   # znt whats-next
│   ├── wrap_up.rs      # znt wrap-up
│   ├── rebuild.rs      # znt rebuild (delete DB, replay JSONL)
│   ├── schema.rs       # znt schema <type> (dump JSON Schema)
│   └── hook.rs         # znt hook {pre-commit,post-checkout,post-merge}
└── spike_agentfs.rs    # (existing spike — Phase 7 prep)
    spike_clap.rs       # (existing spike — patterns consumed)
```

```
zen-hooks/src/
├── lib.rs              # Public API re-exports
├── error.rs            # HookError enum
├── scripts.rs          # Generate hook shell script content
├── installer.rs        # Hook installation (symlink strategy)
├── validator.rs        # Pre-commit JSONL validation (serde_json + jsonschema)
├── checkout.rs         # Post-checkout: detect JSONL changes via gix tree diff
├── merge.rs            # Post-merge: detect conflict markers, trigger rebuild
├── repo.rs             # gix repo discovery, config reading, HEAD/branch info
├── session_tags.rs     # Lightweight session tags (zenith/ses-xxx)
└── spike_git_hooks.rs  # (existing spike — patterns consumed)
```

### Dependency Flow (Phase 5)

```
zen-core (types, error hierarchy)
    │
    ├──► zen-config (configuration)
    ├──► zen-db (repos, trail, ZenService)
    ├──► zen-lake (DuckDB cache, source files)
    ├──► zen-parser (extract_api, detect_language)
    ├──► zen-embeddings (EmbeddingEngine)
    ├──► zen-registry (RegistryClient)
    ├──► zen-search (SearchEngine, GrepEngine, walk)
    ├──► zen-schema (SchemaRegistry)
    │
    ├──► zen-hooks (git hooks — THIS PHASE)
    │       │
    │       └──► gix, serde_json, jsonschema, zen-schema
    │
    └──► zen-cli (binary — THIS PHASE)
            │
            └──► ALL crates above + clap, anyhow, tracing-subscriber
```

### Startup Sequence (`main.rs`)

```
1. Parse CLI args (clap::Parser::parse())
2. Load .env (dotenvy::dotenv().ok())
3. Init tracing (tracing-subscriber, ZENITH_LOG env filter)
4. Extract GlobalFlags
5. Find project root (.zenith/ walk-up) — or skip for `init`
6. Load ZenConfig (figment)
7. warn_unconfigured() — detect config typos
8. Init AppContext (ZenService, ZenLake, SourceFileStore, EmbeddingEngine, RegistryClient)
9. Dispatch to command handler
10. Handle errors → print to stderr, exit(1)
```

**Special cases**:
- `znt init`: Does NOT require existing `.zenith/` (creates it)
- `znt rebuild`: Deletes and recreates the DB
- `znt hook <name>`: Lightweight — only needs zen-hooks, not full AppContext
- `znt schema <type>`: Only needs SchemaRegistry, not DB/Lake

---

## 5. PR 1 — Stream A: Core Infrastructure

**Tasks**: 5.1, 5.2, 5.15, 5.21
**Estimated LOC**: ~800 production, ~200 tests

Must be implemented first. All command handlers depend on these components.

### A1. Cargo.toml Updates

**`zen-cli/Cargo.toml`**:

```toml
[[bin]]
name = "znt"           # CHANGED from "zen"
path = "src/main.rs"

[dependencies]
# ... existing ...
zen-hooks.workspace = true   # NEW — hook installation + znt hook subcommand
zen-schema.workspace = true  # NEW — znt schema command (SchemaRegistry)
dirs.workspace = true        # NEW — .zenith/ path resolution
tempfile.workspace = true    # NEW — move from [dev-dependencies] to [dependencies] for install.rs clone_package
```

**Workspace `Cargo.toml`**: Add `zen-hooks` to internal dependencies if not already present.

### A2. `src/cli.rs` — Clap Derive Structs (task 5.1)

Full `Cli` struct with all subcommands and global flags. Follows spike 0.9 patterns.

```rust
use clap::{Parser, Subcommand, ValueEnum};

#[derive(Parser)]
#[command(name = "znt", version, about = "Zenith — developer knowledge toolbox")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    /// Output format
    #[arg(short, long, global = true, default_value = "json")]
    pub format: OutputFormat,

    /// Max results
    #[arg(short, long, global = true)]
    pub limit: Option<u32>,

    /// Quiet mode (suppress non-essential output)
    #[arg(short, long, global = true)]
    pub quiet: bool,

    /// Verbose mode (debug logging)
    #[arg(short, long, global = true)]
    pub verbose: bool,

    /// Project root path (default: auto-detect via .zenith/)
    #[arg(short, long, global = true)]
    pub project: Option<String>,
}

#[derive(Clone, Copy, ValueEnum)]
pub enum OutputFormat {
    Json,
    Table,
    Raw,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Initialize zenith for a project
    Init { /* flags from 04-cli-api-design.md §3 */ },
    /// Onboard existing project
    Onboard { /* flags */ },
    /// Session management
    Session { #[command(subcommand)] action: SessionCommands },
    /// Install and index a package
    Install { /* args + flags */ },
    /// Search indexed documentation
    Search { /* args + flags, including --mode for recursive */ },
    /// Regex search (package source or local files)
    Grep { /* args + flags from 13-zen-grep-design.md §5 */ },
    /// Cache management
    Cache { #[command(subcommand)] action: CacheCommands },
    /// Research items
    Research { #[command(subcommand)] action: ResearchCommands },
    /// Findings
    Finding { #[command(subcommand)] action: FindingCommands },
    /// Hypotheses
    Hypothesis { #[command(subcommand)] action: HypothesisCommands },
    /// Insights
    Insight { #[command(subcommand)] action: InsightCommands },
    /// Issues
    Issue { #[command(subcommand)] action: IssueCommands },
    /// Tasks
    Task { #[command(subcommand)] action: TaskCommands },
    /// Log implementation
    Log { /* args */ },
    /// Compatibility checks
    Compat { #[command(subcommand)] action: CompatCommands },
    /// Studies
    Study { #[command(subcommand)] action: StudyCommands },
    /// Create entity link (requires source_type, source_id, target_type, target_id, relation)
    Link { source_type: String, source_id: String, target_type: String, target_id: String, relation: String },
    /// Remove entity link
    Unlink { link_id: String },
    /// View audit trail
    Audit { /* filter flags */ },
    /// Project state and next steps
    WhatsNext,
    /// End session, sync, summarize
    WrapUp { /* flags */ },
    /// Rebuild DB from JSONL trail files
    Rebuild { /* flags */ },
    /// Dump JSON Schema for a registered type
    Schema { type_name: String },
    /// Git hook handler (called by hook scripts, not by users directly)
    Hook { #[command(subcommand)] action: HookCommands },
}
```

**Subcommand enums**: `SessionCommands`, `CacheCommands`, `ResearchCommands`, `FindingCommands`, `HypothesisCommands`, `InsightCommands`, `IssueCommands`, `TaskCommands`, `CompatCommands`, `StudyCommands`, `HookCommands` — each with their action variants and argument fields. Follow exact argument definitions from `04-cli-api-design.md`.

### A3. `src/context.rs` — AppContext (task 5.2 partial)

```rust
use std::path::PathBuf;
use zen_config::ZenConfig;
use zen_db::service::ZenService;
use zen_embeddings::EmbeddingEngine;
use zen_lake::{ZenLake, source_files::SourceFileStore};
use zen_registry::RegistryClient;

pub struct AppContext {
    pub service: ZenService,
    pub config: ZenConfig,
    pub lake: ZenLake,
    pub source_store: SourceFileStore,
    pub embedder: EmbeddingEngine,
    pub registry: RegistryClient,
    pub project_root: PathBuf,
}

impl AppContext {
    pub async fn init(project_root: PathBuf, config: ZenConfig) -> anyhow::Result<Self> {
        let zenith_dir = project_root.join(".zenith");
        let db_path = zenith_dir.join("zenith.db");
        let trail_dir = zenith_dir.join("trail");
        let lake_path = zenith_dir.join("lake.duckdb");
        let source_path = zenith_dir.join("source_files.duckdb");

        let service = ZenService::new_local(
            db_path.to_str().unwrap(),
            Some(trail_dir),
        ).await?;

        let lake = ZenLake::open_local(lake_path.to_str().unwrap())?;
        let source_store = SourceFileStore::open(source_path.to_str().unwrap())?;
        let embedder = EmbeddingEngine::new()?;  // cache dir is hardcoded to ~/.zenith/cache/fastembed/
        let registry = RegistryClient::new();

        Ok(Self {
            service, config, lake, source_store,
            embedder, registry, project_root,
        })
    }
}
```

### A4. `src/output.rs` — Output Formatting (task 5.15)

```rust
use serde::Serialize;
use crate::cli::OutputFormat;

pub fn output<T: Serialize>(value: &T, format: OutputFormat) -> anyhow::Result<()> {
    match format {
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(value)?);
        }
        OutputFormat::Table => {
            // Default table: pretty JSON (individual commands can override with custom tables)
            println!("{}", serde_json::to_string_pretty(value)?);
        }
        OutputFormat::Raw => {
            println!("{}", serde_json::to_string(value)?);
        }
    }
    Ok(())
}

/// Table formatter for entity lists.
pub fn print_entity_table(headers: &[&str], rows: &[Vec<String>]) {
    // Calculate column widths
    let widths: Vec<usize> = headers.iter().enumerate().map(|(i, h)| {
        rows.iter()
            .map(|r| r.get(i).map_or(0, String::len))
            .max()
            .unwrap_or(0)
            .max(h.len())
    }).collect();

    // Print header
    let header: String = headers.iter().zip(&widths)
        .map(|(h, w)| format!("{:<width$}", h, width = w))
        .collect::<Vec<_>>()
        .join("  ");
    println!("{header}");
    println!("{}", "-".repeat(header.len()));

    // Print rows
    for row in rows {
        let line: String = row.iter().zip(&widths)
            .map(|(val, w)| format!("{:<width$}", val, width = w))
            .collect::<Vec<_>>()
            .join("  ");
        println!("{line}");
    }
}
```

### A5. `src/main.rs` — Bootstrap (task 5.2)

```rust
use anyhow::{Context, Result};
use clap::Parser;

mod cli;
mod commands;
mod context;
mod output;
mod pipeline;

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();

    let cli = cli::Cli::parse();

    // Init tracing
    let log_level = if cli.verbose { "debug" } else if cli.quiet { "error" } else { "warn" };
    let filter = tracing_subscriber::EnvFilter::try_from_env("ZENITH_LOG")
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(log_level));
    tracing_subscriber::fmt().with_env_filter(filter).init();

    // Extract global flags
    let flags = cli::GlobalFlags {
        format: cli.format,
        limit: cli.limit,
        quiet: cli.quiet,
        verbose: cli.verbose,
        project: cli.project.clone(),
    };

    // Commands that don't need a project
    match &cli.command {
        cli::Commands::Init { .. } => {
            return commands::init::handle_init(/* ... */).await;
        }
        cli::Commands::Hook { action } => {
            return commands::hook::handle_hook(action).await;
        }
        cli::Commands::Schema { type_name } => {
            return commands::schema::handle_schema(type_name, &flags);
        }
        _ => {}
    }

    // Find project root
    let start = cli.project.as_deref()
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::env::current_dir().expect("cannot read cwd"));
    let project_root = find_project_root(&start)
        .context("Not a zenith project (no .zenith/ directory found). Run 'znt init' first.")?;

    // Load config
    let config = zen_config::ZenConfig::load()?;
    warn_unconfigured(&config);

    // Init context
    let mut ctx = context::AppContext::init(project_root, config).await
        .context("Failed to initialize zenith")?;

    // Dispatch
    commands::dispatch(cli.command, &mut ctx, &flags).await
}

fn find_project_root(start: &std::path::Path) -> Option<std::path::PathBuf> {
    let mut current = start.to_path_buf();
    loop {
        if current.join(".zenith").is_dir() {
            return Some(current);
        }
        if !current.pop() {
            return None;
        }
    }
}

fn warn_unconfigured(config: &zen_config::ZenConfig) {
    // task 5.21: detect figment config sections with all-default values
    // when matching env vars exist (typo detection)
}
```

### A6. `src/commands/mod.rs` — Dispatch

```rust
pub mod init;
pub mod session;
pub mod research;
pub mod finding;
pub mod hypothesis;
pub mod insight;
pub mod issue;
pub mod task;
pub mod log;
pub mod compat;
pub mod study;
pub mod link;
pub mod audit;
pub mod whats_next;
pub mod wrap_up;
pub mod search;
pub mod grep;
pub mod cache;
pub mod install;
pub mod onboard;
pub mod rebuild;
pub mod schema;
pub mod hook;

use crate::cli::{Commands, GlobalFlags};
use crate::context::AppContext;

pub async fn dispatch(command: Commands, ctx: &mut AppContext, flags: &GlobalFlags) -> anyhow::Result<()> {
    match command {
        Commands::Session { action } => session::handle(action, ctx, flags).await,
        Commands::Research { action } => research::handle(action, ctx, flags).await,
        Commands::Finding { action } => finding::handle(action, ctx, flags).await,
        Commands::Hypothesis { action } => hypothesis::handle(action, ctx, flags).await,
        Commands::Insight { action } => insight::handle(action, ctx, flags).await,
        Commands::Issue { action } => issue::handle(action, ctx, flags).await,
        Commands::Task { action } => task::handle(action, ctx, flags).await,
        Commands::Log { .. } => log::handle(/* .. */).await,
        Commands::Compat { action } => compat::handle(action, ctx, flags).await,
        Commands::Study { action } => study::handle(action, ctx, flags).await,
        Commands::Link { source_type, source_id, target_type, target_id, relation } => link::handle_link(&source_type, &source_id, &target_type, &target_id, &relation, ctx, flags).await,
        Commands::Unlink { link_id } => link::handle_unlink(&link_id, ctx, flags).await,
        Commands::Audit { .. } => audit::handle(/* .. */).await,
        Commands::WhatsNext => whats_next::handle(ctx, flags).await,
        Commands::WrapUp { .. } => wrap_up::handle(/* .. */).await,
        Commands::Search { .. } => search::handle(/* .. */).await,
        Commands::Grep { .. } => grep::handle(/* .. */).await,
        Commands::Cache { action } => cache::handle(action, ctx, flags).await,
        Commands::Install { .. } => install::handle(/* .. */).await,
        Commands::Onboard { .. } => onboard::handle(/* .. */).await,
        Commands::Rebuild { .. } => rebuild::handle(/* .. */).await,
        // Init, Hook, Schema handled before project root detection in main.rs
        Commands::Init { .. } | Commands::Hook { .. } | Commands::Schema { .. } => unreachable!(),
    }
}
```

### A7. Tests for Stream A

- CLI parses all subcommands without error (clap `debug_assert_no_default_values()`)
- Global flags work before and after subcommands
- `OutputFormat` enum: `json`, `table`, `raw` all parse
- `find_project_root()`: finds `.zenith/` in current dir, parent dir, returns None when absent
- `output()`: JSON output is valid JSON, raw output is single-line JSON
- `print_entity_table()`: column alignment correct for varying widths

---

## 6. PR 2 — Stream B: Knowledge Commands

**Tasks**: 5.4, 5.5, 5.16
**Depends on**: Stream A
**Estimated LOC**: ~1200 production, ~400 tests

### B1. `commands/session.rs` — Session Management (task 5.4)

```rust
pub async fn handle(action: SessionCommands, ctx: &AppContext, flags: &GlobalFlags) -> Result<()> {
    match action {
        SessionCommands::Start => {
            // start_session() returns (Session, Option<Session>) — orphan detection is built-in.
            // Any existing active session is automatically abandoned and returned as the second element.
            let (session, orphaned) = ctx.service.start_session().await?;
            output(&SessionStartResponse { session, orphaned }, flags.format)?;
        }
        SessionCommands::End { summary } => {
            // end_session() takes (session_id, summary) — no abandon flag.
            // Abandoning is handled automatically by start_session() when it detects orphans.
            let sessions = ctx.service.list_sessions(Some(SessionStatus::Active), 1).await?;
            let session_id = sessions.first().map(|s| &s.id)
                .ok_or_else(|| anyhow!("No active session"))?;
            let ended = ctx.service.end_session(session_id, &summary).await?;
            output(&ended, flags.format)?;
        }
        SessionCommands::List { status, limit } => {
            let sessions = ctx.service.list_sessions(status, limit.unwrap_or(20)).await?;
            output(&sessions, flags.format)?;
        }
    }
    Ok(())
}
```

**Response structs** (implement `Serialize`):

```rust
#[derive(Serialize)]
struct SessionStartResponse {
    session: Session,
    orphaned: Option<Session>,  // Previous active session that was auto-abandoned
}
```

### B2. `commands/research.rs` — Research CRUD (task 5.5 partial)

Pattern repeated for all knowledge entities. Each handler:
1. Matches on subcommand action
2. For `Create`: generate ID → build entity struct → call `ctx.service.create_*()` → output
3. For `Update`: build update via `*UpdateBuilder` → call `ctx.service.update_*()` → output
4. For `List`: call `ctx.service.list_*()` with filters → output
5. For `Get`: call `ctx.service.get_*()` → output
6. For `Registry` (research only): call `ctx.registry.search()` → output

```rust
ResearchCommands::Registry { query, ecosystem, limit } => {
    let limit = limit.unwrap_or(flags.limit.unwrap_or(10)) as usize;
    let results = if let Some(eco) = ecosystem {
        ctx.registry.search(&query, &eco, limit).await?
    } else {
        // search_all() returns Vec<PackageInfo> directly (not Result) — no ? operator
        ctx.registry.search_all(&query, limit).await
    };
    output(&results, flags.format)?;
}
```

### B3. `commands/finding.rs` — Finding CRUD + Tagging (task 5.5 partial)

Includes `tag` and `untag` subcommands in addition to standard CRUD:

```rust
FindingCommands::Tag { id, tag } => {
    let sessions = ctx.service.list_sessions(Some(SessionStatus::Active), 1).await?;
    let session_id = sessions.first().map(|s| &s.id)
        .ok_or_else(|| anyhow!("No active session. Run 'znt session start' first."))?;
    ctx.service.tag_finding(session_id, &id, &tag).await?;
    output(&json!({"tagged": true, "finding_id": id, "tag": tag}), flags.format)?;
}
FindingCommands::Untag { id, tag } => {
    let sessions = ctx.service.list_sessions(Some(SessionStatus::Active), 1).await?;
    let session_id = sessions.first().map(|s| &s.id)
        .ok_or_else(|| anyhow!("No active session. Run 'znt session start' first."))?;
    ctx.service.untag_finding(session_id, &id, &tag).await?;
    output(&json!({"untagged": true, "finding_id": id, "tag": tag}), flags.format)?;
}
```

### B4. `commands/hypothesis.rs` — Hypothesis CRUD + Status (task 5.5 partial)

Status transitions validated by `HypothesisRepo` (Phase 2). CLI just passes the new status:

```rust
HypothesisCommands::Update { id, status, content, reason } => {
    let sessions = ctx.service.list_sessions(Some(SessionStatus::Active), 1).await?;
    let session_id = sessions.first().map(|s| &s.id)
        .ok_or_else(|| anyhow!("No active session. Run 'znt session start' first."))?;
    let mut builder = HypothesisUpdateBuilder::new();
    if let Some(s) = status { builder = builder.status(parse_enum::<HypothesisStatus>(&s)?); }
    if let Some(c) = content { builder = builder.content(c); }
    if let Some(r) = reason { builder = builder.reason(Some(r)); }
    let hyp = ctx.service.update_hypothesis(session_id, &id, builder.build()).await?;
    output(&hyp, flags.format)?;
}
```

### B5. `commands/insight.rs` — Insight CRUD (task 5.5 partial)

Standard CRUD pattern — create, update, list, get.

### B6. `commands/study.rs` — Study Commands (task 5.16)

Maps to `StudyRepo` methods. Six subcommands per `04-cli-api-design.md` §16:

```rust
StudyCommands::Create { topic, library, methodology, summary } => { /* ... */ }
StudyCommands::Assume { id, content, evidence } => { /* ... */ }
StudyCommands::Test { id, assumption_id, result, evidence } => { /* ... */ }
StudyCommands::Get { id } => {
    // Returns full study state with progress, assumptions, findings, conclusions
    let state = ctx.service.get_study_full_state(&id).await?;
    output(&state, flags.format)?;
}
StudyCommands::Conclude { id, summary } => { /* ... */ }
StudyCommands::List { status, library, limit } => { /* ... */ }
```

### B7. Tests for Stream B

- `znt session start` → creates session, returns session info JSON
- `znt session start` with orphaned session → marks orphan, creates new
- `znt finding create --content "..." --confidence high` → creates finding with correct fields
- `znt finding tag <id> "test-result"` → adds tag
- `znt hypothesis update <id> --status confirmed` → valid transition succeeds
- `znt hypothesis update <id> --status invalid_status` → error
- `znt research list --limit 5` → returns at most 5 items
- `znt study create` → `znt study assume` → `znt study test` → `znt study get` → full lifecycle
- All responses are valid JSON

---

## 7. PR 3 — Stream C: Work & Cross-Cutting Commands

**Tasks**: 5.6, 5.7, 5.8
**Depends on**: Stream A
**Estimated LOC**: ~900 production, ~300 tests

### C1. `commands/issue.rs` — Issue CRUD (task 5.6 partial)

Issue types: `bug`, `feature`, `spike`, `epic`, `request`. Validated via `IssueType` enum.

```rust
IssueCommands::Create { title, issue_type, description, parent } => {
    let issue_type = parse_enum::<IssueType>(&issue_type.unwrap_or("feature".into()))?;
    // ...
}
IssueCommands::Get { id } => {
    // Returns issue with child issues and linked tasks
    let issue = ctx.service.get_issue(&id).await?;
    let children = ctx.service.get_child_issues(&id).await?;
    let tasks = ctx.service.get_tasks_for_issue(&id).await?;
    output(&IssueDetailResponse { issue, children, tasks }, flags.format)?;
}
```

### C2. `commands/task.rs` — Task CRUD + Complete (task 5.6 partial)

Includes `complete` subcommand as shorthand for `update --status done`:

```rust
TaskCommands::Complete { id } => {
    let sessions = ctx.service.list_sessions(Some(SessionStatus::Active), 1).await?;
    let session_id = sessions.first().map(|s| &s.id)
        .ok_or_else(|| anyhow!("No active session. Run 'znt session start' first."))?;
    let update = TaskUpdateBuilder::new().status(TaskStatus::Done).build();
    let task = ctx.service.update_task(session_id, &id, update).await?;
    output(&task, flags.format)?;
}
```

### C3. `commands/log.rs` — Implementation Log (task 5.6 partial)

Parses `file#start-end` format:

```rust
pub async fn handle(location: &str, task_id: Option<&str>, description: Option<&str>,
                     ctx: &AppContext, flags: &GlobalFlags) -> Result<()> {
    let (file_path, line_start, line_end) = parse_location(location)?;
    // service.create_impl_log(file_path, line_start, line_end, task_id, description)
}

fn parse_location(loc: &str) -> Result<(String, Option<i64>, Option<i64>)> {
    // "src/main.rs#10-20" → ("src/main.rs", Some(10), Some(20))
    // "src/main.rs#10" → ("src/main.rs", Some(10), None)
    // "src/main.rs" → ("src/main.rs", None, None)
}
```

### C4. `commands/compat.rs` — Compatibility Checks (task 5.6 partial)

```rust
CompatCommands::Check { package_a, package_b, status, notes } => {
    // Create or update compatibility check between two packages
}
```

### C5. `commands/link.rs` — Entity Linking (task 5.7)

```rust
pub async fn handle_link(source_type: &str, source_id: &str,
                          target_type: &str, target_id: &str,
                          relation: &str,
                          ctx: &AppContext, flags: &GlobalFlags) -> Result<()> {
    // create_link() takes 6 params: session_id, source_type, source_id, target_type, target_id, relation
    let sessions = ctx.service.list_sessions(Some(SessionStatus::Active), 1).await?;
    let session_id = sessions.first().map(|s| &s.id)
        .ok_or_else(|| anyhow!("No active session"))?;
    let source_type = parse_enum::<EntityType>(source_type)?;
    let target_type = parse_enum::<EntityType>(target_type)?;
    let relation = parse_enum::<Relation>(relation)?;
    let link = ctx.service.create_link(
        session_id, source_type, source_id, target_type, target_id, relation,
    ).await?;
    output(&link, flags.format)?;
    Ok(())
}

pub async fn handle_unlink(link_id: &str, ctx: &AppContext, flags: &GlobalFlags) -> Result<()> {
    let sessions = ctx.service.list_sessions(Some(SessionStatus::Active), 1).await?;
    let session_id = sessions.first().map(|s| &s.id)
        .ok_or_else(|| anyhow!("No active session"))?;
    ctx.service.delete_link(session_id, link_id).await?;
    output(&json!({"deleted": true, "link_id": link_id}), flags.format)?;
    Ok(())
}
```

### C6. `commands/audit.rs` — Audit Trail (task 5.8)

Supports all filter flags from `04-cli-api-design.md` §15:

```rust
pub async fn handle(entity_type: Option<&str>, entity_id: Option<&str>,
                     action: Option<&str>, session: Option<&str>,
                     search: Option<&str>, ctx: &AppContext, flags: &GlobalFlags) -> Result<()> {
    let limit = flags.limit.unwrap_or(50);

    if let Some(query) = search {
        // search_audit() takes (&str, u32) — limit is u32
        let entries = ctx.service.search_audit(query, limit).await?;
        output(&entries, flags.format)?;
    } else {
        // query_audit() takes &AuditFilter struct, not positional args
        let filter = AuditFilter {
            entity_type: entity_type.map(|s| parse_enum(s)).transpose()?,
            entity_id: entity_id.map(String::from),
            action: action.map(|s| parse_enum(s)).transpose()?,
            session_id: session.map(String::from),
            limit: Some(limit),
        };
        let entries = ctx.service.query_audit(&filter).await?;
        output(&entries, flags.format)?;
    }
    Ok(())
}
```

### C7. Tests for Stream C

- `znt issue create --title "Bug" --type bug` → creates issue with type `bug`
- `znt issue get <id>` → returns issue with child issues and linked tasks
- `znt task complete <id>` → status changes to `done`
- `znt log "src/main.rs#10-20" --task <id>` → creates impl log linked to task
- `znt link <src> <target> "depends_on"` → creates entity link
- `znt unlink <id>` → deletes entity link
- `znt audit --entity-type finding` → returns only finding audit entries
- `znt audit --search "created"` → FTS search works

---

## 8. PR 4 — Stream D: Search, Registry & Indexing Commands

**Tasks**: 5.3 (partial), 5.10, 5.11, 5.12, 5.14, 5.19, 5.20, 5.24
**Depends on**: Streams A + B (needs session)
**Estimated LOC**: ~1000 production, ~300 tests

### D1. `commands/search.rs` — Search Command (task 5.10, 5.24)

Wires to `SearchEngine` with mode dispatch:

```rust
pub async fn handle(query: &str, package: Option<&str>, ecosystem: Option<&str>,
                     kind: Option<&str>, mode: Option<&str>, version: Option<&str>,
                     budget: Option<u32>, ctx: &mut AppContext, flags: &GlobalFlags) -> Result<()> {
    let search_mode = match mode.unwrap_or("hybrid") {
        "vector" => SearchMode::Vector,
        "fts" => SearchMode::Fts,
        "hybrid" => SearchMode::Hybrid { alpha: 0.7 },
        "recursive" => SearchMode::Recursive,
        "graph" => SearchMode::Graph,
        other => anyhow::bail!("Unknown search mode: {other}. Valid: vector, fts, hybrid, recursive, graph"),
    };

    let filters = SearchFilters {
        package: package.map(String::from),
        ecosystem: ecosystem.map(String::from),
        kind: kind.map(String::from),
        version: version.map(String::from),
        entity_types: vec![],
        limit: Some(flags.limit.unwrap_or(20)),
        min_score: None,
    };

    let mut engine = SearchEngine::new(
        &ctx.service,
        &ctx.lake,
        &ctx.source_store,
        &mut ctx.embedder,
    );

    let results = engine.search(query, search_mode, filters).await?;
    output(&results, flags.format)?;
    Ok(())
}
```

**Recursive mode** (task 5.24): Budget flags `--max-depth`, `--max-chunks`, `--max-bytes` passed through. Output includes `summary_json` / `summary_json_pretty` per 07-implementation-plan.md task 5.24. If `--format raw`, outputs the raw JSON summary.

### D2. `commands/grep.rs` — Grep Command (task 5.19)

Two modes per `13-zen-grep-design.md` §5:

```rust
pub async fn handle(pattern: &str, paths: Vec<String>,
                     package: Option<&str>, ecosystem: Option<&str>,
                     version: Option<&str>, all_packages: bool,
                     case_insensitive: bool, word: bool, literal: bool,
                     context_lines: Option<usize>, skip_tests: bool,
                     ctx: &AppContext, flags: &GlobalFlags) -> Result<()> {
    let mut opts = GrepOptions::default();
    opts.case_insensitive = case_insensitive;
    opts.fixed_strings = literal;
    opts.word_regexp = word;
    let cl = context_lines.unwrap_or(0) as u32;
    opts.context_before = cl;
    opts.context_after = cl;
    opts.skip_tests = skip_tests;
    opts.max_count = Some(flags.limit.unwrap_or(100));

    let result = if let Some(pkg) = package {
        // Package mode: DuckDB fetch + Rust regex + symbol correlation
        let packages = vec![(
            ecosystem.unwrap_or("rust").to_string(),
            pkg.to_string(),
            version.unwrap_or("latest").to_string(),
        )];
        GrepEngine::grep_package(&ctx.source_store, &ctx.lake, pattern, &packages, &opts)?
    } else if all_packages {
        // All-packages mode — requires list_indexed_packages() (NEW — must be added to ZenLake, see §15 M17)
        let packages = ctx.lake.list_indexed_packages()?;
        GrepEngine::grep_package(&ctx.source_store, &ctx.lake, pattern, &packages, &opts)?
    } else if !paths.is_empty() {
        // Local mode: grep crate + ignore crate
        let pathbufs: Vec<PathBuf> = paths.iter().map(PathBuf::from).collect();
        GrepEngine::grep_local(pattern, &pathbufs, &opts)?
    } else {
        anyhow::bail!("Must provide --package, --all-packages, or [path...] arguments");
    };

    output(&result, flags.format)?;
    Ok(())
}
```

### D3. `commands/cache.rs` — Cache Management (task 5.20)

```rust
// NOTE: Several ZenLake/SourceFileStore methods referenced here do NOT exist yet.
// Pre-requisite tasks (add before Stream D implementation):
//   - Add list_indexed_packages() -> Vec<(String,String,String)> to ZenLake (§15 M17)
//   - Add count_indexed_packages() -> usize to ZenLake (§15 M23)
//   - Add clear() to ZenLake (§15 M20) — DELETE FROM all tables
//   - Add clear() to SourceFileStore (§15 M22) — DELETE FROM source_files

pub async fn handle(action: CacheCommands, ctx: &AppContext, flags: &GlobalFlags) -> Result<()> {
    match action {
        CacheCommands::List => {
            // list_indexed_packages() — NEW, must be added to ZenLake
            let packages = ctx.lake.list_indexed_packages()?;
            output(&packages, flags.format)?;
        }
        CacheCommands::Clean { package, ecosystem, version } => {
            if let Some(pkg) = package {
                // delete_package() takes (ecosystem, package, version) — not a single string
                let eco = ecosystem.as_deref().unwrap_or("rust");
                let ver = version.as_deref().unwrap_or("latest");
                ctx.lake.delete_package(eco, &pkg, ver)?;
                ctx.source_store.delete_package_sources(eco, &pkg, ver)?;
            } else {
                // clear() — NEW, must be added to both ZenLake and SourceFileStore
                ctx.lake.clear()?;
                ctx.source_store.clear()?;
            }
            output(&json!({"cleaned": true}), flags.format)?;
        }
        CacheCommands::Stats => {
            let lake_size = std::fs::metadata(ctx.project_root.join(".zenith/lake.duckdb"))
                .map(|m| m.len()).unwrap_or(0);
            let source_size = std::fs::metadata(ctx.project_root.join(".zenith/source_files.duckdb"))
                .map(|m| m.len()).unwrap_or(0);
            // count_indexed_packages() — NEW, must be added to ZenLake
            output(&json!({
                "lake_size_bytes": lake_size,
                "source_size_bytes": source_size,
                "total_bytes": lake_size + source_size,
                "packages": ctx.lake.count_indexed_packages()?,
            }), flags.format)?;
        }
    }
    Ok(())
}
```

### D4. `commands/install.rs` — Package Installation (task 5.11)

Clone repo → run indexing pipeline → update project_dependencies:

```rust
pub async fn handle(package: &str, ecosystem: Option<&str>, version: Option<&str>,
                     include_tests: bool, force: bool,
                     ctx: &mut AppContext, flags: &GlobalFlags) -> Result<()> {
    let ecosystem = ecosystem.unwrap_or("rust");
    let version = version.unwrap_or("latest");

    // Check if already indexed (skip unless --force)
    if !force && ctx.lake.is_package_indexed(ecosystem, package, version)? {
        output(&json!({"skipped": true, "reason": "already indexed", "package": package}), flags.format)?;
        return Ok(());
    }

    // Clone to temp directory
    let temp_dir = tempfile::tempdir()?;
    clone_package(ecosystem, package, version, temp_dir.path()).await?;

    // Run indexing pipeline
    // Note: IndexingPipeline::new() takes owned ZenLake + SourceFileStore.
    // Pre-req: Refactor IndexingPipeline to borrow (&ZenLake, &SourceFileStore)
    // so it can use resources from AppContext without moving them.
    // See D0e in task checklist.
    let pipeline = IndexingPipeline::new(&ctx.lake, &ctx.source_store);
    let result = pipeline.index_directory(
        temp_dir.path(), ecosystem, package, version,
        &mut ctx.embedder, !include_tests,
    )?;

    // Register in project dependencies — upsert_dependency() takes a full ProjectDependency struct
    ctx.service.upsert_dependency(&ProjectDependency {
        ecosystem: ecosystem.to_string(),
        name: package.to_string(),
        version: Some(version.to_string()),
        source: "znt install".to_string(),
        indexed: true,
        indexed_at: Some(chrono::Utc::now()),
    }).await?;

    output(&result, flags.format)?;
    Ok(())
}

async fn clone_package(ecosystem: &str, package: &str, version: &str, dest: &Path) -> Result<()> {
    // For MVP: shell out to git clone / cargo download based on ecosystem
    // Phase 7 replaces this with AgentFS workspace
    match ecosystem {
        "rust" => { /* git clone from crates.io repository URL */ }
        "npm" | "javascript" | "typescript" => { /* npm pack / git clone */ }
        _ => anyhow::bail!("Package installation not yet supported for ecosystem: {ecosystem}"),
    }
    Ok(())
}
```

### D5. `commands/onboard.rs` — Project Onboarding (task 5.12)

```rust
pub async fn handle(workspace: bool, root: Option<&str>,
                     skip_indexing: bool, ecosystem: Option<&str>,
                     ctx: &mut AppContext, flags: &GlobalFlags) -> Result<()> {
    let root_path = root.map(PathBuf::from)
        .unwrap_or_else(|| ctx.project_root.clone());

    // Detect project type and parse manifest
    let project_info = detect_project(&root_path)?;

    // Parse dependencies from manifest
    let deps = parse_manifest(&root_path, &project_info.ecosystem)?;

    let mut results = OnboardResults {
        project: project_info,
        dependencies: DependencyResults {
            detected: deps.len(),
            already_indexed: 0,
            newly_indexed: 0,
            failed: 0,
        },
    };

    if !skip_indexing {
        for dep in &deps {
            if ctx.lake.is_package_indexed(&dep.ecosystem, &dep.name, &dep.version.as_deref().unwrap_or("latest"))? {
                results.dependencies.already_indexed += 1;
                continue;
            }
            match install_single(dep, ctx).await {
                Ok(_) => results.dependencies.newly_indexed += 1,
                Err(e) => {
                    tracing::warn!("Failed to index {}: {e}", dep.name);
                    results.dependencies.failed += 1;
                }
            }
        }
    }

    output(&results, flags.format)?;
    Ok(())
}
```

### D6. `commands/init.rs` — Project Initialization (task 5.3, 5.18a partial)

```rust
pub async fn handle_init(name: Option<&str>, ecosystem: Option<&str>,
                          no_index: bool, skip_hooks: bool,
                          flags: &GlobalFlags) -> Result<()> {
    let project_root = std::env::current_dir()?;
    let zenith_dir = project_root.join(".zenith");

    if zenith_dir.exists() {
        anyhow::bail!(".zenith/ already exists. This project is already initialized.");
    }

    // Create directory structure
    // Note: cache/fastembed/ is NOT created here — EmbeddingEngine::new() manages its own
    // cache at ~/.zenith/cache/fastembed/ (hardcoded, not per-project)
    std::fs::create_dir_all(&zenith_dir)?;
    std::fs::create_dir_all(zenith_dir.join("trail"))?;
    std::fs::create_dir_all(zenith_dir.join("hooks"))?;

    // Create .gitignore (task 5.18a)
    let gitignore_content = "\
# Zenith — derived/binary files (rebuildable from trail/)
zenith.db
zenith.db-wal
zenith.db-shm
*.db-journal
lake.duckdb
lake.duckdb.wal
source_files.duckdb
source_files.duckdb.wal
cache/
";
    std::fs::write(zenith_dir.join(".gitignore"), gitignore_content)?;

    // Initialize database
    let db_path = zenith_dir.join("zenith.db");
    let service = ZenService::new_local(
        db_path.to_str().unwrap(),
        Some(zenith_dir.join("trail")),
    ).await?;

    // Detect project info
    let project_name = name.map(String::from)
        .or_else(|| detect_project_name(&project_root))
        .unwrap_or_else(|| project_root.file_name().unwrap().to_string_lossy().into_owned());

    let project_ecosystem = ecosystem.map(String::from)
        .or_else(|| detect_ecosystem(&project_root));

    // Save project meta — set_meta(key, value) stores key-value pairs
    service.set_meta("name", &project_name).await?;
    if let Some(ref eco) = project_ecosystem {
        service.set_meta("ecosystem", eco).await?;
    }

    // Start initial session — returns (Session, Option<Session>)
    let (session, _orphaned) = service.start_session().await?;

    // Install git hooks (task 5.18e — delegated to Stream E, but .gitignore created here)
    if !skip_hooks {
        // Hook installation deferred to Stream E (zen-hooks crate)
        // For now: generate hook scripts in .zenith/hooks/
        zen_hooks::scripts::generate_hook_scripts(&zenith_dir.join("hooks"))?;
        zen_hooks::installer::install_hooks(&project_root, &zenith_dir.join("hooks"))?;
    }

    output(&InitResponse {
        project: ProjectInfo { name: project_name, ecosystem: project_ecosystem, root_path: project_root },
        session: SessionInfo { id: session.id, status: "active".into() },
    }, flags.format)?;
    Ok(())
}
```

### D7. Tests for Stream D

- `znt search "async spawn"` → returns search results (needs indexed data)
- `znt search --mode recursive` → dispatches to recursive engine
- `znt grep "fn main" src/` → local grep returns matches with line numbers
- `znt grep --package tokio "spawn"` → package grep returns matches with symbol correlation
- `znt grep` (no args) → error message
- `znt cache list` → returns indexed packages
- `znt cache stats` → returns size info
- `znt init` → creates `.zenith/` with expected structure
- `znt init` in existing project → error
- `znt install <package>` → indexes and stores (integration test with small fixture)

---

## 9. PR 5 — Stream E: Git Hooks & Rebuild

**Tasks**: 5.17, 5.18a-e, 5.23
**Depends on**: Streams A + D (needs init)
**Estimated LOC**: ~900 production (zen-hooks) + ~300 production (zen-cli), ~400 tests

### E1. zen-hooks Cargo.toml Update

```toml
[dependencies]
# ... existing ...
zen-schema.workspace = true   # NEW — for pre-commit schema validation
```

### E2. `zen-hooks/src/error.rs` — Error Types

```rust
use thiserror::Error;

#[derive(Debug, Error)]
pub enum HookError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("json parse error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("schema validation failed: {0}")]
    SchemaValidation(String),

    #[error("git error: {0}")]
    Git(String),

    #[error("hook installation failed: {0}")]
    Installation(String),

    #[error("not a git repository")]
    NotGitRepo,
}
```

### E3. `zen-hooks/src/scripts.rs` — Hook Script Generation (task 5.18a)

Generates thin shell wrapper scripts that call `znt hook <name>`:

```rust
pub fn generate_hook_scripts(hooks_dir: &Path) -> Result<(), HookError> {
    let hooks = [
        ("pre-commit", PRE_COMMIT_SCRIPT),
        ("post-checkout", POST_CHECKOUT_SCRIPT),
        ("post-merge", POST_MERGE_SCRIPT),
    ];
    for (name, content) in hooks {
        let path = hooks_dir.join(name);
        std::fs::write(&path, content)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755))?;
        }
    }
    Ok(())
}

const PRE_COMMIT_SCRIPT: &str = r#"#!/bin/sh
# Zenith pre-commit hook: validate staged JSONL trail files
command -v znt >/dev/null 2>&1 && znt hook pre-commit || true
"#;

const POST_CHECKOUT_SCRIPT: &str = r#"#!/bin/sh
# Zenith post-checkout hook: rebuild DB if JSONL trail changed
# $1 = prev HEAD, $2 = new HEAD, $3 = branch flag
command -v znt >/dev/null 2>&1 && znt hook post-checkout "$@" || true
"#;

const POST_MERGE_SCRIPT: &str = r#"#!/bin/sh
# Zenith post-merge hook: rebuild DB if trail files changed in merge
# $1 = squash flag
command -v znt >/dev/null 2>&1 && znt hook post-merge "$@" || true
"#;
```

### E4. `zen-hooks/src/installer.rs` — Hook Installation (task 5.18e)

Symlink-based installation with conflict detection:

```rust
pub fn install_hooks(project_root: &Path, hooks_source_dir: &Path) -> Result<(), HookError> {
    let repo = gix::discover(project_root)
        .map_err(|_| HookError::NotGitRepo)?;

    // Check for core.hooksPath (husky, lefthook, etc.)
    let config = repo.config_snapshot();
    if let Some(hooks_path) = config.string("core.hooksPath") {
        tracing::warn!(
            "core.hooksPath is set to '{}'. Zenith hooks installed to .zenith/hooks/ but won't run automatically. \
             Use `git config --unset core.hooksPath` or add zenith hooks to your hook manager.",
            hooks_path
        );
    }

    let git_hooks_dir = repo.path().join("hooks");
    std::fs::create_dir_all(&git_hooks_dir)?;

    let hook_names = ["pre-commit", "post-checkout", "post-merge"];
    for name in hook_names {
        let target = git_hooks_dir.join(name);
        let source = hooks_source_dir.join(name);

        if target.exists() || target.is_symlink() {
            // Don't overwrite existing hooks
            tracing::warn!(
                "Existing hook at '{}' — skipping. Manually add: command -v znt && znt hook {name} || true",
                target.display()
            );
            continue;
        }

        #[cfg(unix)]
        std::os::unix::fs::symlink(&source, &target)
            .map_err(|e| HookError::Installation(format!("symlink {}: {e}", name)))?;

        #[cfg(not(unix))]
        std::fs::copy(&source, &target)
            .map_err(|e| HookError::Installation(format!("copy {}: {e}", name)))?;
    }

    Ok(())
}
```

### E5. `zen-hooks/src/validator.rs` — Pre-Commit Validation (tasks 5.18b, 5.23)

Validates staged `.zenith/trail/*.jsonl` files using `serde_json` + `jsonschema` with schemars-generated schemas from zen-schema:

```rust
use zen_schema::SchemaRegistry;

pub fn validate_staged_trail_files(project_root: &Path) -> Result<ValidationResult, HookError> {
    let repo = gix::discover(project_root)?;
    let schema_registry = SchemaRegistry::new();

    // Get staged files matching .zenith/trail/*.jsonl
    let staged_jsonl = get_staged_trail_files(&repo)?;

    let mut errors = Vec::new();
    let mut files_checked = 0;
    let mut ops_checked = 0;

    for file_path in &staged_jsonl {
        files_checked += 1;
        let content = std::fs::read_to_string(file_path)?;

        for (line_num, line) in content.lines().enumerate() {
            if line.trim().is_empty() { continue; }
            ops_checked += 1;

            // Parse JSON
            let value: serde_json::Value = match serde_json::from_str(line) {
                Ok(v) => v,
                Err(e) => {
                    errors.push(format!("{}:{}: invalid JSON: {e}", file_path.display(), line_num + 1));
                    continue;
                }
            };

            // Validate against trail envelope schema — validate(name, instance), no dedicated method
            if let Err(e) = schema_registry.validate("trail_operation", &value) {
                errors.push(format!("{}:{}: schema error: {e}", file_path.display(), line_num + 1));
            }
        }
    }

    Ok(ValidationResult { files_checked, ops_checked, errors })
}
```

### E6. `zen-hooks/src/checkout.rs` — Post-Checkout (task 5.18c)

Detect JSONL trail changes between old and new HEAD via `gix` tree diff:

```rust
pub fn check_trail_changes(project_root: &Path, old_head: &str, new_head: &str, is_branch: bool) -> Result<CheckoutAction, HookError> {
    if !is_branch {
        // File checkout, not branch switch — skip
        return Ok(CheckoutAction::Skip);
    }

    let repo = gix::discover(project_root)?;

    // Compare trees for .zenith/trail/ changes
    let old_tree = resolve_tree(&repo, old_head)?;
    let new_tree = resolve_tree(&repo, new_head)?;

    let trail_changed = has_trail_changes(&repo, &old_tree, &new_tree)?;

    if trail_changed {
        Ok(CheckoutAction::Rebuild)
    } else {
        Ok(CheckoutAction::Skip)
    }
}

pub enum CheckoutAction {
    Skip,
    Rebuild,
}
```

### E7. `zen-hooks/src/merge.rs` — Post-Merge (task 5.18d)

Detect conflict markers in JSONL files and trigger rebuild:

```rust
pub fn check_merge_trail(project_root: &Path) -> Result<MergeAction, HookError> {
    let trail_dir = project_root.join(".zenith").join("trail");
    if !trail_dir.exists() {
        return Ok(MergeAction::Skip);
    }

    let mut has_conflicts = false;
    let mut trail_changed = false;

    for entry in std::fs::read_dir(&trail_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().is_some_and(|e| e == "jsonl") {
            trail_changed = true;
            let content = std::fs::read_to_string(&path)?;
            if content.contains("<<<<<<<") || content.contains("=======") || content.contains(">>>>>>>") {
                has_conflicts = true;
                tracing::error!("Conflict markers in {}", path.display());
            }
        }
    }

    if has_conflicts {
        Ok(MergeAction::ConflictDetected)
    } else if trail_changed {
        Ok(MergeAction::Rebuild)
    } else {
        Ok(MergeAction::Skip)
    }
}
```

### E8. `zen-hooks/src/repo.rs` — gix Repo Utilities

```rust
pub fn discover_repo(start: &Path) -> Result<gix::Repository, HookError> {
    gix::discover(start).map_err(|_| HookError::NotGitRepo)
}

pub fn current_branch(repo: &gix::Repository) -> Option<String> {
    repo.head_ref().ok()?.map(|r| r.name().shorten().to_string())
}
```

### E9. `zen-hooks/src/session_tags.rs` — Session Git Tags

```rust
pub fn create_session_tag(project_root: &Path, session_id: &str) -> Result<(), HookError> {
    let repo = discover_repo(project_root)?;
    let tag_name = format!("zenith/ses-{session_id}");

    // Check if tag already exists (gix MustNotExist bug workaround — spike 0.13)
    if repo.find_reference(&tag_name).is_ok() {
        return Ok(()); // Already exists, idempotent
    }

    let head = repo.head_id()?;
    repo.tag_reference(tag_name, head, gix::refs::transaction::PreviousValue::MustNotExist)?;
    Ok(())
}
```

### E10. `commands/rebuild.rs` — Rebuild Command (task 5.17)

```rust
pub async fn handle(trail_dir: Option<&str>, strict: bool,
                     flags: &GlobalFlags) -> Result<()> {
    let project_root = std::env::current_dir()?;
    let zenith_dir = project_root.join(".zenith");
    let trail_path = trail_dir.map(PathBuf::from)
        .unwrap_or_else(|| zenith_dir.join("trail"));
    let db_path = zenith_dir.join("zenith.db");

    // Delete existing DB files
    for ext in &["", "-wal", "-shm"] {
        let p = db_path.with_extension(format!("db{ext}"));
        if p.exists() { std::fs::remove_file(&p)?; }
    }

    // Rebuild from JSONL — TrailReplayer::rebuild() is a static associated function (no new())
    let mut service = ZenService::new_local(
        db_path.to_str().unwrap(),
        None, // Disable trail writing during rebuild
    ).await?;

    let result = TrailReplayer::rebuild(&mut service, &trail_path, strict).await?;

    output(&result, flags.format)?;
    Ok(())
}
```

### E11. `commands/hook.rs` — Hook Subcommand (task 5.18b-d)

```rust
pub async fn handle_hook(action: &HookCommands) -> Result<()> {
    let project_root = std::env::current_dir()?;

    match action {
        HookCommands::PreCommit => {
            let result = zen_hooks::validator::validate_staged_trail_files(&project_root)?;
            if !result.errors.is_empty() {
                for err in &result.errors {
                    eprintln!("error: {err}");
                }
                std::process::exit(1);
            }
        }
        HookCommands::PostCheckout { prev_head, new_head, branch_flag } => {
            let action = zen_hooks::checkout::check_trail_changes(
                &project_root, prev_head, new_head, *branch_flag == "1",
            )?;
            if matches!(action, zen_hooks::checkout::CheckoutAction::Rebuild) {
                eprintln!("zenith: JSONL trail changed. Rebuilding database...");
                // Trigger rebuild
                handle_rebuild_internal(&project_root).await?;
            }
        }
        HookCommands::PostMerge { squash_flag } => {
            let action = zen_hooks::merge::check_merge_trail(&project_root)?;
            match action {
                zen_hooks::merge::MergeAction::ConflictDetected => {
                    eprintln!("zenith: ERROR — conflict markers detected in JSONL trail files. Resolve conflicts and run 'znt rebuild'.");
                }
                zen_hooks::merge::MergeAction::Rebuild => {
                    eprintln!("zenith: Trail files changed in merge. Rebuilding database...");
                    handle_rebuild_internal(&project_root).await?;
                }
                zen_hooks::merge::MergeAction::Skip => {}
            }
        }
    }
    Ok(())
}
```

### E12. Tests for Stream E

**zen-hooks tests**:
- Script generation: creates executable files with correct content
- Installer: creates symlinks in `.git/hooks/`, detects existing hooks
- Installer: warns on `core.hooksPath` config
- Validator: accepts valid JSONL, rejects malformed JSON, rejects schema violations
- Post-checkout: detects trail changes between two commits
- Post-merge: detects conflict markers, detects clean merge changes
- Session tag: creates `zenith/ses-xxx` tag, idempotent on duplicate

**zen-cli tests**:
- `znt rebuild` → deletes DB, replays JSONL, reports stats
- `znt hook pre-commit` → validates staged JSONL files
- Integration: `znt init` installs hooks → git commit with valid JSONL → passes
- Integration: `znt init --skip-hooks` → no hooks installed

---

## 10. PR 6 — Stream F: Workflow & Polish Commands

**Tasks**: 5.9, 5.13, 5.21, 5.22
**Depends on**: Streams A–E
**Estimated LOC**: ~500 production, ~200 tests

### F1. `commands/whats_next.rs` — Project State (task 5.9)

```rust
pub async fn handle(ctx: &AppContext, flags: &GlobalFlags) -> Result<()> {
    // whats_next() takes NO arguments — returns WhatsNextResponse with all state
    let state = ctx.service.whats_next().await?;

    match flags.format {
        OutputFormat::Json | OutputFormat::Table => {
            output(&state, flags.format)?;
        }
        OutputFormat::Raw => {
            // Raw mode: return last N audit entries for LLM consumption
            let filter = AuditFilter {
                limit: Some(flags.limit.unwrap_or(20)),
                ..Default::default()
            };
            let entries = ctx.service.query_audit(&filter).await?;
            for entry in &entries {
                println!("{}", serde_json::to_string(entry)?);
            }
        }
    }
    Ok(())
}
```

### F2. `commands/wrap_up.rs` — Session Wrap-Up (task 5.13)

```rust
pub async fn handle(summary: Option<&str>,
                     ctx: &AppContext, flags: &GlobalFlags) -> Result<()> {
    // 1. Get current active session (no active_session_id() — use list_sessions)
    let sessions = ctx.service.list_sessions(Some(SessionStatus::Active), 1).await?;
    let session_id = sessions.first().map(|s| s.id.clone())
        .ok_or_else(|| anyhow!("No active session to wrap up"))?;

    let summary_text = summary.unwrap_or("Session completed");

    // 2. Create session snapshot — create_snapshot(session_id, summary)
    let snapshot = ctx.service.create_snapshot(&session_id, summary_text).await?;

    // 3. End session — end_session(session_id, summary), no wrap_up_session()
    ctx.service.end_session(&session_id, summary_text).await?;

    // 4. Export audit for this session — query_audit(&AuditFilter)
    let filter = AuditFilter {
        session_id: Some(session_id.clone()),
        limit: Some(100),
        ..Default::default()
    };
    let audit = ctx.service.query_audit(&filter).await?;

    // 5. Cloud sync (Phase 8 — stub for now)
    let sync_status = json!({
        "status": "local_only",
        "turso_synced": false,
        "note": "Cloud sync available in Phase 8"
    });

    output(&WrapUpResponse {
        session: SessionSummary { id: session_id, status: "wrapped_up".into(), snapshot },
        audit_count: audit.len(),
        sync: sync_status,
    }, flags.format)?;

    Ok(())
}
```

### F3. `commands/schema.rs` — Schema Dump (task 5.22)

```rust
pub fn handle_schema(type_name: &str, flags: &GlobalFlags) -> Result<()> {
    let registry = zen_schema::SchemaRegistry::new();
    let schema = registry.get(type_name)
        .ok_or_else(|| anyhow::anyhow!(
            "Unknown type: '{type_name}'. Available: {}",
            registry.list().join(", ")  // list() not list_types()
        ))?;

    match flags.format {
        OutputFormat::Json | OutputFormat::Table => {
            println!("{}", serde_json::to_string_pretty(&schema)?);
        }
        OutputFormat::Raw => {
            println!("{}", serde_json::to_string(&schema)?);
        }
    }
    Ok(())
}
```

### F4. `warn_unconfigured()` Implementation (task 5.21)

```rust
fn warn_unconfigured(config: &ZenConfig) {
    let env_vars: Vec<(String, String)> = std::env::vars()
        .filter(|(k, _)| k.starts_with("ZENITH_"))
        .collect();

    if env_vars.is_empty() { return; }

    // Check each config section against env vars
    let sections = [
        ("turso", &config.turso.url, "ZENITH_TURSO"),
        ("r2", &config.r2.bucket, "ZENITH_R2"),
        ("clerk", &config.clerk.publishable_key, "ZENITH_CLERK"),
        ("axiom", &config.axiom.token, "ZENITH_AXIOM"),
    ];

    for (name, value, prefix) in sections {
        if value.is_empty() && env_vars.iter().any(|(k, _)| k.starts_with(prefix)) {
            tracing::warn!(
                "{name} config has default values but {prefix}* env vars detected. \
                 Did you use double underscores for nested keys? Example: {prefix}__URL"
            );
        }
    }
}
```

### F5. Tests for Stream F

- `znt whats-next` → returns structured project state with counts
- `znt whats-next --format raw` → returns NDJSON audit entries
- `znt wrap-up` → ends session, creates snapshot, reports status
- `znt schema finding` → dumps valid JSON Schema for Finding
- `znt schema nonexistent` → error with list of available types
- `warn_unconfigured()` detects env vars with missing double underscores

---

## 11. Execution Order

### Phase 5 Task Checklist

```
Phase 5 Prerequisites (all DONE):
  [x] Phase 2: zen-db (15 repo modules, ZenService, trail writer/replayer)
  [x] Phase 3: zen-parser (extract_api, 1328 tests)
  [x] Phase 3: zen-embeddings (EmbeddingEngine)
  [x] Phase 3: zen-lake (ZenLake, SourceFileStore, schemas)
  [x] Phase 3: zen-cli/pipeline.rs (IndexingPipeline)
  [x] Phase 4: zen-search (109+ tests, all modes)
  [x] Phase 4: zen-registry (42 tests, 11 ecosystems)

Stream A: Core Infrastructure (tasks 5.1, 5.2, 5.15, 5.21)
  [ ] A1. Update Cargo.toml: rename binary to `znt`, add zen-hooks + zen-schema + dirs + tempfile deps
  [ ] A2. Create src/cli.rs — full Cli struct with all subcommand enums
  [ ] A3. Create src/context.rs — AppContext initialization
  [ ] A4. Create src/output.rs — OutputFormat, output(), print_entity_table()
  [ ] A5. Rewrite src/main.rs — bootstrap: load config, init tracing, find root, dispatch
  [ ] A6. Create src/commands/mod.rs — dispatch function
  [ ] A7. Tests: CLI parsing, find_project_root, output formatting

Stream B: Knowledge Commands (tasks 5.4, 5.5, 5.16)
  [ ] B1. Create commands/session.rs — start, end, list
  [ ] B2. Create commands/research.rs — create, update, list, get, registry
  [ ] B3. Create commands/finding.rs — create, update, list, get, tag, untag
  [ ] B4. Create commands/hypothesis.rs — create, update, list, get
  [ ] B5. Create commands/insight.rs — create, update, list, get
  [ ] B6. Create commands/study.rs — create, assume, test, get, conclude, list
  [ ] B7. Tests: CRUD lifecycle, session management, study lifecycle

Stream C: Work & Cross-Cutting Commands (tasks 5.6, 5.7, 5.8)
  [ ] C1. Create commands/issue.rs — create, update, list, get
  [ ] C2. Create commands/task.rs — create, update, list, get, complete
  [ ] C3. Create commands/log.rs — log with file#lines parsing
  [ ] C4. Create commands/compat.rs — check, list, get
  [ ] C5. Create commands/link.rs — link, unlink
  [ ] C6. Create commands/audit.rs — query with all filters
  [ ] C7. Tests: issue hierarchy, task completion, audit filtering

Stream D: Search, Registry & Indexing (tasks 5.3, 5.10, 5.11, 5.12, 5.14, 5.19, 5.20, 5.24)
  [ ] D0a. PRE-REQ: Add list_indexed_packages() to ZenLake (M17 — needed by grep --all-packages, cache list)
  [ ] D0b. PRE-REQ: Add count_indexed_packages() to ZenLake (M23 — needed by cache stats)
  [ ] D0c. PRE-REQ: Add clear() to ZenLake (M20 — needed by cache clean --all)
  [ ] D0d. PRE-REQ: Add clear() to SourceFileStore (M22 — needed by cache clean --all)
  [ ] D0e. PRE-REQ: Refactor IndexingPipeline to borrow (&ZenLake, &SourceFileStore) instead of taking owned values (M46 — needed by install.rs/onboard.rs which use AppContext)
  [ ] D1. Create commands/search.rs — vector/fts/hybrid/recursive/graph dispatch
  [ ] D2. Create commands/grep.rs — package mode + local mode
  [ ] D3. Create commands/cache.rs — list, clean, stats
  [ ] D4. Create commands/install.rs — clone + index pipeline
  [ ] D5. Create commands/onboard.rs — detect project, parse manifest, batch index
  [ ] D6. Create commands/init.rs — .zenith/ creation, project detect, .gitignore
  [ ] D7. Tests: search dispatch, grep modes, init structure

Stream E: Git Hooks & Rebuild (tasks 5.17, 5.18a-e, 5.23)
  [ ] E1. Update zen-hooks Cargo.toml: add zen-schema dep
  [ ] E2. Create zen-hooks/src/error.rs — HookError enum
  [ ] E3. Create zen-hooks/src/scripts.rs — hook shell script generation
  [ ] E4. Create zen-hooks/src/installer.rs — symlink-based installation
  [ ] E5. Create zen-hooks/src/validator.rs — pre-commit JSONL validation
  [ ] E6. Create zen-hooks/src/checkout.rs — post-checkout tree diff
  [ ] E7. Create zen-hooks/src/merge.rs — post-merge conflict detection
  [ ] E8. Create zen-hooks/src/repo.rs — gix repo utilities
  [ ] E9. Create zen-hooks/src/session_tags.rs — lightweight session tags
  [ ] E10. Create commands/rebuild.rs — delete DB, replay JSONL
  [ ] E11. Create commands/hook.rs — pre-commit/post-checkout/post-merge dispatch
  [ ] E12. Wire hook installation into commands/init.rs
  [ ] E13. Tests: hook scripts, installer, validator, rebuild, integration

Stream F: Workflow & Polish (tasks 5.9, 5.13, 5.21, 5.22)
  [ ] F1. Create commands/whats_next.rs — project state + next steps
  [ ] F2. Create commands/wrap_up.rs — session wrap-up + snapshot
  [ ] F3. Create commands/schema.rs — JSON Schema dump
  [ ] F4. Implement warn_unconfigured() in main.rs
  [ ] F5. Tests: whats-next, wrap-up, schema dump, config typo detection
```

### Execution Sequence

```
    Stream A ──────► Stream B ──────────────────────────► Stream F
                         │                                   ▲
                         └──► Stream C ──────────────────────┤
                                                             │
                    Stream D ──────► Stream E ───────────────┘
```

- **Streams B and C** can overlap (different command modules, no file conflicts)
- **Stream D** depends on Stream A but can start as soon as A is done
- **Stream E** depends on Stream D (needs init.rs for hook wiring)
- **Stream F** is the final integration pass

---

## 12. Gotchas & Warnings

### 12.1 Binary Name Collision

The current `Cargo.toml` has `name = "zen"`. Must rename to `"znt"` (spike 0.13 decision). This affects:
- `[[bin]]` table in `zen-cli/Cargo.toml`
- All hook scripts referencing the binary
- Any documentation or test scripts

### 12.2 `EmbeddingEngine` Requires `&mut self`

`fastembed`'s `embed()` takes `&mut self`. The `SearchEngine` needs `&mut EmbeddingEngine`. This means `AppContext.embedder` needs to be passed mutably to search commands. Structure handlers carefully — `ctx` is `&mut AppContext` for search/install/onboard commands.

**Validated in**: spike 0.6, Phase 3 implementation.

### 12.3 DuckDB Is Synchronous

`ZenLake` and `SourceFileStore` use DuckDB which is synchronous. In the CLI context this is fine (no concurrent DuckDB access). If performance becomes an issue, use `tokio::task::spawn_blocking()`.

**Validated in**: spike 0.4, documented strategy.

### 12.4 `ZenService` Requires Async Runtime

All `ZenService` repo methods are `async` (libsql is async). The CLI must use `#[tokio::main]`. Use `tokio` with `features = ["full"]` (already in workspace deps).

### 12.5 `agentfs-sdk` in Dependencies

`zen-cli/Cargo.toml` already has `agentfs-sdk.workspace = true`. This is for Phase 7. Keep it but don't use it in Phase 5 code. It should not cause compilation issues.

### 12.6 `pipeline.rs` Already Exists

The `IndexingPipeline` in `pipeline.rs` is Phase 3 code and is already done. Phase 5's `install.rs` and `onboard.rs` call into it. Do NOT rewrite `pipeline.rs` — use it as-is.

### 12.7 Global Flags Must Use `global = true`

Without `global = true`, flags before the subcommand don't work (`znt --format table finding list` fails). This was validated in spike 0.9.

### 12.8 `figment::Jail` for Config Test Isolation

Config tests that set env vars must use `figment::Jail` (or equivalent isolation). Rust 2024 edition makes `std::env::set_var()` unsafe. This was validated in zen-config spike.

### 12.9 Hook Scripts Need Executable Permission

On Unix, generated hook scripts must have `0o755` permissions. The `scripts.rs` module handles this, but tests on non-Unix platforms need to account for the difference.

### 12.10 gix `MustNotExist` Bug

`gix 0.70` `PreviousValue::MustNotExist` does NOT reject duplicate refs. Always use `find_reference()` to check existence before creating session tags.

**Validated in**: spike 0.13.

### 12.11 `main.rs` Module Declarations

Current `main.rs` has `mod pipeline;` and spike module declarations. The rewrite must preserve `mod pipeline;` and remove spike declarations (move to `#[cfg(test)]` or keep as-is if needed).

---

## 13. Milestone 5 Validation

### Validation Commands

```bash
# Full workspace build
cargo build --workspace

# Phase 5 specific tests
cargo test -p zen-cli
cargo test -p zen-hooks

# Clippy
cargo clippy -p zen-cli -p zen-hooks --no-deps -- -D warnings

# Binary exists and runs
cargo build -p zen-cli
./target/debug/znt --help
./target/debug/znt --version
```

### Integration Test Sequence

The following sequence must work end-to-end:

```bash
# 1. Initialize project
cd /tmp/test-project && git init && cargo init
znt init

# 2. Verify structure
ls .zenith/            # trail/, hooks/, .gitignore, zenith.db
ls .git/hooks/         # pre-commit -> ../../.zenith/hooks/pre-commit (symlink)

# 3. Start session
znt session start

# 4. Create knowledge entities
znt finding create --content "reqwest supports tower middleware"
znt finding create --content "tokio spawn requires Send" --confidence high
znt hypothesis create --content "tower-http has built-in retry" --status unverified
znt insight create --content "reqwest + tower is the best HTTP stack for our use case"

# 5. Track work
znt issue create --title "Implement HTTP client" --type feature
znt task create --title "Test reqwest + tower" --issue <issue-id>
znt task complete <task-id>
znt log "src/http.rs#10-50" --task <task-id>

# 6. Link entities
znt link <finding-id> <hypothesis-id> "supports"

# 7. Query state
znt audit --entity-type finding
znt whats-next
znt finding list --format table

# 8. Search (requires indexed packages)
znt install tokio --ecosystem rust
znt search "spawn task" --mode hybrid
znt grep "async fn" --package tokio

# 9. Cache management
znt cache list
znt cache stats

# 10. Study workflow
znt study create --topic "tokio spawn semantics" --library tokio
znt study assume <study-id> --content "spawn requires Send"
znt study conclude <study-id> --summary "Confirmed: Send + 'static required"

# 11. Schema inspection
znt schema finding
znt schema trail_operation

# 12. Rebuild test
rm .zenith/zenith.db
znt rebuild
znt finding list    # All findings still present

# 13. Wrap up
znt wrap-up

# 14. Git integration
git add .zenith/trail/
git commit -m "session 1"    # pre-commit hook validates JSONL
```

### Test Count Targets

| Crate | Expected Tests | Coverage |
|-------|---------------|----------|
| `zen-cli` | ~60-80 | CLI parsing, command handlers, output formatting, integration |
| `zen-hooks` | ~30-40 | Hook scripts, installer, validator, gix ops, session tags |
| **Total** | ~90-120 | |

### Milestone 5 Acceptance Criteria

- [ ] `cargo build -p zen-cli` produces `znt` binary
- [ ] `cargo test -p zen-cli -p zen-hooks` — all tests pass
- [ ] `cargo clippy -p zen-cli -p zen-hooks --no-deps -- -D warnings` — clean
- [ ] `znt init` creates `.zenith/` with valid structure
- [ ] `znt session start` → knowledge CRUD → `znt audit` → `znt whats-next` works
- [ ] `znt install <package>` → `znt search` returns results
- [ ] `znt rebuild` recreates DB from JSONL trail
- [ ] `znt grep` works in both package and local modes
- [ ] Git hooks install and validate JSONL on commit
- [ ] All output respects `--format json|table|raw`
- [ ] `--verbose` enables debug logging, `--quiet` suppresses non-essential output
- [ ] `warn_unconfigured()` detects figment config typos

---

## 14. Validation Traceability Matrix

This matrix maps Phase 5 behaviors to their validation evidence.

| Area | Claim | Status | Spike/Test Evidence | Source |
|------|-------|--------|---------------------|--------|
| Clap derive | `Parser`/`Subcommand`/`ValueEnum` derive work | Validated | spike 0.9 | `zen-cli/src/spike_clap.rs` |
| Global flags | `global = true` works before AND after subcommand | Validated | spike 0.9 | `zen-cli/src/spike_clap.rs` |
| Nested subcommands | Two-level subcommands (`znt finding create`) work | Validated | spike 0.9 | `zen-cli/src/spike_clap.rs` |
| OutputFormat enum | `ValueEnum` restricts to json/table/raw | Validated | spike 0.9 | `zen-cli/src/spike_clap.rs` |
| Hook shell wrapper | `command -v znt && znt hook <name> \|\| true` pattern | Validated | spike 0.13 | `zen-hooks/src/spike_git_hooks.rs` |
| JSONL validation | serde_json + jsonschema catches all edge cases | Validated | spike 0.13 (22 tests) | `zen-hooks/src/spike_git_hooks.rs` |
| Hook installation | Symlink strategy, conflict detection | Validated | spike 0.13 | `zen-hooks/src/spike_git_hooks.rs` |
| gix repo discovery | `gix::discover()` finds repo from subdirectory | Validated | spike 0.13 | `zen-hooks/src/spike_git_hooks.rs` |
| Session tags | `zenith/ses-xxx` lightweight tags via gix | Validated | spike 0.13 | `zen-hooks/src/spike_git_hooks.rs` |
| gix tree diff | Post-checkout JSONL change detection | Validated | spike 0.13 | `zen-hooks/src/spike_git_hooks.rs` |
| gix MustNotExist bug | Use `find_reference()` before tag creation | Validated | spike 0.13 | `zen-hooks/src/spike_git_hooks.rs` |
| Config typo detection | figment uses defaults when env keys don't match | Validated | zen-config spike (46 tests) | `zen-config/src/` |
| IndexingPipeline | walk → parse → embed → store works end-to-end | Validated | Phase 3 | `zen-cli/src/pipeline.rs` |
| ZenService CRUD | All 15 repo modules with mutation protocol | Validated | Phase 2 | `zen-db/src/repos/` |
| TrailReplayer | DB rebuilds from JSONL trail | Validated | Phase 2, spike 0.12 | `zen-db/src/trail/replayer.rs` |
| SearchEngine dispatch | All modes (Vector/FTS/Hybrid/Recursive/Graph) | Validated | Phase 4 (109+ tests) | `zen-search/src/` |
| RegistryClient | 11 ecosystem search | Validated | Phase 4 (42 tests) | `zen-registry/src/` |
| GrepEngine | Package + local modes | Validated | Phase 4, spike 0.14 | `zen-search/src/grep.rs` |
| SchemaRegistry | 26 schemas, validation | Validated | Phase 1 (42 tests) | `zen-schema/src/` |
| AgentFS | agentfs-sdk 0.6.0 works | Validated | spike 0.7 | `zen-cli/src/spike_agentfs.rs` |

---

## Cross-References

- CLI API design: [04-cli-api-design.md](./04-cli-api-design.md)
- Crate designs (zen-cli §11): [05-crate-designs.md](./05-crate-designs.md)
- Implementation plan (Phase 5 tasks): [07-implementation-plan.md](./07-implementation-plan.md) §7
- Git hooks spike: [11-git-hooks-spike-plan.md](./11-git-hooks-spike-plan.md)
- Grep design: [13-zen-grep-design.md](./13-zen-grep-design.md)
- JSONL strategy: [10-git-jsonl-strategy.md](./10-git-jsonl-strategy.md)
- Phase 2 plan (ZenService, repos): [20-phase2-storage-layer-plan.md](./20-phase2-storage-layer-plan.md)
- Phase 3 plan (pipeline, embeddings, lake): [23-phase3-parsing-indexing-plan.md](./23-phase3-parsing-indexing-plan.md)
- Phase 4 plan (search, registry): [24-phase4-search-registry-plan.md](./24-phase4-search-registry-plan.md)

---

## 15. Post-Review Mismatch Log

**Review date**: 2026-02-17
**Reviewer**: Automated cross-reference audit against actual crate implementations
**Resolution date**: 2026-02-17
**Methodology**: Every API call, constructor, method name, and type reference in this plan was checked against the actual source code in `zenith/crates/`. All 41 mismatches have been resolved by updating the plan's code snippets and descriptions to match actual APIs.

### Category A: API Signature Mismatches (BLOCKING — code won't compile)

**M1. `ZenLake::open()` does not exist — actual name is `open_local()`**

Plan (§A3 context.rs, line 564): `let lake = ZenLake::open(&lake_path)?;`
Actual (`zen-lake/src/lib.rs:48`): `pub fn open_local(path: &str) -> Result<Self, LakeError>`

Also: the plan passes `&PathBuf`, actual takes `&str`. Fix: `ZenLake::open_local(lake_path.to_str().unwrap())?`

**M2. `SourceFileStore::open()` takes `&str`, not `&Path`**

Plan (§A3 context.rs, line 565): `let source_store = SourceFileStore::open(&source_path)?;`
Actual (`zen-lake/src/source_files.rs:70`): `pub fn open(path: &str) -> Result<Self, LakeError>`

Fix: `SourceFileStore::open(source_path.to_str().unwrap())?`

**M3. `EmbeddingEngine::new()` takes no arguments — plan passes `Option<PathBuf>`**

Plan (§A3, line 566): `let embedder = EmbeddingEngine::new(Some(cache_dir))?;`
Actual (`zen-embeddings/src/lib.rs:53`): `pub fn new() -> Result<Self, EmbeddingError>` — cache dir is hardcoded to `~/.zenith/cache/fastembed/` internally via `dirs::home_dir()`.

Fix: `EmbeddingEngine::new()?`. The cache dir parameter does not exist. If custom cache dir is needed, the `EmbeddingEngine` API would need to be extended first.

**M4. `RegistryClient::search()` returns `Result<Vec<PackageInfo>>`, not `Vec<PackageInfo>`**

Plan (§B2, line 835): `ctx.registry.search(&query, &eco, limit).await?` — correct.
Plan (§B2, line 836): `ctx.registry.search_all(&query, limit).await?` — WRONG: `search_all()` returns `Vec<PackageInfo>` (not Result), uses `unwrap_or_log` internally.

Fix: `let results = ctx.registry.search_all(&query, limit).await;` (no `?`).

**M5. `whats_next()` takes no arguments — plan passes `limit`**

Plan (§F1, line 717): `ctx.service.whats_next(flags.limit.unwrap_or(10) as usize).await?`
Actual (`zen-db/src/repos/whats_next.rs:38`): `pub async fn whats_next(&self) -> Result<WhatsNextResponse, DatabaseError>` — no limit parameter.

Fix: Remove limit argument. If per-command limit is needed, apply it to the response fields after the call.

**M6. `query_audit()` takes `&AuditFilter`, not positional args**

Plan (§C6, lines 1002–1008): `ctx.service.query_audit(entity_type, entity_id, action, session, limit).await?`
Actual (`zen-db/src/repos/audit.rs:51-54`): `pub async fn query_audit(&self, filter: &AuditFilter) -> Result<Vec<AuditEntry>, DatabaseError>` where `AuditFilter` is a struct.

Fix: Build `AuditFilter { entity_type, entity_id, action, session_id, limit }` and pass `&filter`.

**M7. `search_audit()` takes `(query, limit)`, not `(query, limit) as usize`**

Plan (§C6, line 999): `ctx.service.search_audit(query, limit).await?`
Actual (`zen-db/src/repos/audit.rs:117`): `pub async fn search_audit(&self, query: &str, limit: u32)` — takes `u32`, not `usize`.

Minor but needs cast consistency.

**M8. `start_session()` returns `(Session, Option<Session>)`, not just `Session`**

Plan (§B1, line 795-797): Calls `service.start_session()` and expects orphan detection as separate call.
Actual (`zen-db/src/repos/session.rs:26`): `pub async fn start_session(&self) -> Result<(Session, Option<Session>), DatabaseError>` — orphan detection is built-in, returns the new session AND any abandoned orphan.

Fix: Destructure: `let (session, orphaned) = ctx.service.start_session().await?;`

**M9. `end_session()` takes `(session_id, summary)`, not `(session_id, abandon_flag)`**

Plan (§B1, line 799): `service.end_session(session_id, abandon)` — passes boolean abandon flag.
Actual (`zen-db/src/repos/session.rs:83-86`): `pub async fn end_session(&self, session_id: &str, summary: &str) -> Result<Session, DatabaseError>` — takes a summary string. There is no `abandon` parameter; abandoning is done internally by `start_session()`.

Fix: CLI `SessionCommands::End` needs a `--summary` argument, not `--abandon`. The abandon logic is handled automatically by `start_session()`.

**M10. `create_link()` takes 6 params including `session_id` and typed `EntityType`/`Relation` — plan passes 3 strings**

Plan (§C5, line 976): `ctx.service.create_link(source, target, relation).await?`
Actual (`zen-db/src/repos/link.rs:28-36`): `pub async fn create_link(&self, session_id: &str, source_type: EntityType, source_id: &str, target_type: EntityType, target_id: &str, relation: Relation) -> Result<EntityLink, DatabaseError>`

Fix: The CLI `znt link` command needs to accept source_type, source_id, target_type, target_id, and relation. It also needs the active session_id. The current CLI design (`Link { source: String, target: String, relation: String }`) is too simple — needs entity type fields or ID-prefix-based type inference.

**M11. `create_snapshot()` takes `(session_id, summary)`, not just `(session_id)`**

Plan (§F2, line 745): `ctx.service.create_session_snapshot(&session_id).await?`
Actual (`zen-db/src/repos/session.rs:206-209`): `pub async fn create_snapshot(&self, session_id: &str, summary: &str) -> Result<SessionSnapshot, DatabaseError>`

Fix: Method name is `create_snapshot` (not `create_session_snapshot`), and it takes a summary parameter.

**M12. No `wrap_up_session()` method exists — plan invents it**

Plan (§F2, line 748): `ctx.service.wrap_up_session(&session_id).await?`
Actual: No such method. The existing method is `end_session(session_id, summary)`.

Fix: Use `ctx.service.end_session(&session_id, &summary).await?`.

**M13. No `active_session_id()` method exists on `ZenService`**

Plan (§3.4, line 188; §F2, line 742): `ctx.service.active_session_id().await?`
Actual: No such method. To get the active session, use `list_sessions(Some(SessionStatus::Active), 1)` and take the first result.

Fix: Either add a convenience method to `ZenService`, or implement inline: `let sessions = ctx.service.list_sessions(Some(SessionStatus::Active), 1).await?; let session_id = sessions.first().map(|s| &s.id).ok_or_else(|| anyhow!("No active session"))?;`

**M14. `SchemaRegistry` has `list()` not `list_types()`**

Plan (§F3, line 777): `registry.list_types().join(", ")`
Actual (`zen-schema/src/registry.rs:157`): `pub fn list(&self) -> Vec<&'static str>`

Fix: `registry.list().join(", ")`

**M15. No `validate_trail_operation()` method on `SchemaRegistry`**

Plan (§E5, line 1504): `schema_registry.validate_trail_operation(&value)`
Actual: Only `validate(name, instance)` exists. Must use `registry.validate("trail_operation", &value)`.

Fix: `schema_registry.validate("trail_operation", &value)`

**M16. `get_linked_tasks()` does not exist — actual is `get_tasks_for_issue()`**

Plan (§C1, line 926): `ctx.service.get_linked_tasks(&id).await?`
Actual (`zen-db/src/repos/task.rs:324`): `pub async fn get_tasks_for_issue(&self, issue_id: &str) -> Result<Vec<Task>, DatabaseError>`

Fix: Use `ctx.service.get_tasks_for_issue(&id).await?`

### Category B: Non-Existent Methods on `ZenLake` (must be added or plan adjusted)

**M17. `ZenLake::list_indexed_packages()` does not exist**

Plan (§D2, line 1104; §D3, line 1125): `ctx.lake.list_indexed_packages()?`
Actual: Only `is_package_indexed(eco, pkg, ver)` exists. No list/enumerate method.

Fix: Add `list_indexed_packages()` to `ZenLake` in store.rs, or query `indexed_packages` table directly: `SELECT ecosystem, package, version FROM indexed_packages`.

**M18. `ZenLake::is_indexed()` does not exist — actual is `is_package_indexed()`**

Plan (§D4, line 1168): `ctx.lake.is_indexed(ecosystem, package, version)?`
Actual (`zen-lake/src/store.rs:208`): `pub fn is_package_indexed(&self, ecosystem: &str, package: &str, version: &str)`

**M19. `ZenLake::remove_package()` — exists as `delete_package()`**

Plan (§D3, line 1130): `ctx.lake.remove_package(&pkg)?`
Actual (`zen-lake/src/store.rs:253`): `pub fn delete_package(&self, ecosystem: &str, package: &str, version: &str)` — takes 3 params, not 1.

**M20. `ZenLake::clear()` does not exist**

Plan (§D3, line 1134): `ctx.lake.clear()?`
Actual: No such method. Must be added, or use `DELETE FROM api_symbols; DELETE FROM doc_chunks; DELETE FROM indexed_packages;`.

**M21. `SourceFileStore::remove_package()` does not exist — actual is `delete_package_sources()`**

Plan (§D3, line 1131): `ctx.source_store.remove_package(&pkg)?`
Actual (`zen-lake/src/source_files.rs:118`): `pub fn delete_package_sources(&self, ecosystem: &str, package: &str, version: &str)` — takes 3 params.

**M22. `SourceFileStore::clear()` does not exist**

Plan (§D3, line 1135): `ctx.source_store.clear()?`
Must be added to the crate.

**M23. `ZenLake::count_indexed_packages()` does not exist**

Plan (§D3, line 1148): `ctx.lake.count_indexed_packages()?`
Must be added.

### Category C: Dependency / Build Mismatches

**M24. `zen-cli/Cargo.toml` is missing `zen-schema` dependency**

The plan references `zen_schema::SchemaRegistry` in `commands/schema.rs` (§F3) and for the `znt schema` command, but `zen-schema.workspace = true` is NOT in zen-cli's current Cargo.toml. Add it.

**M25. `zen-cli/Cargo.toml` is missing `zen-hooks` dependency**

Plan correctly identifies this (§A1) but it's worth noting as blocking.

**M26. `zen-cli/Cargo.toml` is missing `dirs` dependency**

Plan correctly identifies this (§A1). Workspace has `dirs = "6.0"` but zen-cli doesn't list it.

**M27. `zen-cli/Cargo.toml` is missing `tempfile` in `[dependencies]` (not just `[dev-dependencies]`)**

Plan §D4 install.rs uses `tempfile::tempdir()` in production code for package cloning. Currently `tempfile` is only in `[dev-dependencies]`.

**M28. `zen-hooks/Cargo.toml` is missing `zen-schema` dependency**

Plan correctly identifies this (§E1). Needed for `validator.rs` pre-commit validation.

### Category D: Task Coverage Gaps

**M29. Plan omits `znt decision` commands from Phase 5**

07-implementation-plan.md §7 Phase 5 tasks reference spike 0.22 decision traces (§ spike result), and zen-db has `repos/` modules for decisions (mentioned in `spike_decision_traces.rs`). However, the 25-phase5-cli-shell-plan does not include any `znt decision` CLI command. The `Commands` enum has no `Decision` variant.

Status: The `decisions` table was added in spike 0.22 but there is no `decision.rs` in repos/. This may be intentionally deferred to Phase 6. Document explicitly.

**M30. Plan doesn't cover `znt research registry` as separate subcommand routing**

07-implementation-plan.md task 5.14 says: "Implement `znt research registry` wired to RegistryClient". The plan covers this inside `research.rs` (§B2, line 830) but the CLI struct shows `Research { action: ResearchCommands }` without showing that `ResearchCommands::Registry` is a variant. Verify it's included in the subcommand enum definition.

**M31. No `detect_orphaned_sessions()` public method — it's private**

Plan (§B1, line 795): "Detect and mark orphaned active sessions (via `service.detect_orphaned_sessions()`)"
Actual (`zen-db/src/repos/session.rs:260`): `async fn detect_orphan_sessions` is **private** (no `pub`). It's called internally by `start_session()`.

Fix: Remove separate orphan detection call from session handler — `start_session()` already handles it.

### Category E: Structural / Convention Issues

**M32. Plan startup sequence calls `dotenvy::dotenv()` before `Cli::parse()` (line 642), but `ZenConfig::load()` does NOT call dotenvy**

Plan (§A5, line 642): `dotenvy::dotenv().ok()` then later `ZenConfig::load()`.
Actual: `ZenConfig::load()` does NOT call dotenvy. `ZenConfig::load_with_dotenv()` does.

This is actually correct behavior (dotenvy called once in main before config load), but the plan should use `ZenConfig::load()` (not `load_with_dotenv()`) since dotenvy was already called. Current plan does this correctly.

**M33. `dispatch()` uses `&mut AppContext` but most handlers only need `&AppContext`**

Plan (§A6, line 742): `pub async fn dispatch(command: Commands, ctx: &mut AppContext, flags: &GlobalFlags)`
This is needed because `SearchEngine::new()` takes `&mut EmbeddingEngine`. Only search/install/onboard commands need `&mut`. The current approach works but means all handlers receive `&mut` even when they don't need it. Consider borrowing `embedder` separately for the commands that need it.

**M34. `TrailReplayer::rebuild()` is a static method on `TrailReplayer`, takes `&mut ZenService` — plan creates instance**

Plan (§E10, line 1636-1637): `let replayer = TrailReplayer::new(&service); let result = replayer.rebuild(&trail_path, strict).await?;`
Actual (`zen-db/src/trail/replayer.rs:16`): `pub async fn rebuild(service: &mut ZenService, trail_dir: &Path, strict: bool)` — it's an associated function (no `self`), takes `&mut ZenService`. There is no `TrailReplayer::new()`.

Fix: `TrailReplayer::rebuild(&mut service, &trail_path, strict).await?`

**M35. `zen-hooks` needs `zen-core` dependency for `gix::refs::transaction::PreviousValue`**

Plan §E9 session_tags.rs uses `gix::refs::transaction::PreviousValue::MustNotExist`. This should work since `gix` is in zen-hooks deps. However, the tag_reference API may differ by gix version — verified correct for gix 0.70.

### Category F: Incorrect Assumptions

**M36. `add_project_dependency()` does not exist on `ZenService`**

Plan (§D4, line 1187): `ctx.service.add_project_dependency(ecosystem, package, version).await?`
Actual: The method is `upsert_dependency(&self, dep: &ProjectDependency)` which takes a full `ProjectDependency` struct.

Fix: Build `ProjectDependency { ecosystem, name, version, source, indexed, indexed_at }` and call `upsert_dependency`.

**M37. `create_project_meta()` does not exist**

Plan (§D6, line 1301): `service.create_project_meta(&project_name, project_ecosystem.as_deref()).await?`
Actual: The method is `set_meta(key, value)` — stores key-value pairs, not a structured project record.

Fix: Use `service.set_meta("name", &project_name).await?; if let Some(eco) = &project_ecosystem { service.set_meta("ecosystem", eco).await?; }`

**M38. `GrepEngine` methods are associated functions, not methods on `&self`**

Plan (§D2, line 1101): `GrepEngine::grep_package(&ctx.source_store, &ctx.lake, pattern, &packages, &opts)?`
Actual: This is correct — they are associated functions (no `self`). But note that `GrepEngine` itself is never constructed; both methods are called directly on the type. The plan correctly uses this pattern.

**M39. `SearchEngine::search()` takes `&mut self`, plan creates `SearchEngine` inline**

Plan (§D1, lines 1059-1066) correctly constructs the engine and calls `.search()`. The plan uses `&mut ctx.embedder` which is correct given that `ctx` is `&mut AppContext`.

### Category G: Missing zen-cli Dependencies in Workspace

**M40. `zen-cli` Cargo.toml is missing `serde_json` usage annotation**

`serde_json` is already in deps. This is fine.

**M41. Plan `output.rs` uses `OutputFormat` but it's defined in `cli.rs`**

Plan §A4 output.rs references `OutputFormat` in the function signature but it's defined in `cli.rs`. Need to either import `crate::cli::OutputFormat` in output.rs, or move `OutputFormat` to output.rs and re-export from cli.rs.

### Category H: Missing `session_id` Parameters and Struct Field Mismatches (Post-Review Round 2)

**M42. `tag_finding()` and `untag_finding()` require `session_id` parameter**

Plan (§B3): `ctx.service.tag_finding(&id, &tag).await?` — missing `session_id`.
Actual (`zen-db/src/repos/finding.rs:256`): `tag_finding(&self, session_id: &str, finding_id: &str, tag: &str)`.

Fix: Added active session lookup + pass `session_id` as first arg to both `tag_finding` and `untag_finding`.

**M43. `update_hypothesis()` requires `session_id` and uses `reason` not `evidence`**

Plan (§B4): `ctx.service.update_hypothesis(&id, builder.build()).await?` — missing `session_id`. Also used `evidence` field but actual `HypothesisUpdate` has `reason: Option<Option<String>>`.
Actual (`zen-db/src/repos/hypothesis.rs:116`): `update_hypothesis(&self, session_id: &str, hyp_id: &str, update: HypothesisUpdate)`.

Fix: Added active session lookup, changed `evidence` → `reason`, pass `session_id`, and use return value directly (returns updated `Hypothesis`).

**M44. `update_task()` requires `session_id` parameter**

Plan (§C2): `ctx.service.update_task(&id, update).await?` — missing `session_id`.
Actual (`zen-db/src/repos/task.rs:116`): `update_task(&self, session_id: &str, task_id: &str, update: TaskUpdate)`.

Fix: Added active session lookup + pass `session_id`. `update_task` returns the updated `Task` directly.

**M45. `SearchFilters` missing `entity_types` and `min_score` fields; `limit` is `Option<u32>` not `u32`; `SearchEngine` needs `let mut`**

Plan (§D1): `SearchFilters { ..., limit: flags.limit.unwrap_or(20) }` and `let engine = SearchEngine::new(...)`.
Actual (`zen-search/src/lib.rs:58-66`): `limit: Option<u32>`, plus `entity_types: Vec<String>` and `min_score: Option<f64>` are required fields. `SearchEngine::search()` takes `&mut self`.

Fix: Added `entity_types: vec![]`, `limit: Some(...)`, `min_score: None`. Changed `let engine` to `let mut engine`.

**M46. `GrepOptions` field names differ from plan**

Plan (§D2): Uses `word_boundary`, `literal`, `context_lines`, `max_matches`.
Actual (`zen-search/src/grep.rs:85-113`): `word_regexp`, `fixed_strings`, `context_before`/`context_after` (separate), `max_count: Option<u32>`. Also has `smart_case`, `multiline`, `include_glob`, `exclude_glob`, `no_symbols` fields.

Fix: Changed to `GrepOptions::default()` with field-by-field assignment using correct names.

**M47. `IndexingPipeline::new()` takes owned `ZenLake` + `SourceFileStore`, not references**

Plan (§D4): `IndexingPipeline::new(/* Note: pipeline needs references, not owned */)`.
Actual (`zen-cli/src/pipeline.rs:45`): `pub fn new(lake: ZenLake, source_store: SourceFileStore) -> Self` — takes owned values.

Fix: Added pre-req task D0e to refactor `IndexingPipeline` to borrow instead of own, since `install.rs` and `onboard.rs` use `AppContext` which owns these resources.

### Summary Statistics

| Category | Count | Status |
|----------|-------|--------|
| A: API signature mismatches | 16 | ✅ ALL RESOLVED — plan code snippets updated |
| B: Non-existent ZenLake/SourceFileStore methods | 7 | ✅ RESOLVED — method names fixed; 4 new methods added as pre-req tasks (D0a-d) |
| C: Dependency/build mismatches | 5 | ✅ RESOLVED — Cargo.toml sections updated (A1, E1) |
| D: Task coverage gaps | 3 | ✅ RESOLVED — M29 documented as deferred, M30 confirmed in enum, M31 fixed |
| E: Structural/convention issues | 4 | ✅ RESOLVED — M32 confirmed correct, M33 noted, M34 fixed, M35 confirmed |
| F: Incorrect assumptions | 4 | ✅ RESOLVED — M36 upsert_dependency, M37 set_meta, M38/M39 confirmed correct |
| G: Missing import/module issues | 2 | ✅ RESOLVED — M40 confirmed fine, M41 import added |
| H: Missing session_id + struct field mismatches | 6 | ✅ ALL RESOLVED — session_id added, field names corrected, pre-req D0e added |
| **Total** | **47** | **ALL RESOLVED** |

### Resolution Actions Taken

1. **§A1 Cargo.toml**: Added `zen-schema`, `tempfile` deps to zen-cli; confirmed `zen-hooks`, `dirs`
2. **§A3 context.rs**: Fixed `ZenLake::open_local()`, `SourceFileStore::open()` `&str` params, `EmbeddingEngine::new()` no args
3. **§A4 output.rs**: Added `use crate::cli::OutputFormat` import
4. **§A2 cli.rs**: Updated `Link` variant to 5 fields (source_type, source_id, target_type, target_id, relation)
5. **§A6 dispatch**: Updated `Link` match arm for 5-field destructure
6. **§B1 session.rs**: Fixed `start_session()` tuple return, `end_session(id, summary)`, removed orphan detection call
7. **§B2 research.rs**: Fixed `search_all()` — removed `?` operator (returns `Vec` not `Result`)
8. **§3.4 finding.rs example**: Replaced `active_session_id()` with `list_sessions(Active, 1)` pattern
9. **§C1 issue.rs**: Fixed `get_linked_tasks()` → `get_tasks_for_issue()`
10. **§C5 link.rs**: Fixed `create_link()` to 6-param call with session_id + typed EntityType/Relation
11. **§C6 audit.rs**: Fixed `query_audit()` to use `&AuditFilter` struct, `search_audit` limit as `u32`
12. **§D3 cache.rs**: Fixed method names (`delete_package`, `delete_package_sources` with 3 params), added pre-req task notes
13. **§D4 install.rs**: Fixed `is_indexed` → `is_package_indexed`, `add_project_dependency` → `upsert_dependency(&ProjectDependency{...})`
14. **§D5 onboard.rs**: Fixed `is_indexed` → `is_package_indexed`
15. **§D6 init.rs**: Fixed `create_project_meta` → `set_meta(key, value)`, `start_session()` tuple destructure
16. **§E5 validator.rs**: Fixed `validate_trail_operation` → `validate("trail_operation", &value)`
17. **§E10 rebuild.rs**: Fixed `TrailReplayer::new()` → static `TrailReplayer::rebuild(&mut service, ...)`
18. **§F1 whats_next.rs**: Removed limit arg from `whats_next()`, fixed `query_audit` to `AuditFilter`
19. **§F2 wrap_up.rs**: Fixed `active_session_id` → `list_sessions`, `create_session_snapshot` → `create_snapshot(id, summary)`, `wrap_up_session` → `end_session`, `query_audit` → `AuditFilter`
20. **§F3 schema.rs**: Fixed `list_types()` → `list()`
21. **Task checklist**: Added D0a-d pre-requisite tasks for missing ZenLake/SourceFileStore methods
22. **§B3 finding.rs**: Added `session_id` to `tag_finding()` and `untag_finding()` calls (M42)
23. **§B4 hypothesis.rs**: Added `session_id` to `update_hypothesis()`, changed `evidence` → `reason` (M43)
24. **§C2 task.rs**: Added `session_id` to `update_task()` call (M44)
25. **§D1 search.rs**: Added `entity_types`, `min_score` to `SearchFilters`; fixed `limit` to `Option<u32>`; changed `let engine` to `let mut engine` (M45)
26. **§D2 grep.rs**: Fixed `GrepOptions` field names (`fixed_strings`, `word_regexp`, `context_before`/`context_after`, `max_count`) using `Default` + field assignment (M46)
27. **§D4 install.rs**: Documented `IndexingPipeline` ownership mismatch; added pre-req task D0e (M47)
28. **Task checklist**: Added D0e pre-requisite task for `IndexingPipeline` borrow refactor
