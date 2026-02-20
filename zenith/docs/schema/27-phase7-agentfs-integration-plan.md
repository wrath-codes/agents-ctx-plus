# Phase 7: AgentFS Integration — Implementation Plan

**Version**: 2026-02-19
**Status**: Implemented (PR1-PR2 complete)
**Depends on**: Phase 0 (spike 0.7 — `agentfs-sdk` validation — **DONE**), Phase 5 (zen-cli — working `znt` binary with session/install/wrap-up/audit commands — **DONE**)
**Produces**: Milestone 7 — Per-session AgentFS workspace databases with KV metadata and tool call tracking. Session-scoped audit trail via tool call reinterpretation. Workspace lifecycle (create → use → snapshot). Cleanup/retention is deferred — workspace DBs persist indefinitely.

> **⚠️ Scope**: Phase 7 is **workspace plumbing + CLI wiring**. All session/install/wrap-up/audit business logic already exists in upstream crates (session CRUD via `ZenService`, install pipeline via `IndexingPipeline`, audit queries via `AuditRepo`). This phase adds AgentFS-backed per-session workspace databases that wrap those existing operations with KV metadata and tool call tracking. CLI commands do **not** route filesystem I/O through AgentFS — actual file operations use the host filesystem (`TempDir` for clones, standard paths for everything else). AgentFS provides the audit/metadata layer, not filesystem isolation. No new database tables, no new entity types.

---

## Table of Contents

1. [Overview](#1-overview)
2. [Implementation Outcome](#2-implementation-outcome-as-of-2026-02-19)
3. [Key Decisions](#3-key-decisions)
4. [Architecture](#4-architecture)
5. [PR 1 — Stream A: Core Types + AgentFS Wrapper](#5-pr-1--stream-a-core-types--agentfs-wrapper)
6. [PR 2 — Stream B: CLI Wiring (Session, Install, Wrap-Up, Audit)](#6-pr-2--stream-b-cli-wiring-session-install-wrap-up-audit)
7. [Execution Order](#7-execution-order)
8. [Gotchas & Warnings](#8-gotchas--warnings)
9. [Milestone 7 Validation](#9-milestone-7-validation)
10. [Validation Traceability Matrix](#10-validation-traceability-matrix)
11. [Plan Review — Mismatch Log](#11-plan-review--mismatch-log)

---

## 1. Overview

**Goal**: Wire `agentfs-sdk` into the Zenith CLI to provide per-session workspace databases with KV metadata, tool call audit trails, and workspace snapshots at session wrap-up. CLI commands do not route filesystem I/O through AgentFS — AgentFS serves as a session-scoped metadata and audit layer, not a filesystem isolation boundary. Package indexing uses `tempfile::TempDir` for clone isolation (the git clone lifecycle is too short-lived for persistent AgentFS workspaces), but records install events into the session's AgentFS workspace for audit continuity. The audit trail captures tool call events (e.g., `install_index`) with optional `path` fields — `WorkspaceAuditEntry.path` is `None` for events that don't involve specific files.

**Crates touched**:
- `zen-core` — **light**: add `workspace.rs` module with workspace data types (`WorkspaceBackend`, `WorkspaceInfo`, `WorkspaceSnapshot`, `WorkspaceAuditEntry`, `WorkspaceChannelStatus`)
- `zen-cli` — **medium**: add `workspace/` module with `agentfs.rs` wrapper, wire into `session/start.rs`, `wrap_up/handle.rs`, `audit/files.rs`, `install.rs`, add `audit/merge.rs` for timeline merging

**Dependency changes needed**:
- `zen-cli`: `agentfs-sdk.workspace = true` already in `[dependencies]` (added in Phase 5 prep)
- `zen-cli`: `chrono.workspace = true` already in `[dependencies]` (for timestamp handling in workspace types)
- No new workspace-level dependency additions needed

**Estimated deliverables**: ~7 modified/new production files, ~350 LOC production code, ~90 LOC tests

**PR strategy**: 2 PRs by stream. Stream A provides the type foundation and AgentFS wrapper. Stream B wires it into CLI commands.

| PR | Stream | Contents | Depends On |
|----|--------|----------|------------|
| PR 1 | A: Core Types + Wrapper | `zen-core/src/workspace.rs`, `zen-cli/src/workspace/mod.rs`, `zen-cli/src/workspace/agentfs.rs` | None (clean start) |
| PR 2 | B: CLI Wiring | `commands/session/start.rs`, `commands/wrap_up/handle.rs`, `commands/audit/files.rs`, `commands/audit/merge.rs`, `commands/install.rs` | Stream A |

---

## 2. Implementation Outcome (as of 2026-02-19)

### zen-core — Workspace Types

| Aspect | Status | Detail |
|--------|--------|--------|
| **`workspace.rs`** | **DONE** | 1 enum + 4 structs: `WorkspaceBackend` (enum), `WorkspaceInfo`, `WorkspaceSnapshot`, `WorkspaceAuditEntry`, `WorkspaceChannelStatus`. All derive `Serialize`/`Deserialize`/`JsonSchema`. |

### zen-cli — Workspace Module + CLI Wiring

| Aspect | Status | Detail |
|--------|--------|--------|
| **`workspace/mod.rs`** | **DONE** | Re-exports `agentfs` module. |
| **`workspace/agentfs.rs`** | **DONE** | 5 public functions: `create_session_workspace()`, `record_install_event()`, `session_workspace_snapshot()`, `session_file_audit()`, `active_session_file_audit()`. 7 private helpers: `open_session_workspace()`, `open_active_workspace()`, `persistent_workspace_db_path()`, `validate_session_id()`, `workspace_id()`, `now_epoch_secs()`, `parse_timestamp_from_tool_call()`. |
| **Session start wiring** | **DONE** | `commands/session/start.rs` creates AgentFS workspace after session creation, abandons session on workspace failure. |
| **Install wiring** | **DONE** | `commands/install.rs` records install event into active session's workspace after successful indexing. |
| **Wrap-up wiring** | **DONE** | `commands/wrap_up/handle.rs` captures workspace snapshot (tool call aggregation) with graceful degradation on failure. |
| **Audit --files wiring** | **DONE** | `commands/audit/files.rs` merges entity audit + file audit from AgentFS with dual-channel error handling. |
| **Audit merge** | **DONE** | `commands/audit/merge.rs` provides chronological timeline merge of entity + file audit entries. 1 unit test. |
| **Spike** | Consumed | Spike 0.7 patterns promoted into production `workspace/agentfs.rs`. |

### Stream Completion Summary

| PR | Stream | Status | Delivered |
|----|--------|--------|-----------|
| PR 1 | A: Core Types + Wrapper | **DONE** | Workspace types in zen-core, AgentFS wrapper in zen-cli |
| PR 2 | B: CLI Wiring | **DONE** | Session start, install, wrap-up, audit --files all wired |

---

## 3. Key Decisions

All decisions derive from spike 0.7 findings ([spike_agentfs.rs](../../crates/zen-cli/src/spike_agentfs.rs)) and Phase 5 CLI conventions.

### 3.1 AgentFS for Session Workspaces, TempDir for Clone Isolation

**Decision**: Each `znt session start` creates a persistent AgentFS workspace database (keyed by session ID, stored at `.zenith/workspaces/{session-id}.db`). The workspace DB provides KV metadata storage and tool call tracking — it does **not** serve as a filesystem isolation boundary (CLI commands use the host filesystem for all file I/O). Package cloning during `znt install` uses `tempfile::TempDir` (standard library temp directories) for the short-lived clone→parse→cleanup lifecycle, but records install events into the session's AgentFS workspace for audit continuity.

**Rationale**: AgentFS provides persistent KV store and tool call tracking that survive across session operations, enabling session-scoped audit trails and metadata. But CLI file operations (git clone, tree-sitter parse, DuckDB writes) stay on the host filesystem — routing them through AgentFS would add overhead with no isolation benefit for a single-user CLI. Recording install events as AgentFS tool calls bridges the gap: the audit trail captures what was indexed without requiring the clone files to persist.

**Validated in**: Spike 0.7 (`spike_agentfs_indexing_workspace_pattern` test demonstrates the full lifecycle).

### 3.2 Graceful Degradation Pattern: Dual-Channel Error Handling

**Decision**: Workspace operations never block the primary CLI command flow. If AgentFS fails (workspace creation, snapshot capture, install event recording, file audit query), the primary operation succeeds and a warning is logged. The `WorkspaceChannelStatus` struct reports `"ok"` or `"error"` per channel in structured output.

**Rationale**: AgentFS is an enhancement layer, not a critical path. Session start is the one exception — workspace creation failure causes session abandonment (the session is useless without workspace isolation). All other workspace operations degrade gracefully.

**Validated in**: Phase 5 wiring — `commands/wrap_up/handle.rs` wraps `session_workspace_snapshot()` in `match` with `WorkspaceChannelStatus`, `commands/install.rs` wraps `record_install_event()` in `if let Err(error)` with `tracing::warn!`.

### 3.3 Session Start: Workspace Creation Is Atomic with Session Creation

**Decision**: If `create_session_workspace()` fails after `start_session()` succeeds, the session is immediately abandoned via `ctx.service.abandon_session(&session.id)`. This ensures no orphaned sessions exist without workspaces.

**Rationale**: A session without a workspace would produce inconsistent behavior — `znt audit --files` would fail, `znt wrap-up` snapshot would fail. It's safer to fail fast and let the user retry.

**Validated in**: `commands/session/start.rs` implementation — the `Err` branch calls `abandon_session()` before returning the error.

### 3.4 No Workspace Trait — Direct SDK Integration

**Decision**: No `Workspace` trait abstraction. The `workspace::agentfs` module provides standalone functions that operate on `AgentFS` instances directly. The original plan (task 7.1) described a `Workspace` trait, but the implementation uses concrete types instead.

**Rationale**: Since spike 0.7 passed (and task 0.10 was cancelled), there is only one backend — AgentFS. A trait would add indirection without value. The `WorkspaceBackend` enum exists in zen-core for forward compatibility (could add `TempDir` variant later), but the runtime code is always `AgentFS::open()`.

### 3.5 File Audit via Tool Call Reinterpretation

**Decision**: `session_file_audit()` queries AgentFS `tools.recent()` and reinterprets each tool call as a `WorkspaceAuditEntry` by extracting fields from the serialized `ToolCall` JSON. IDs are synthesized as `wsa-{original_id}` or `wsa-{timestamp_micros}-{index}` (note: source timestamps are epoch seconds with nanos=0, so `timestamp_micros()` produces `seconds * 1_000_000` — the microsecond granularity comes from the format, not the input precision).

**Rationale**: AgentFS tool calls are the only session-scoped activity records. Reinterpreting them as audit entries provides a unified audit interface (`znt audit --files`) without requiring a separate audit table in AgentFS. Note that `WorkspaceAuditEntry.path` is only populated when the tool call's `parameters` contain a `path` field — the current `install_index` event has `{ecosystem, package, version}` parameters with no `path`, so file-level granularity depends on future tool call types recording path information.

**Validated in**: Spike 0.7 tool tracking tests + `commands/audit/files.rs` dual-channel implementation.

### 3.6 Workspace DB Path Convention: `.zenith/workspaces/{session-id}.db`

**Decision**: Workspace databases live at `.zenith/workspaces/{session-id}.db`. Session ID validation rejects path traversal (`..`, `/`, `\\`) and non-alphanumeric characters (only `[A-Za-z0-9_-]` allowed).

**Rationale**: One DB file per session enables independent lifecycle management. Session ID validation prevents path injection attacks via crafted session IDs.

### 3.7 Active Workspace Discovery: Most-Recently-Modified DB File

**Decision**: `active_session_file_audit()` (called when `--session` is not provided) discovers the active workspace by scanning `.zenith/workspaces/` for the most recently modified `.db` file.

**Rationale**: Avoids requiring the caller to know the active session ID. The most-recently-modified heuristic works because `create_session_workspace()` writes KV pairs at creation time, and `record_install_event()` writes during install — both update the modification time.

### 3.8 Merged Timeline Output for `znt audit --files --merge-timeline`

**Decision**: `audit/merge.rs` provides `merge_timeline()` that interleaves entity audit entries and file audit entries into a single chronologically sorted `Vec<TimelineEntry>`. Each entry carries a `source` field (`"entity"` or `"file"`) for disambiguation.

**Rationale**: LLMs and humans both benefit from seeing entity mutations and file operations in a single timeline. The `--merge-timeline` flag controls whether the output is merged or dual-channel (separate `entity_audit` and `file_audit` arrays).

---

## 4. Architecture

### Module Structure

```
zen-core/src/
└── workspace.rs                 # NEW — WorkspaceBackend, WorkspaceInfo, WorkspaceSnapshot,
                                 #        WorkspaceAuditEntry, WorkspaceChannelStatus

zen-cli/src/
├── workspace/
│   ├── mod.rs                   # NEW — pub mod agentfs
│   └── agentfs.rs               # NEW — AgentFS SDK wrappers (5 pub + 7 private functions)
├── commands/
│   ├── session/
│   │   ├── start.rs             # MODIFIED — workspace creation on session start
│   │   └── types.rs             # MODIFIED — SessionStartResponse includes WorkspaceInfo
│   ├── wrap_up/
│   │   └── handle.rs            # MODIFIED — workspace snapshot at wrap-up
│   ├── audit.rs                 # MODIFIED — dispatch: branches on --files via #[path] sub-modules
│   ├── audit/
│   │   ├── files.rs             # NEW — dual-channel entity + file audit query
│   │   ├── merge.rs             # NEW — chronological timeline merge
│   │   ├── query.rs             # EXISTING (restructured) — entity-only audit query + fetch()
│   │   └── search.rs            # EXISTING (restructured) — entity-only audit search + fetch()
│   └── install.rs               # MODIFIED — record install event in workspace
└── main.rs                      # MODIFIED — add `mod workspace`
```

### Upstream Dependencies — All Ready

| Dependency | Method | Crate | Status | Usage |
|------------|--------|-------|--------|-------|
| `start_session()` | `ZenService::start_session()` | zen-db | **DONE** | Session creation (triggers workspace) |
| `abandon_session()` | `ZenService::abandon_session()` | zen-db | **DONE** | Rollback on workspace failure |
| `end_session()` | `ZenService::end_session()` | zen-db | **DONE** | Wrap-up (triggers snapshot) |
| `list_sessions()` | `ZenService::list_sessions(status, limit)` | zen-db | **DONE** | Find active session for install events |
| `query_audit()` | `ZenService::query_audit(filter)` | zen-db | **DONE** | Entity audit channel in `--files` |
| `AgentFS::open()` | `agentfs_sdk::AgentFS::open(opts)` | agentfs-sdk | **DONE** | Open/create workspace DBs |
| `agent.kv.set/get` | KV CRUD | agentfs-sdk | **DONE** | Session metadata storage |
| `agent.fs.mkdir` | Filesystem ops | agentfs-sdk | **DONE** | Workspace root creation |
| `agent.tools.record/recent` | Tool tracking | agentfs-sdk | **DONE** | Install event recording, file audit |

### Data Flow

```
znt session start
  → start_session() → Session { id: "ses-xxx" }
  → create_session_workspace(project_root, "ses-xxx")
      → AgentFS::open(.zenith/workspaces/ses-xxx.db)
      → agent.kv.set("session_id", "ses-xxx")
      → agent.kv.set("workspace_root", "/workspace")
      → agent.fs.mkdir("/workspace", 0, 0)
  → returns SessionStartResponse { session, orphaned, workspace: WorkspaceInfo }
  → ON FAILURE: abandon_session("ses-xxx")

znt install tokio
  → [... indexing pipeline via TempDir ...]
  → on success: record_install_event(project_root, session_id, "rust", "tokio", "1.49.0", true, None)
      → open_session_workspace(project_root, session_id)
      → agent.tools.record("install_index", started, ended, params, result, None)
  → ON FAILURE: tracing::warn! (non-blocking)

znt wrap-up
  → end_session() → Session
  → session_workspace_snapshot(project_root, "ses-xxx")
      → open_session_workspace(project_root, "ses-xxx")
      → agent.tools.recent(Some(500))
      → count success/error → WorkspaceSnapshot
  → returns WrapUpResponse { ..., workspace_snapshot_status, workspace_snapshot }
  → ON FAILURE: WorkspaceChannelStatus { status: "error", error: Some(...) }

znt audit --files [--session ses-xxx] [--merge-timeline]
  → audit.rs handle(): branches on args.files
  → entity channel: query::fetch() (wraps query_audit) or search::fetch() (wraps search_audit)
  → file channel:
      → if --session: session_file_audit(project_root, session_id, limit, search)
      → else: active_session_file_audit(project_root, limit, search)
          → open_active_workspace() → most-recently-modified .db
          → kv.get("session_id") → extract session ID
          → delegates to session_file_audit(project_root, session_id, limit, search)
              → open_session_workspace() (re-opens workspace by session ID path)
              → agent.tools.recent(Some(i64::from(limit))) → reinterpret as WorkspaceAuditEntry[]
  → if --merge-timeline: merge_timeline(entity, file) → sorted TimelineEntry[]
  → else: AuditFilesResponse { entity_audit, file_audit }
  → ON FAILURE (either channel): channel status reports error, other channel still returns data
```

---

## 5. PR 1 — Stream A: Core Types + AgentFS Wrapper

**Tasks**: 7.1, 7.2
**Estimated LOC**: ~340 production

### A1. `zen-core/src/workspace.rs` — Workspace Types (task 7.1)

```rust
use chrono::{DateTime, Utc};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum WorkspaceBackend {
    Agentfs,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct WorkspaceInfo {
    pub backend: WorkspaceBackend,
    pub workspace_id: String,
    pub root: String,
    pub persistent: bool,
    pub created: bool,
    pub status: String,
    pub note: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct WorkspaceSnapshot {
    pub status: String,
    pub workspace_id: String,
    pub files_total: u64,
    pub bytes_total: u64,
    pub tool_calls_total: u64,
    pub tool_calls_success: u64,
    pub tool_calls_failed: u64,
    pub captured_at: DateTime<Utc>,
    pub note: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct WorkspaceAuditEntry {
    pub id: String,
    pub session_id: String,
    pub workspace_id: String,
    pub source: String,
    pub event: String,
    pub path: Option<String>,
    pub tool: String,
    pub status: String,
    pub params: Option<serde_json::Value>,
    pub result: Option<serde_json::Value>,
    pub error: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct WorkspaceChannelStatus {
    pub status: String,
    pub error: Option<String>,
}
```

**Design notes**:
- All types derive `JsonSchema` for consistency with zen-core entity conventions (validated in spike 0.15).
- `WorkspaceBackend` is a single-variant enum — extensible for future backends without breaking serde.
- `WorkspaceSnapshot.files_total`/`bytes_total` are `u64` but currently always `0` — AgentFS doesn't expose directory traversal/stat aggregation. Tracked as future enhancement.
- `WorkspaceChannelStatus` enables structured error reporting in dual-channel responses (audit --files, wrap-up).

### A2. `zen-cli/src/workspace/agentfs.rs` — AgentFS SDK Wrapper (task 7.2)

**5 public functions:**

| Function | Signature | Usage |
|----------|-----------|-------|
| `create_session_workspace` | `(project_root: &Path, session_id: &str) -> Result<WorkspaceInfo>` | Session start |
| `record_install_event` | `(project_root: &Path, session_id: &str, ecosystem: &str, package: &str, version: &str, success: bool, error: Option<&str>) -> Result<()>` | Install |
| `session_workspace_snapshot` | `(project_root: &Path, session_id: &str) -> Result<WorkspaceSnapshot>` | Wrap-up |
| `session_file_audit` | `(project_root: &Path, session_id: &str, limit: u32, search: Option<&str>) -> Result<Vec<WorkspaceAuditEntry>>` | Audit --files with --session |
| `active_session_file_audit` | `(project_root: &Path, limit: u32, search: Option<&str>) -> Result<Vec<WorkspaceAuditEntry>>` | Audit --files (active session) |

**7 private helpers:**

| Helper | Purpose |
|--------|---------|
| `open_session_workspace()` | Open AgentFS for known session ID |
| `open_active_workspace()` | Discover and open most-recently-modified workspace DB |
| `persistent_workspace_db_path()` | Resolve `.zenith/workspaces/{session-id}.db` with validation |
| `validate_session_id()` | Reject path traversal and non-alphanumeric characters |
| `workspace_id()` | Format workspace ID as `ws-{session_id}` |
| `now_epoch_secs()` | Current UTC timestamp as epoch seconds (via `Utc::now().timestamp()`) |
| `parse_timestamp_from_tool_call()` | Extract `DateTime<Utc>` from tool call JSON (`started_at` or `ended_at` field) |

**Key implementation patterns:**

1. **Workspace creation**: `AgentFS::open(AgentFSOptions::with_path(path))` → `kv.set("session_id", ...)` → `fs.mkdir("/workspace", 0, 0)`.
2. **Tool call recording**: `agent.tools.record(name, started_at, ended_at, params, result, error)` with positional args (not the simplified API from Turso docs).
3. **Snapshot aggregation**: `agent.tools.recent(Some(500))` → serialize each call to `serde_json::Value` → match `status` field as `"success"` or `"error"` → count.
4. **File audit reinterpretation**: Each `ToolCall` → `serde_json::to_value()` → extract `name`/`parameters`/`status`/`error`/`started_at` → construct `WorkspaceAuditEntry`.
5. **Session ID validation**: `validate_session_id()` rejects `..`, `/`, `\\`, and characters outside `[A-Za-z0-9_-]`.
6. **Active workspace discovery**: Read `.zenith/workspaces/`, filter by `.db` extension, sort by modification time, take last.

### A3. `zen-cli/src/workspace/mod.rs` — Module Re-export

```rust
pub mod agentfs;
```

### A4. `zen-cli/src/main.rs` — Module Declaration

Add:

```rust
mod workspace;
```

---

## 6. PR 2 — Stream B: CLI Wiring (Session, Install, Wrap-Up, Audit)

**Tasks**: 7.3, 7.4, 7.5, 7.6
**Estimated LOC**: ~250 production, ~90 tests

### B1. `commands/session/start.rs` — Workspace Creation on Session Start (task 7.3)

```rust
pub async fn run(ctx: &AppContext, flags: &GlobalFlags) -> anyhow::Result<()> {
    let (session, orphaned) = ctx.service.start_session().await?;
    let workspace =
        match crate::workspace::agentfs::create_session_workspace(&ctx.project_root, &session.id)
            .await
        {
            Ok(workspace) => workspace,
            Err(error) => {
                if let Err(abandon_error) = ctx.service.abandon_session(&session.id).await {
                    tracing::error!(
                        session = %session.id,
                        %abandon_error,
                        "session start: failed to abandon session after workspace init failure"
                    );
                }
                return Err(error);
            }
        };
    output(
        &SessionStartResponse { session, orphaned, workspace },
        flags.format,
    )
}
```

**Key behavior**: Workspace creation failure is the **only** case where workspace error is fatal. The session is abandoned to prevent orphaned sessions without workspaces. The `tracing::error!` in the abandon fallback ensures observability if both operations fail.

### B2. `commands/session/types.rs` — Response Type (task 7.3)

```rust
#[derive(Debug, Serialize)]
pub struct SessionStartResponse {
    pub session: Session,
    pub orphaned: Option<Session>,
    pub workspace: WorkspaceInfo,
}
```

### B3. `commands/install.rs` — Install Event Recording (task 7.4)

After successful indexing and dependency upsert, record the install event into the active session's workspace:

```rust
if let Some(session) = ctx
    .service
    .list_sessions(Some(SessionStatus::Active), 1)
    .await?
    .first()
    .cloned()
    && let Err(error) = crate::workspace::agentfs::record_install_event(
        &ctx.project_root,
        &session.id,
        &ecosystem,
        &args.package,
        &version,
        true,
        None,
    )
    .await
{
    tracing::warn!(
        session = %session.id,
        package = %args.package,
        %error,
        "install: failed to write workspace audit event"
    );
}
```

**Key behavior**: Non-blocking — install succeeds even if workspace audit recording fails. Uses `list_sessions(Active, 1)` to find the active session without requiring the caller to pass it. If no active session exists, the block is skipped entirely.

### B4. `commands/wrap_up/handle.rs` — Workspace Snapshot (task 7.5)

```rust
let (workspace_snapshot_status, workspace_snapshot) =
    match crate::workspace::agentfs::session_workspace_snapshot(&ctx.project_root, &session.id)
        .await
    {
        Ok(snapshot) => (
            WorkspaceChannelStatus { status: "ok".to_string(), error: None },
            Some(snapshot),
        ),
        Err(error) => (
            WorkspaceChannelStatus { status: "error".to_string(), error: Some(error.to_string()) },
            None,
        ),
    };
```

**Key behavior**: Graceful degradation — wrap-up completes even if workspace snapshot fails. Both `workspace_snapshot_status` and `workspace_snapshot` are included in the `WrapUpResponse` for structured observability.

### B5. `commands/audit/files.rs` — Dual-Channel File Audit (task 7.6)

Two response types:
- `AuditFilesResponse`: Separate `entity_audit: Vec<AuditEntry>` and `file_audit: Vec<WorkspaceAuditEntry>` arrays
- `AuditFilesMergedResponse`: Single `timeline: Vec<TimelineEntry>` sorted chronologically

Channel resolution logic:
1. Entity channel: `search::fetch()` (wraps `search_audit()`) if `--search` provided, else `query::fetch()` (wraps `query_audit()`) with filters
2. File channel: `session_file_audit()` if `--session` provided, else `active_session_file_audit()` (discovers most-recently-modified .db, reads session ID from KV, delegates to `session_file_audit()`)
3. If both channels fail: `anyhow::bail!` with both error messages
4. If one channel fails: report error in channel status, return data from successful channel
5. If `--merge-timeline`: merge and sort both channels into `Vec<TimelineEntry>`

### B6. `commands/audit/merge.rs` — Timeline Merge

```rust
pub fn merge_timeline(
    entity_audit: &[AuditEntry],
    file_audit: &[WorkspaceAuditEntry],
) -> anyhow::Result<Vec<TimelineEntry>> {
    let mut merged = Vec::with_capacity(entity_audit.len() + file_audit.len());
    for entry in entity_audit {
        merged.push(TimelineEntry {
            source: "entity".to_string(),
            created_at: entry.created_at.to_rfc3339(),
            entry: serde_json::to_value(entry)?,
        });
    }
    for entry in file_audit {
        merged.push(TimelineEntry {
            source: "file".to_string(),
            created_at: entry.created_at.to_rfc3339(),
            entry: serde_json::to_value(entry)?,
        });
    }
    merged.sort_by(|a, b| a.created_at.cmp(&b.created_at).then_with(|| a.source.cmp(&b.source)));
    Ok(merged)
}
```

**Sort stability**: Uses RFC 3339 string comparison (lexicographically correct for ISO 8601 timestamps). Ties broken by `source` (entity sorts before file alphabetically).

---

## 7. Execution Order

```
PR 1 (Stream A):
  1. Add zen-core/src/workspace.rs (types)
  2. Add zen-cli/src/workspace/mod.rs + agentfs.rs (wrapper)
  3. Add `mod workspace` to main.rs
  4. Verify: cargo build -p zen-core -p zen-cli

PR 2 (Stream B):
  5. Wire session/start.rs (workspace creation + abandon on failure)
  6. Wire session/types.rs (add WorkspaceInfo to response)
  7. Wire install.rs (record_install_event after indexing)
  8. Wire wrap_up/handle.rs (workspace snapshot with degradation)
  9. Add audit/files.rs (dual-channel query)
  10. Add audit/merge.rs (timeline merge)
  11. Verify: cargo test -p zen-cli
```

---

## 8. Gotchas & Warnings

### 8.1 AgentFS SDK API Mismatch with Turso Documentation

**Problem**: The Turso docs at `docs.turso.tech/agentfs/sdk/rust` describe a high-level API (`write_file()`, `rm()`, `exists()`) that does **not exist** in `agentfs-sdk` 0.6.0. The actual API is POSIX-level: `create_file(path, mode, uid, gid)` → `pwrite(path, offset, data)`, `stat()` instead of `exists()`, `remove()` instead of `rm()`.

**Impact**: Anyone implementing against the Turso docs will get compilation errors. All code must use the actual API validated in spike 0.7.

**Resolution**: `workspace/agentfs.rs` uses only the validated POSIX-level API. No convenience wrappers were added (the touch points are few enough that raw API calls are clear).

### 8.2 Crate Name Confusion: `agentfs` vs `agentfs-sdk`

**Problem**: Turso docs say `agentfs = "0.1"`, but the correct crate on crates.io is `agentfs-sdk` (v0.6.0, by penberg). The `agentfs` crate (v0.2.0) is by a different author and is a completely separate project.

**Impact**: Using the wrong crate name in Cargo.toml will compile but link to the wrong library.

**Resolution**: Workspace Cargo.toml specifies `agentfs-sdk = "0.6"`. This is validated by spike 0.7.

### 8.3 `tools.recent()` Returns ToolCall Structs, Not JSON

**Problem**: `agent.tools.recent(limit)` returns `Vec<ToolCall>` where `ToolCall` fields are accessed via the struct, but the status field uses a `ToolCallStatus` enum (not a string). To construct `WorkspaceAuditEntry`, the code serializes to `serde_json::Value` first and then extracts string fields.

**Impact**: Direct field access for status comparison doesn't work with string matching. Must serialize first.

**Resolution**: `session_workspace_snapshot()` and `session_file_audit()` both use `serde_json::to_value(&call)?` as the intermediate representation.

### 8.4 Workspace Snapshot `files_total` and `bytes_total` Always Zero

**Problem**: AgentFS doesn't expose a directory listing or recursive stat API. The `session_workspace_snapshot()` function sets `files_total: 0` and `bytes_total: 0` with a note explaining the limitation.

**Impact**: Workspace snapshot tool call metrics are accurate, but file metrics are unavailable.

**Resolution**: The `note` field contains `"file stats are pending deeper AgentFS traversal support"`. This is acceptable for MVP — tool call metrics are the primary signal.

### 8.5 `open_active_workspace()` Sorting Heuristic

**Problem**: The function sorts workspace DB files by `(modification_time, path)` and takes the last one. If two workspaces are modified at the same second, the one with the lexicographically later path wins. This is generally safe because session IDs are unique, but theoretically could pick the wrong workspace in a race condition.

**Additional concern**: Depending on SQLite/WAL behavior and OS platform, simply opening or querying a workspace DB (e.g., via `znt audit --files`) can update its `mtime`, causing "active" to shift from "last used for real work" to "last audited." This can produce surprising results on subsequent `--files` calls without `--session`.

**Impact**: Very low probability in practice (requires two session starts within the same second). The mtime-bias concern is more likely but only affects the `--files` (no `--session`) path.

**Resolution**: Acceptable for MVP. The `--session <id>` flag on `znt audit --files` provides deterministic workspace selection when needed.

### 8.6 `agentfs-sdk` Depends on `turso ^0.4.4` (Limbo-Based)

**Problem**: `agentfs-sdk` internally uses the `turso` crate (Limbo-based SQLite), which coexists with zenith's `libsql` dependency. These are separate database engines: `libsql` for zenith's own state (Turso Cloud sync), `turso` (via `agentfs-sdk`) for AgentFS's internal storage.

**Impact**: Two SQLite engines in the dependency tree, increasing binary size. No runtime conflict — they manage separate databases.

**Resolution**: Accepted. The binary size increase (~2-3 MB) is negligible compared to `duckdb-bundled` (~30 MB).

### 8.7 AgentFS Requires `tokio::test(flavor = "multi_thread")`

**Problem**: `agentfs-sdk` uses `turso` internally which may require the multi-threaded tokio runtime for background tasks. Tests using `#[tokio::test]` (current-thread) may hang or fail.

**Impact**: All tests touching AgentFS must use `#[tokio::test(flavor = "multi_thread")]`.

**Resolution**: Spike 0.7 tests all use `multi_thread` flavor. Production code runs in `#[tokio::main]` which defaults to multi-thread. No action needed for production; test authors must remember the flavor annotation.

### 8.8 `WorkspaceAuditEntry.path` Is Usually `None`

**Problem**: The `session_file_audit()` function populates `WorkspaceAuditEntry.path` only when the tool call's `parameters` JSON contains a `path` field. The only tool call currently recorded in production is `install_index`, whose parameters are `{ecosystem, package, version}` — no `path` field.

**Impact**: `znt audit --files` returns entries with `path: null` for all install events. The "file audit" channel provides tool-call-level audit, not file-level audit, until future tool call types include `path` in their parameters.

**Resolution**: Acceptable for MVP. The audit channel name (`--files`) is aspirational — it will gain file-level granularity as more tool call types are recorded with path information. The `path` field is `Option<String>` by design to accommodate both cases.

### 8.9 Workspace DB Accumulation — No Retention Policy

**Problem**: `.zenith/workspaces/` grows indefinitely. Each `znt session start` creates a new `{session-id}.db` file. Neither `znt wrap-up` nor `znt session abandon` deletes or archives workspace DBs.

**Impact**: Over many sessions, the `workspaces/` directory accumulates DB files. Each file is small (typically <1 MB for tool call data), but unbounded growth is a hygiene concern.

**Resolution**: Acceptable for MVP. Future options: (1) add `znt workspace clean` command with retention policy, (2) delete DB on wrap-up after snapshot, (3) add `--keep-workspace` flag to opt out of cleanup. Manual `rm .zenith/workspaces/*.db` is safe for immediate relief.

### 8.10 Session ID Validation: No Length or Emptiness Check

**Problem**: `validate_session_id()` rejects path traversal and non-alphanumeric characters, but does not check for empty strings or excessively long IDs.

**Impact**: An empty session ID would produce a workspace path `.zenith/workspaces/.db`. An extremely long session ID could exceed OS path length limits (typically 255 bytes for filename, 4096 for full path).

**Resolution**: Low risk — session IDs are generated internally by `ZenService::start_session()` with a consistent `ses-{uuid}` format. External callers (e.g., `--session` flag on `znt audit --files`) could pass pathological values, but the resulting `AgentFS::open()` call would fail with a clear error. Add length/emptiness guards in a future hardening pass.

### 8.11 Tool Call Status Parsing via JSON Serialization Is Brittle

**Problem**: `session_workspace_snapshot()` and `session_file_audit()` determine tool call status by serializing `ToolCall` to `serde_json::Value` and string-matching the `status` field (`"success"`, `"error"`). The `agentfs-sdk` crate exposes `ToolCallStatus` as a Rust enum (e.g., `ToolCallStatus::Success`), which could be matched directly.

**Impact**: If `agentfs-sdk` changes the serde representation of `ToolCallStatus` (e.g., from `"success"` to `"Success"` or `{"status": "success"}`), the string matching silently breaks — unrecognized statuses fall through to the `_ => {}` branch and are uncounted.

**Resolution**: Accepted for MVP — the spike validates the current serialization format. The serialize-then-match pattern was chosen because multiple fields need extraction from the same `Value` (name, parameters, status, error, timestamps). Switching to direct enum matching would require accessing `call.status` separately from the JSON extraction of other fields. If `agentfs-sdk` ships a breaking serde change, update the match arms.

---

## 9. Milestone 7 Validation

### Validation Command

```bash
cargo test -p zen-core -p zen-cli
```

### Acceptance Criteria

| # | Criterion | Status |
|---|-----------|--------|
| M7.1 | `znt session start` creates `.zenith/workspaces/{session-id}.db` file | **DONE** |
| M7.2 | `znt session start` response JSON includes `workspace.backend: "agentfs"` | **DONE** |
| M7.3 | Workspace creation failure causes session abandonment (no orphaned sessions) | **DONE** |
| M7.4 | `znt install tokio` records install event in active session's workspace | **DONE** |
| M7.5 | Install continues if workspace audit recording fails (graceful degradation) | **DONE** |
| M7.6 | `znt wrap-up` captures workspace snapshot with tool call metrics | **DONE** |
| M7.7 | Wrap-up completes if snapshot capture fails (graceful degradation) | **DONE** |
| M7.8 | `znt audit --files` returns dual-channel response (entity + file audit) | **DONE** |
| M7.9 | `znt audit --files --merge-timeline` returns chronologically sorted timeline | **DONE** |
| M7.10 | `znt audit --files --session ses-xxx` queries specific session's workspace | **DONE** |
| M7.11 | Both audit channels can fail independently without crashing | **DONE** |
| M7.12 | Session ID validation rejects path traversal attempts | **DONE** |

---

## 10. Validation Traceability Matrix

Every implementation decision traces back to a spike test, upstream validated API, or Phase 5 convention.

| Implementation | Validated By | Evidence |
|----------------|-------------|----------|
| `AgentFS::open(AgentFSOptions::with_path(path))` | Spike 0.7 | `spike_agentfs_persistent_opens` — opens with path, creates DB file |
| `AgentFS::open(AgentFSOptions::ephemeral())` | Spike 0.7 | `spike_agentfs_ephemeral_opens` — in-memory for tests |
| `agent.kv.set(key, &value)` | Spike 0.7 | `spike_agentfs_kv_crud` — typed set/get/delete/keys |
| `agent.kv.get::<T>(key)` → `Option<T>` | Spike 0.7 | `spike_agentfs_kv_structured_data` — serde struct roundtrip |
| `agent.fs.mkdir(path, uid, gid)` | Spike 0.7 | `spike_agentfs_filesystem_ops` — directory creation |
| `agent.tools.record(name, start, end, params, result, error)` | Spike 0.7 | `spike_agentfs_tool_tracking` — positional args, all fields |
| `agent.tools.recent(Some(limit))` | Spike 0.7 | `spike_agentfs_tool_tracking` — returns Vec with names/status |
| `agent.tools.stats_for(name)` | Spike 0.7 | `spike_agentfs_tool_stats` — per-tool success/error counts (validated for future use, not yet in production code path) |
| Full clone→parse→audit→cleanup lifecycle | Spike 0.7 | `spike_agentfs_indexing_workspace_pattern` — end-to-end |
| POSIX-level API (not Turso docs API) | Spike 0.7 | Module-level docstring documents all mismatches |
| `agentfs-sdk` 0.6.0 from crates.io (not git) | Spike 0.7 | `Cargo.toml` uses `agentfs-sdk = "0.6"`, no git URL |
| `require_active_session_id()` pattern | Phase 5 | `commands::shared::session` — reused across all Phase 5 commands |
| `output()` formatter | Phase 5 | `crate::output::output` — JSON/table/raw output |
| Graceful degradation pattern | Phase 5 | `commands/wrap_up/handle.rs` — cloud sync degrades gracefully |
| `SessionStartResponse` with embedded info | Phase 5 | `commands/session/types.rs` — extends existing response struct |
| Dual-channel error pattern | Phase 7 | `commands/audit/files.rs` — both channels report independently |
| Timeline merge (sort by created_at) | Phase 7 | `commands/audit/merge.rs` — unit test validates sort order |
| `tracing::warn!` for non-fatal workspace errors | Phase 5 | Consistent with existing warn patterns in install.rs, wrap_up |

---

## 11. Plan Review — Mismatch Log

Review of this plan against actual implementation source files, following the Phase 6 categorized findings format.

**Methodology**: Every code listing, function signature, data flow claim, gotcha, module path, and traceability reference compared against actual source. Phase 5 conventions cross-referenced for pattern claims.

### Round 1 — Internal Review (plan vs. source code)

| # | Category | Description | Severity | Resolution |
|---|----------|-------------|----------|------------|
| F1 | F | Public function count said 6, actual is 5 (only 5 names listed and 5 exist) | **Medium** | ✅ Fixed: Changed to "5 public functions" in §2 and §A2 |
| F2 | F | Private helper count said 4, actual is 7; §2 and §A2 disagreed on which helpers to list. Missing: `workspace_id()`, `now_epoch_secs()`, `parse_timestamp_from_tool_call()` | **Medium** | ✅ Fixed: Changed to "7 private helpers", reconciled lists, added 3 missing helpers to §A2 table |
| F3 | F | `active_session_file_audit` data flow omitted KV lookup + delegation to `session_file_audit()` + double-open of workspace DB | **Medium** | ✅ Fixed: Data flow now shows `kv.get("session_id")` → delegation → `open_session_workspace()` re-open |
| F4 | F | Module structure omitted `commands/audit.rs` entry point (dispatch with `#[path]` sub-modules); `query.rs`/`search.rs` restructured from Phase 5 flat file | **Medium** | ✅ Fixed: Added `audit.rs` to module diagram with annotation |
| E1 | E | "5 structs" included an enum (`WorkspaceBackend`) | Low | ✅ Fixed: Changed to "1 enum + 4 structs" |
| E2 | E | Prose said `agent.tools.recent(500)`, actual API takes `Option<i64>` — called as `Some(500)` | Low | ✅ Fixed: Updated data flow and §A2 pattern #3 to `Some(500)` / `Some(i64::from(limit))` |
| E3 | E | Fallback ID said `wsa-{timestamp}-{index}`, actual uses `timestamp_micros()` (microseconds) | Low | ✅ Fixed: Changed to `wsa-{timestamp_micros}-{index}` with precision note |
| E4 | E | Entity channel said `search_audit()`/`query_audit()` directly, actual calls `search::fetch()`/`query::fetch()` wrappers | Low | ✅ Fixed: §B5 and data flow updated to show wrapper functions |
| E5 | E | Traceability matrix listed `tools.stats_for()` — validated in spike but unused in production | Low | ✅ Fixed: Added "(validated for future use, not yet in production code path)" note |

### Round 2 — Oracle Review (architectural + semantic accuracy)

| # | Category | Description | Severity | Resolution |
|---|----------|-------------|----------|------------|
| O1 | S | "Workspace isolation" / "filesystem isolation" overstated — CLI does not route FS I/O through AgentFS; it provides session-scoped DB with KV + tool tracking, not FS-level isolation | **High** | ✅ Fixed: Tightened language in **Produces**, scope banner, §1 overview, and §3.1. Clarified AgentFS is audit/metadata layer, not isolation boundary. |
| O2 | S | "File-level audit trail" overstated — only `install_index` is recorded, which has no `path` param; `WorkspaceAuditEntry.path` is usually `None` | **High** | ✅ Fixed: Added clarification in §1 overview, §3.5 rationale, and new gotcha §8.8. |
| O3 | S | "Crash-safe lifecycle (create → use → snapshot → cleanup)" — cleanup is not implemented; workspace DBs persist indefinitely | **Medium** | ✅ Fixed: Removed "cleanup" from **Produces** lifecycle claim. Added gotcha §8.9 documenting DB accumulation. |
| O4 | S | Session ID validation: no length or emptiness check | Low | ✅ Fixed: Added gotcha §8.10. |
| O5 | S | Workspace DB accumulation — `.zenith/workspaces/` grows indefinitely with no retention policy | **Medium** | ✅ Fixed: Added gotcha §8.9 with future cleanup options. |
| O6 | S | `timestamp_micros()` precision claim misleading — source timestamps are epoch seconds (nanos=0) | Low | ✅ Fixed: Added precision note in §3.5. |
| O7 | S | Active workspace mtime heuristic can be biased by reads/queries (SQLite/WAL may update mtime on open) | Low | ✅ Fixed: Folded into §8.5 as "Additional concern." |
| O8 | S | Tool call status parsing via JSON serialization is brittle vs matching `ToolCallStatus` enum directly | Low | ✅ Fixed: Added gotcha §8.11. |

### Summary

**Round 1**: 4 medium + 5 low = 9 findings — all fixed (documentation precision: counts, data flow, module structure)
**Round 2**: 2 high + 2 medium + 4 low = 8 findings — all fixed (semantic accuracy: overstatements, missing gotchas)
**Total**: 17 findings, all resolved. 20 items verified correct (no mismatch).

**Overall assessment**: Plan is now accurate. All code listings match source exactly. All function signatures verified correct. All 11 gotchas cover known limitations. Key semantic corrections: (1) AgentFS provides session-scoped metadata/audit, not filesystem isolation; (2) audit trail is tool-call-level, not file-level, until future event types add `path` parameters; (3) workspace lifecycle is create→use→snapshot with no cleanup — DBs persist indefinitely.

---

## Cross-References

- AgentFS spike: [spike_agentfs.rs](../../crates/zen-cli/src/spike_agentfs.rs) (8 tests, all passing)
- Implementation plan (Phase 7 tasks): [07-implementation-plan.md](./07-implementation-plan.md) §9
- Phase 5 plan (CLI conventions): [25-phase5-cli-shell-plan.md](./25-phase5-cli-shell-plan.md)
- Crate designs (zen-cli §11): [05-crate-designs.md](./05-crate-designs.md)
- Workspace types source: [zen-core/src/workspace.rs](../../crates/zen-core/src/workspace.rs)
- AgentFS wrapper source: [zen-cli/src/workspace/agentfs.rs](../../crates/zen-cli/src/workspace/agentfs.rs)
- Session start wiring: [commands/session/start.rs](../../crates/zen-cli/src/commands/session/start.rs)
- Wrap-up wiring: [commands/wrap_up/handle.rs](../../crates/zen-cli/src/commands/wrap_up/handle.rs)
- Audit files wiring: [commands/audit/files.rs](../../crates/zen-cli/src/commands/audit/files.rs)
- Audit merge: [commands/audit/merge.rs](../../crates/zen-cli/src/commands/audit/merge.rs)
- Install wiring: [commands/install.rs](../../crates/zen-cli/src/commands/install.rs)
