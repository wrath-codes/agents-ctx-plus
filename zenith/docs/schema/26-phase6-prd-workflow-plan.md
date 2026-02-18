# Phase 6: PRD Workflow — Implementation Plan

**Version**: 2026-02-17
**Status**: Planning
**Depends on**: Phase 5 (zen-cli — working `znt` binary with issue/task/link/audit commands — **DONE**)
**Produces**: Milestone 6 — Full PRD workflow via `znt prd` commands. LLMs can create PRDs, generate task trees, execute tasks one-by-one, and track progress across sessions.

> **⚠️ Scope**: Phase 6 is **CLI wiring + orchestration only**. All underlying entity operations already exist in upstream crates (issue CRUD via `IssueRepo`, task CRUD via `TaskRepo`, entity linking via `LinkRepo`, audit via `AuditRepo`). This phase adds a `znt prd` command surface that orchestrates those existing operations into the ai-dev-tasks PRD workflow. No new database tables, no new storage backends.

---

## Table of Contents

1. [Overview](#1-overview)
2. [Key Decisions](#2-key-decisions)
3. [Architecture](#3-architecture)
4. [PR 1 — Stream A: CLI Structs + PRD Create/Update/Get](#4-pr-1--stream-a-cli-structs--prd-createupdateget)
5. [PR 2 — Stream B: Task Generation + Subtasks + Complete + List](#5-pr-2--stream-b-task-generation--subtasks--complete--list)
6. [Execution Order](#6-execution-order)
7. [Gotchas & Warnings](#7-gotchas--warnings)
8. [Milestone 6 Validation](#8-milestone-6-validation)
9. [Validation Traceability Matrix](#9-validation-traceability-matrix)

---

## 1. Overview

**Goal**: Add `znt prd` subcommand tree that implements the ai-dev-tasks PRD workflow using existing Zenith entities. PRDs are epic issues; tasks are linked to epics; sub-tasks link to parents via `depends_on` entity links. Progress is tracked through task status aggregation.

**Crates touched**:
- `zen-cli` — **medium**: new `prd` subcommand enum, `commands/prd.rs` module with 7 subcommand handlers
- `zen-db` — **none**: all needed repo methods already exist (see §3 Upstream Dependencies)

**Dependency changes needed**:
- `zen-cli`: No new crate dependencies (all needed deps already present)
- `zen-cli/src/cli/subcommands/`: Add `prd.rs` with `PrdCommands` enum
- `zen-cli/src/cli/subcommands/mod.rs`: Re-export `PrdCommands`
- `zen-cli/src/cli/root_commands.rs`: Add `Prd` variant to `Commands`
- `zen-cli/src/commands/mod.rs`: Add `pub mod prd`
- `zen-cli/src/commands/dispatch.rs`: Add `Prd` dispatch arm

**Estimated deliverables**: ~8 new production files, ~600–900 LOC production code, ~300 LOC tests

**PR strategy**: 2 PRs by stream. Stream A provides the foundation (CLI structs + core commands), Stream B adds task generation and completion.

| PR | Stream | Contents | Depends On |
|----|--------|----------|------------|
| PR 1 | A: CLI Structs + Core | `PrdCommands` enum, `commands/prd/create.rs`, `commands/prd/update.rs`, `commands/prd/get.rs`, `commands/prd/list.rs`, dispatch wiring | None (clean start) |
| PR 2 | B: Task Tree + Complete | `commands/prd/tasks.rs`, `commands/prd/subtasks.rs`, `commands/prd/complete.rs`, integration tests | Stream A |

---

## 2. Key Decisions

All decisions derive from the PRD workflow design ([06-prd-workflow.md](./06-prd-workflow.md)) and validated Phase 5 patterns.

### 2.1 PRDs Are Epic Issues — No New Entity Type

**Decision**: A PRD is an issue with `type = epic`. No new database table or entity struct.

**Rationale**: The `issues` table already has `type` column with `epic` as a valid `IssueType` variant. The PRD markdown content is stored in the `description` field. This is documented in [06-prd-workflow.md §6](./06-prd-workflow.md#6-data-model-integration) and confirmed by the data model — no new tables needed.

**Validated in**: Phase 2 (`IssueRepo` handles all issue types including `epic`), Phase 5 (`znt issue create --type epic` already works).

### 2.2 Task Hierarchy via Entity Links — No New Columns

**Decision**: Parent tasks are linked to the epic via `issue_id` (existing FK on `tasks` table). Sub-tasks link to their parent task via `entity_links` with `depends_on` relation. No `parent_task_id` column.

**Rationale**: The `tasks.issue_id` FK already provides epic→task linkage. The `entity_links` table provides the sub-task→parent-task linkage with `depends_on` relation. This is the existing approach documented in [06-prd-workflow.md §4](./06-prd-workflow.md#4-step-2-generate-tasks).

**Validated in**: Phase 2 (`LinkRepo.create_link()` with `Relation::DependsOn`), Phase 5 (`znt link` command works).

### 2.3 Progress Tracking via Task Status Aggregation

**Decision**: `znt prd get` aggregates task statuses for the linked epic. No separate progress tracking table.

**Rationale**: `get_tasks_for_issue(epic_id)` returns all tasks linked to the epic. Counting by `status` gives `total`, `done`, `in_progress`, `open`, `blocked` counts. This matches the ai-dev-tasks progress display.

**Validated in**: Phase 2 (`TaskRepo.get_tasks_for_issue()`), Phase 5 (`znt issue get` already returns `IssueDetailResponse { issue, children, tasks }`).

### 2.4 Command Handler Pattern: Reuse Existing Shared Utilities

**Decision**: PRD command handlers follow the same conventions as all other Phase 5 commands:
- Use `require_active_session_id()` from `commands::shared::session`
- Use `parse_enum()` from `commands::shared::parse`
- Use `effective_limit()` from `commands::shared::limit`
- Use `output()` from `crate::output`
- Accept `&AppContext` and `&GlobalFlags`

**Rationale**: Consistency with 24 existing command modules. No new utilities needed.

### 2.5 Tasks JSON Argument: `--tasks` as JSON Array

**Decision**: `znt prd tasks <epic-id> --tasks '["title1", "title2", ...]'` accepts a JSON array of task titles. `znt prd subtasks <parent-task-id> --tasks '["sub1", "sub2"]'` uses the same pattern.

**Rationale**: Matches the design in [06-prd-workflow.md §7](./06-prd-workflow.md#7-cli-commands). LLMs naturally generate JSON arrays. Using `--tasks` as a single JSON string avoids shell escaping issues with repeated `--task` flags.

### 2.6 PRD Get Response: Aggregated View

**Decision**: `znt prd get <id>` returns a `PrdDetailResponse` struct with the epic issue, task progress counts, task items, linked findings (tagged `relevant-files`), and linked hypotheses (status `unverified` for open questions).

**Rationale**: Matches the output spec in [06-prd-workflow.md §7 "znt prd get"](./06-prd-workflow.md#7-cli-commands). The LLM needs a single query to see the full PRD state for multi-session continuity.

---

## 3. Architecture

### Module Structure

```
zen-cli/src/
├── cli/
│   ├── subcommands/
│   │   ├── prd.rs          # NEW — PrdCommands enum
│   │   └── mod.rs          # MODIFIED — add pub mod prd + re-export
│   └── root_commands.rs    # MODIFIED — add Prd variant to Commands
├── commands/
│   ├── prd.rs              # NEW — handler dispatch (like issue.rs pattern)
│   ├── prd/
│   │   ├── create.rs       # NEW — znt prd create
│   │   ├── update.rs       # NEW — znt prd update
│   │   ├── get.rs          # NEW — znt prd get (aggregated view)
│   │   ├── list.rs         # NEW — znt prd list (epic issues only)
│   │   ├── tasks.rs        # NEW — znt prd tasks (generate parent tasks)
│   │   ├── subtasks.rs     # NEW — znt prd subtasks (generate sub-tasks)
│   │   └── complete.rs     # NEW — znt prd complete (mark epic done)
│   ├── dispatch.rs         # MODIFIED — add Prd dispatch arm
│   └── mod.rs              # MODIFIED — add pub mod prd
```

### Upstream Dependencies — All Ready

| Dependency | Method | Crate | Status | Usage |
|------------|--------|-------|--------|-------|
| `create_issue()` | `create_issue(session_id, title, IssueType::Epic, priority, description, parent)` | zen-db | **DONE** | PRD create |
| `get_issue()` | `get_issue(id)` | zen-db | **DONE** | PRD get/update |
| `update_issue()` | `update_issue(session_id, issue_id, IssueUpdate)` | zen-db | **DONE** | PRD update content |
| `transition_issue()` | `transition_issue(session_id, issue_id, IssueStatus)` | zen-db | **DONE** | PRD complete |
| `list_issues()` | `list_issues(limit)` | zen-db | **DONE** | PRD list (filter by type=epic) |
| `search_issues()` | `search_issues(query, limit)` | zen-db | **DONE** | PRD list with search |
| `get_tasks_for_issue()` | `get_tasks_for_issue(issue_id)` | zen-db | **DONE** | PRD get progress |
| `create_task()` | `create_task(session_id, title, description, issue_id, research_id)` | zen-db | **DONE** | PRD tasks/subtasks |
| `create_link()` | `create_link(session_id, source_type, source_id, target_type, target_id, relation)` | zen-db | **DONE** | Sub-task→parent depends_on |
| `get_links_from()` | `get_links_from(source_type, source_id)` | zen-db | **DONE** | Find linked findings/hypotheses |
| `require_active_session_id()` | Shared helper | zen-cli | **DONE** | All PRD commands |
| `parse_enum()` | Shared helper | zen-cli | **DONE** | Status/type parsing |
| `output()` | Output formatter | zen-cli | **DONE** | Response rendering |

### Data Flow

```
znt prd create --title "Feature X"
  → create_issue(session_id, "Feature X", IssueType::Epic, 3, None, None)
  → returns iss-xxx

znt prd update <iss-xxx> --content "<PRD markdown>"
  → update_issue(session_id, iss-xxx, IssueUpdate { description: Some(content) })

znt prd tasks <iss-xxx> --tasks '["Branch", "Models", "Logic", "Tests"]'
  → for each title: create_task(session_id, title, None, Some(iss-xxx), None)
  → returns list of created tasks with confirmation message

znt prd subtasks <tsk-parent> --tasks '["Define schema", "Add migration"]' --epic <iss-xxx>
  → for each title: create_task(session_id, title, None, Some(iss-xxx), None)
  → for each new task: create_link(session_id, Task, tsk-new, Task, tsk-parent, DependsOn)
  → returns list of created sub-tasks

znt prd get <iss-xxx>
  → get_issue(iss-xxx) → epic info
  → get_tasks_for_issue(iss-xxx) → all tasks → aggregate by status
  → get_links_from(Issue, iss-xxx) → find linked findings/hypotheses
  → returns PrdDetailResponse

znt prd complete <iss-xxx>
  → transition_issue(session_id, iss-xxx, IssueStatus::Done)
  → returns updated issue

znt prd list [--status X] [--limit N]
  → list_issues(limit) → filter by type == Epic
  → for each epic: get_tasks_for_issue(epic_id) → progress counts
  → returns list with progress
```

---

## 4. PR 1 — Stream A: CLI Structs + PRD Create/Update/Get

**Tasks**: 6.1, 6.2, 6.5, 6.7
**Estimated LOC**: ~400 production, ~150 tests

### A1. `src/cli/subcommands/prd.rs` — PrdCommands Enum

```rust
use clap::Subcommand;

/// PRD (Product Requirements Document) workflow commands.
#[derive(Clone, Debug, Subcommand)]
pub enum PrdCommands {
    /// Create a new PRD (creates an epic issue).
    Create {
        #[arg(long)]
        title: String,
        #[arg(long)]
        description: Option<String>,
    },
    /// Update PRD content (sets epic description).
    Update {
        /// Epic issue ID.
        id: String,
        /// PRD markdown content.
        #[arg(long)]
        content: String,
    },
    /// Get full PRD with tasks, progress, findings, and open questions.
    Get {
        /// Epic issue ID.
        id: String,
    },
    /// Generate parent tasks for a PRD.
    Tasks {
        /// Epic issue ID.
        id: String,
        /// JSON array of task titles.
        #[arg(long)]
        tasks: String,
    },
    /// Generate sub-tasks for a parent task.
    Subtasks {
        /// Parent task ID.
        id: String,
        /// Epic issue ID (for issue_id linkage).
        #[arg(long)]
        epic: String,
        /// JSON array of sub-task titles.
        #[arg(long)]
        tasks: String,
    },
    /// Mark a PRD as completed.
    Complete {
        /// Epic issue ID.
        id: String,
    },
    /// List all PRDs (epic issues).
    List {
        #[arg(long)]
        status: Option<String>,
        #[arg(long)]
        search: Option<String>,
        #[arg(long)]
        limit: Option<u32>,
    },
}
```

### A2. Wiring Changes

**`src/cli/subcommands/mod.rs`** — Add:

```rust
pub mod prd;
pub use prd::PrdCommands;
```

**`src/cli/root_commands.rs`** — Add to `Commands` enum:

```rust
/// PRD workflow.
Prd {
    #[command(subcommand)]
    action: PrdCommands,
},
```

And add `PrdCommands` to the import from subcommands.

**`src/commands/mod.rs`** — Add:

```rust
pub mod prd;
```

**`src/commands/dispatch.rs`** — Add to match:

```rust
Commands::Prd { action } => commands::prd::handle(&action, ctx, flags).await,
```

### A3. `src/commands/prd.rs` — Handler Dispatch

```rust
#[path = "prd/create.rs"]
mod create;
#[path = "prd/update.rs"]
mod update;
#[path = "prd/get.rs"]
mod get;
#[path = "prd/list.rs"]
mod list;
#[path = "prd/tasks.rs"]
mod tasks;
#[path = "prd/subtasks.rs"]
mod subtasks;
#[path = "prd/complete.rs"]
mod complete;

use crate::cli::GlobalFlags;
use crate::cli::subcommands::PrdCommands;
use crate::context::AppContext;

/// Handle `znt prd`.
pub async fn handle(
    action: &PrdCommands,
    ctx: &mut AppContext,
    flags: &GlobalFlags,
) -> anyhow::Result<()> {
    match action {
        PrdCommands::Create { title, description } => {
            create::run(title, description.as_deref(), ctx, flags).await
        }
        PrdCommands::Update { id, content } => {
            update::run(id, content, ctx, flags).await
        }
        PrdCommands::Get { id } => get::run(id, ctx, flags).await,
        PrdCommands::List { status, search, limit } => {
            list::run(status.as_deref(), search.as_deref(), *limit, ctx, flags).await
        }
        PrdCommands::Tasks { id, tasks } => {
            tasks::run(id, tasks, ctx, flags).await
        }
        PrdCommands::Subtasks { id, epic, tasks } => {
            subtasks::run(id, epic, tasks, ctx, flags).await
        }
        PrdCommands::Complete { id } => complete::run(id, ctx, flags).await,
    }
}
```

### A4. `src/commands/prd/create.rs` — PRD Create (task 6.1)

```rust
use serde::Serialize;
use zen_core::entities::Issue;
use zen_core::enums::IssueType;

use crate::cli::GlobalFlags;
use crate::commands::shared::session::require_active_session_id;
use crate::context::AppContext;
use crate::output::output;

#[derive(Debug, Serialize)]
struct PrdCreateResponse {
    prd: Issue,
}

pub async fn run(
    title: &str,
    description: Option<&str>,
    ctx: &AppContext,
    flags: &GlobalFlags,
) -> anyhow::Result<()> {
    let session_id = require_active_session_id(ctx).await?;
    let issue = ctx
        .service
        .create_issue(&session_id, title, IssueType::Epic, 3, description, None)
        .await?;

    output(&PrdCreateResponse { prd: issue }, flags.format)
}
```

### A5. `src/commands/prd/update.rs` — PRD Update (task 6.2)

```rust
use zen_db::updates::issue::IssueUpdateBuilder;

use crate::cli::GlobalFlags;
use crate::commands::shared::session::require_active_session_id;
use crate::context::AppContext;
use crate::output::output;

pub async fn run(
    id: &str,
    content: &str,
    ctx: &AppContext,
    flags: &GlobalFlags,
) -> anyhow::Result<()> {
    let session_id = require_active_session_id(ctx).await?;

    // Verify it's an epic
    let issue = ctx.service.get_issue(id).await?;
    if issue.issue_type != zen_core::enums::IssueType::Epic {
        anyhow::bail!("Issue '{id}' is not an epic (type: {}). PRD commands only work with epics.", issue.issue_type);
    }

    let update = IssueUpdateBuilder::new().description(Some(content.to_string())).build();
    let updated = ctx.service.update_issue(&session_id, id, update).await?;

    output(&updated, flags.format)
}
```

### A6. `src/commands/prd/get.rs` — PRD Get with Aggregation (task 6.5)

```rust
use serde::Serialize;
use zen_core::entities::{Finding, Hypothesis, Issue, Task};
use zen_core::enums::{EntityType, IssueType, TaskStatus};

use crate::cli::GlobalFlags;
use crate::context::AppContext;
use crate::output::output;

#[derive(Debug, Serialize)]
struct PrdDetailResponse {
    prd: Issue,
    tasks: TaskProgress,
    findings: Vec<Finding>,
    open_questions: Vec<Hypothesis>,
}

#[derive(Debug, Serialize)]
struct TaskProgress {
    total: usize,
    done: usize,
    in_progress: usize,
    open: usize,
    blocked: usize,
    items: Vec<Task>,
}

pub async fn run(id: &str, ctx: &AppContext, flags: &GlobalFlags) -> anyhow::Result<()> {
    let issue = ctx.service.get_issue(id).await?;
    if issue.issue_type != IssueType::Epic {
        anyhow::bail!("Issue '{id}' is not an epic (type: {}). Use 'znt issue get' for non-epic issues.", issue.issue_type);
    }

    // Get all tasks linked to this epic
    let tasks = ctx.service.get_tasks_for_issue(id).await?;

    let done = tasks.iter().filter(|t| t.status == TaskStatus::Done).count();
    let in_progress = tasks.iter().filter(|t| t.status == TaskStatus::InProgress).count();
    let blocked = tasks.iter().filter(|t| t.status == TaskStatus::Blocked).count();
    let open = tasks.iter().filter(|t| t.status == TaskStatus::Open).count();

    // Get linked findings and hypotheses via entity_links
    let links = ctx.service.get_links_from(EntityType::Issue, id).await?;
    let mut findings = Vec::new();
    let mut open_questions = Vec::new();

    for link in &links {
        match link.target_type {
            EntityType::Finding => {
                if let Ok(finding) = ctx.service.get_finding(&link.target_id).await {
                    findings.push(finding);
                }
            }
            EntityType::Hypothesis => {
                if let Ok(hyp) = ctx.service.get_hypothesis(&link.target_id).await {
                    if hyp.status == zen_core::enums::HypothesisStatus::Unverified {
                        open_questions.push(hyp);
                    }
                }
            }
            _ => {}
        }
    }

    output(
        &PrdDetailResponse {
            prd: issue,
            tasks: TaskProgress {
                total: tasks.len(),
                done,
                in_progress,
                open,
                blocked,
                items: tasks,
            },
            findings,
            open_questions,
        },
        flags.format,
    )
}
```

### A7. `src/commands/prd/list.rs` — PRD List (task 6.7)

```rust
use serde::Serialize;
use zen_core::entities::Issue;
use zen_core::enums::{IssueStatus, IssueType};

use crate::cli::GlobalFlags;
use crate::commands::shared::limit::effective_limit;
use crate::commands::shared::parse::parse_enum;
use crate::context::AppContext;
use crate::output::output;

#[derive(Debug, Serialize)]
struct PrdListItem {
    #[serde(flatten)]
    issue: Issue,
    tasks_total: usize,
    tasks_done: usize,
}

pub async fn run(
    status: Option<&str>,
    search: Option<&str>,
    limit: Option<u32>,
    ctx: &AppContext,
    flags: &GlobalFlags,
) -> anyhow::Result<()> {
    let limit = effective_limit(limit, flags.limit, 20);
    // Over-fetch since we filter by type=epic client-side
    let fetch_limit = limit.saturating_mul(5).min(500);

    let mut issues: Vec<Issue> = if let Some(query) = search {
        ctx.service.search_issues(query, fetch_limit).await?
    } else {
        ctx.service.list_issues(fetch_limit).await?
    };

    // Filter to epics only
    issues.retain(|i| i.issue_type == IssueType::Epic);

    // Apply status filter if provided
    if let Some(status) = status {
        let status = parse_enum::<IssueStatus>(status, "status")?;
        issues.retain(|i| i.status == status);
    }

    issues.truncate(usize::try_from(limit)?);

    // Enrich with task progress
    let mut items = Vec::with_capacity(issues.len());
    for issue in issues {
        let tasks = ctx.service.get_tasks_for_issue(&issue.id).await?;
        let tasks_done = tasks.iter().filter(|t| t.status == zen_core::enums::TaskStatus::Done).count();
        items.push(PrdListItem {
            tasks_total: tasks.len(),
            tasks_done,
            issue,
        });
    }

    output(&items, flags.format)
}
```

### A8. Tests for Stream A

- `znt prd create --title "Feature X"` → creates epic issue, returns `PrdCreateResponse` with `type = epic`
- `znt prd create --title "Feature X" --description "Initial idea"` → epic has description set
- `znt prd update <id> --content "<markdown>"` → updates description, returns updated issue
- `znt prd update <non-epic-id>` → error: "not an epic"
- `znt prd get <id>` → returns `PrdDetailResponse` with correct task counts
- `znt prd get <id>` with 0 tasks → counts all zero, items empty
- `znt prd get <non-epic-id>` → error: "not an epic"
- `znt prd list` → returns only epic issues (filters out bug/feature/spike/request)
- `znt prd list --status open` → returns only open epics
- `znt prd list --search "Profile"` → FTS search works
- All responses are valid JSON in all output formats

---

## 5. PR 2 — Stream B: Task Generation + Subtasks + Complete

**Tasks**: 6.3, 6.4, 6.6
**Depends on**: Stream A
**Estimated LOC**: ~300 production, ~200 tests

### B1. `src/commands/prd/tasks.rs` — Generate Parent Tasks (task 6.3)

```rust
use serde::Serialize;
use zen_core::entities::Task;
use zen_core::enums::IssueType;

use crate::cli::GlobalFlags;
use crate::commands::shared::session::require_active_session_id;
use crate::context::AppContext;
use crate::output::output;

#[derive(Debug, Serialize)]
struct PrdTasksResponse {
    tasks: Vec<Task>,
    message: &'static str,
}

pub async fn run(
    epic_id: &str,
    tasks_json: &str,
    ctx: &AppContext,
    flags: &GlobalFlags,
) -> anyhow::Result<()> {
    let session_id = require_active_session_id(ctx).await?;

    // Verify it's an epic
    let issue = ctx.service.get_issue(epic_id).await?;
    if issue.issue_type != IssueType::Epic {
        anyhow::bail!("Issue '{epic_id}' is not an epic. PRD tasks can only be generated for epics.");
    }

    // Parse task titles from JSON array
    let titles: Vec<String> = serde_json::from_str(tasks_json)
        .map_err(|e| anyhow::anyhow!("Invalid --tasks JSON: {e}. Expected: '[\"title1\", \"title2\"]'"))?;

    if titles.is_empty() {
        anyhow::bail!("--tasks array is empty. Provide at least one task title.");
    }

    let mut created = Vec::with_capacity(titles.len());
    for title in &titles {
        let task = ctx
            .service
            .create_task(&session_id, title, None, Some(epic_id), None)
            .await?;
        created.push(task);
    }

    output(
        &PrdTasksResponse {
            tasks: created,
            message: "High-level tasks generated. Ask the user to confirm before generating sub-tasks.",
        },
        flags.format,
    )
}
```

### B2. `src/commands/prd/subtasks.rs` — Generate Sub-Tasks (task 6.4)

```rust
use serde::Serialize;
use zen_core::entities::Task;
use zen_core::enums::{EntityType, Relation};

use crate::cli::GlobalFlags;
use crate::commands::shared::session::require_active_session_id;
use crate::context::AppContext;
use crate::output::output;

#[derive(Debug, Serialize)]
struct PrdSubtasksResponse {
    subtasks: Vec<Task>,
    parent_task_id: String,
}

pub async fn run(
    parent_task_id: &str,
    epic_id: &str,
    tasks_json: &str,
    ctx: &AppContext,
    flags: &GlobalFlags,
) -> anyhow::Result<()> {
    let session_id = require_active_session_id(ctx).await?;

    // Verify parent task exists
    let _parent = ctx.service.get_task(parent_task_id).await?;

    // Verify epic exists and is actually an epic
    let epic = ctx.service.get_issue(epic_id).await?;
    if epic.issue_type != zen_core::enums::IssueType::Epic {
        anyhow::bail!("Issue '{epic_id}' is not an epic. Sub-tasks can only be created under epic issues.");
    }

    // Parse sub-task titles from JSON array
    let titles: Vec<String> = serde_json::from_str(tasks_json)
        .map_err(|e| anyhow::anyhow!("Invalid --tasks JSON: {e}. Expected: '[\"title1\", \"title2\"]'"))?;

    if titles.is_empty() {
        anyhow::bail!("--tasks array is empty. Provide at least one sub-task title.");
    }

    let mut created = Vec::with_capacity(titles.len());
    for title in &titles {
        // Create sub-task linked to the same epic
        let task = ctx
            .service
            .create_task(&session_id, title, None, Some(epic_id), None)
            .await?;

        // Link sub-task → parent via depends_on
        ctx.service
            .create_link(
                &session_id,
                EntityType::Task,
                &task.id,
                EntityType::Task,
                parent_task_id,
                Relation::DependsOn,
            )
            .await?;

        created.push(task);
    }

    output(
        &PrdSubtasksResponse {
            subtasks: created,
            parent_task_id: parent_task_id.to_string(),
        },
        flags.format,
    )
}
```

### B3. `src/commands/prd/complete.rs` — PRD Complete (task 6.6)

```rust
use zen_core::enums::{IssueStatus, IssueType};

use crate::cli::GlobalFlags;
use crate::commands::shared::session::require_active_session_id;
use crate::context::AppContext;
use crate::output::output;

pub async fn run(id: &str, ctx: &AppContext, flags: &GlobalFlags) -> anyhow::Result<()> {
    let session_id = require_active_session_id(ctx).await?;

    // Verify it's an epic
    let issue = ctx.service.get_issue(id).await?;
    if issue.issue_type != IssueType::Epic {
        anyhow::bail!("Issue '{id}' is not an epic. Use 'znt issue update --status done' for non-epic issues.");
    }

    // Handle terminal states
    if issue.status == IssueStatus::Done {
        return output(&issue, flags.format);
    }
    if issue.status == IssueStatus::Abandoned {
        anyhow::bail!("PRD '{id}' is abandoned and cannot be completed. Use 'znt issue update --status in_progress' to reopen first.");
    }

    // Transition: must be in_progress → done (or open/blocked → in_progress first)
    // If still open or blocked, transition to in_progress first
    if issue.status == IssueStatus::Open || issue.status == IssueStatus::Blocked {
        ctx.service
            .transition_issue(&session_id, id, IssueStatus::InProgress)
            .await?;
    }

    let completed = ctx
        .service
        .transition_issue(&session_id, id, IssueStatus::Done)
        .await?;

    output(&completed, flags.format)
}
```

### B4. Tests for Stream B

- `znt prd tasks <epic-id> --tasks '["Branch", "Models", "Tests"]'` → creates 3 tasks linked to epic
- `znt prd tasks <epic-id> --tasks '[]'` → error: empty array
- `znt prd tasks <epic-id> --tasks 'not json'` → error: invalid JSON
- `znt prd tasks <non-epic-id>` → error: not an epic
- `znt prd subtasks <task-id> --epic <epic-id> --tasks '["Sub A", "Sub B"]'` → creates 2 sub-tasks with `depends_on` links to parent
- `znt prd subtasks <nonexistent-task>` → error: task not found
- `znt prd complete <epic-id>` → transitions to done, returns updated issue
- `znt prd complete <non-epic-id>` → error: not an epic
- Full lifecycle integration test:
  1. `znt session start`
  2. `znt prd create --title "Feature X"` → get epic ID
  3. `znt prd update <epic-id> --content "# PRD..."`
  4. `znt prd tasks <epic-id> --tasks '["Setup", "Core", "Tests"]'`
  5. `znt prd subtasks <parent-task-id> --epic <epic-id> --tasks '["Sub 1", "Sub 2"]'`
  6. `znt prd get <epic-id>` → shows 5 tasks, 0 done
  7. `znt task complete <task-id>` (use existing command)
  8. `znt prd get <epic-id>` → shows 5 tasks, 1 done
  9. `znt prd complete <epic-id>` → epic status = done
  10. `znt prd list` → shows epic with progress

---

## 6. Execution Order

### Phase 6 Task Checklist

```
Phase 6 Prerequisites (all DONE):
  [x] Phase 5: zen-cli (working znt binary, all commands)
  [x] Phase 2: IssueRepo (create_issue with IssueType::Epic)
  [x] Phase 2: TaskRepo (create_task with issue_id FK)
  [x] Phase 2: LinkRepo (create_link with DependsOn)
  [x] Phase 2: AuditRepo (automatic audit on all mutations)

Stream A: CLI Structs + Core (tasks 6.1, 6.2, 6.5, 6.7)
  [ ] A1. Create src/cli/subcommands/prd.rs — PrdCommands enum
  [ ] A2. Update src/cli/subcommands/mod.rs — add pub mod prd + re-export
  [ ] A3. Update src/cli/root_commands.rs — add Prd variant + PrdCommands import
  [ ] A4. Create src/commands/prd.rs — handler dispatch
  [ ] A5. Create src/commands/prd/create.rs — znt prd create
  [ ] A6. Create src/commands/prd/update.rs — znt prd update
  [ ] A7. Create src/commands/prd/get.rs — aggregated PRD view
  [ ] A8. Create src/commands/prd/list.rs — epic-filtered list with progress
  [ ] A9. Update src/commands/mod.rs — add pub mod prd
  [ ] A10. Update src/commands/dispatch.rs — add Prd dispatch arm
  [ ] A11. Tests: create, update, get, list

Stream B: Task Tree + Complete (tasks 6.3, 6.4, 6.6)
  [ ] B1. Create src/commands/prd/tasks.rs — generate parent tasks
  [ ] B2. Create src/commands/prd/subtasks.rs — generate sub-tasks with depends_on links
  [ ] B3. Create src/commands/prd/complete.rs — mark epic done
  [ ] B4. Tests: tasks, subtasks, complete, full lifecycle integration
```

### Execution Sequence

```
Stream A ──────► Stream B
```

Stream B depends on Stream A (needs the dispatch wiring and epic verification patterns).

---

## 7. Gotchas & Warnings

### 7.1 Epic Issue Type Validation

All PRD commands except `create` must verify the target issue is an epic. Non-epic issues should be rejected with a clear error pointing the user to the standard `znt issue` commands instead. This guard prevents accidental misuse of `znt prd update` on a bug or feature issue.

### 7.2 Task Status Transitions Enforce State Machine

`IssueStatus` has a state machine: `Open → InProgress → Done`. `transition_issue()` validates allowed transitions. `znt prd complete` on an `Open` epic must first transition to `InProgress` before reaching `Done`. The `complete.rs` handler handles this two-step transition.

### 7.3 `list_issues()` Returns All Types — Client-Side Filtering

There is no `list_issues_by_type()` method. `list_issues(limit)` returns all issue types ordered by priority and date. PRD list must filter client-side (`.retain(|i| i.issue_type == IssueType::Epic)`). To compensate, fetch with a multiplied limit (5x, capped at 500) — same pattern used in `issue/list.rs`.

### 7.4 `--tasks` JSON Parsing

The `--tasks` argument is a single JSON string. Shell quoting can be tricky: `--tasks '["a", "b"]'` works in bash/zsh, but may need escaping on Windows. LLMs generate this naturally. The error message on invalid JSON should include the expected format.

### 7.5 Sub-Task Epic Linkage

Sub-tasks need `--epic <epic-id>` in addition to the parent task ID because `create_task()` links via `issue_id` FK. Without the epic ID, sub-tasks wouldn't appear in `get_tasks_for_issue(epic_id)` results. This differs from the original [06-prd-workflow.md](./06-prd-workflow.md) design which only showed `parent-task-id` — the epic ID is a required addition.

### 7.6 Progress Query is N+1

`znt prd list` calls `get_tasks_for_issue()` for each epic in the list. With many PRDs this could be slow. For MVP this is acceptable. Optimization path: add a `count_tasks_by_status(issue_id)` SQL query returning counts directly. Not needed until users report >50 concurrent PRDs.

### 7.7 `EmbeddingEngine` Mutability — PRD Commands Don't Need It

PRD commands only use `ZenService` (libsql). They don't need `EmbeddingEngine`, `ZenLake`, or `SourceFileStore`. The `ctx: &AppContext` (not `&mut`) suffices for all PRD handlers. However, the dispatch function passes `&mut AppContext` uniformly. PRD handlers should accept `&AppContext` in their individual function signatures (the dispatch match arm can auto-reborrow).

### 7.8 Existing `znt issue create --type epic` Still Works

`znt prd create` is a convenience wrapper around `znt issue create --type epic`. Both commands work — `znt prd create` is the guided workflow entry point, `znt issue create --type epic` is the low-level escape hatch. They produce identical database records.

---

## 8. Milestone 6 Validation

### Validation Commands

```bash
# Full workspace build
cargo build --workspace

# Phase 6 specific tests
cargo test -p zen-cli -- prd

# Clippy
cargo clippy -p zen-cli --no-deps -- -D warnings

# Binary help
./target/debug/znt prd --help
./target/debug/znt prd create --help
./target/debug/znt prd tasks --help
```

### Integration Test Sequence

The following sequence must work end-to-end:

```bash
# 1. Initialize and start session
cd /tmp/test-project && git init && cargo init
znt init
znt session start

# 2. Create a PRD
znt prd create --title "User Profile Editing"
# → {"prd": {"id": "iss-xxx", "type": "epic", "status": "open"}}

# 3. Update with PRD content
znt prd update iss-xxx --content "# User Profile Editing\n\n## Goals\n..."

# 4. Generate parent tasks
znt prd tasks iss-xxx --tasks '["Create feature branch", "Set up data models", "Implement core logic", "Add tests"]'
# → {"tasks": [...], "message": "High-level tasks generated..."}

# 5. Generate sub-tasks for "Set up data models"
znt prd subtasks tsk-yyy --epic iss-xxx --tasks '["Define User schema", "Add migrations", "Create repository trait"]'
# → {"subtasks": [...], "parent_task_id": "tsk-yyy"}

# 6. Check PRD state
znt prd get iss-xxx
# → {"prd": {...}, "tasks": {"total": 7, "done": 0, ...}, ...}

# 7. Execute tasks (using existing commands)
znt task update tsk-aaa --status in_progress
znt task complete tsk-aaa
znt log "src/models/user.rs#1-45" --task tsk-aaa

# 8. Check progress
znt prd get iss-xxx
# → {"tasks": {"total": 7, "done": 1, "in_progress": 0, "open": 6, ...}}

# 9. Link a finding to the PRD
znt finding create --content "User model needs email validation"
znt link issue iss-xxx finding fnd-zzz relates_to

# 10. Check PRD now shows finding
znt prd get iss-xxx
# → {"findings": [{"id": "fnd-zzz", ...}]}

# 11. List all PRDs
znt prd list
znt prd list --status open

# 12. Complete the PRD
znt prd complete iss-xxx
# → {"id": "iss-xxx", "status": "done"}

# 13. Verify audit trail captured everything
znt audit --entity-id iss-xxx
```

### Test Count Targets

| Area | Expected Tests | Coverage |
|------|---------------|----------|
| CLI parsing | ~5 | PrdCommands enum, all subcommands parse |
| create/update | ~5 | Create epic, update content, non-epic rejection |
| get/list | ~6 | Aggregation, filtering, empty state |
| tasks/subtasks | ~6 | JSON parsing, creation, linking, errors |
| complete | ~3 | Status transition, non-epic rejection |
| integration | ~2 | Full lifecycle, multi-session |
| **Total** | **~27** | |

### Milestone 6 Acceptance Criteria

- [ ] `cargo build -p zen-cli` compiles with `znt prd` commands
- [ ] `cargo test -p zen-cli -- prd` — all tests pass
- [ ] `cargo clippy -p zen-cli --no-deps -- -D warnings` — clean
- [ ] `znt prd create` → creates epic issue
- [ ] `znt prd update` → stores PRD markdown in epic description
- [ ] `znt prd tasks` → creates parent tasks linked to epic
- [ ] `znt prd subtasks` → creates sub-tasks with `depends_on` links to parent
- [ ] `znt prd get` → returns aggregated view with task progress, findings, open questions
- [ ] `znt prd complete` → transitions epic to `done`
- [ ] `znt prd list` → lists only epic issues with progress percentages
- [ ] PRDs persist across sessions (start new session, `znt prd get` shows correct state)
- [ ] Multiple PRDs can coexist (`znt prd list` shows all)
- [ ] All output respects `--format json|table|raw`

---

## 9. Validation Traceability Matrix

This matrix maps Phase 6 behaviors to their validation evidence.

| Area | Claim | Status | Evidence | Source |
|------|-------|--------|----------|--------|
| Epic issue type | `IssueType::Epic` is a valid variant | Validated | Phase 1, zen-core enums | `zen-core/src/enums.rs:166` |
| Create epic | `create_issue()` with `IssueType::Epic` works | Validated | Phase 2 (IssueRepo tests), Phase 5 (`znt issue create --type epic`) | `zen-db/src/repos/issue.rs` |
| Update description | `update_issue()` with `IssueUpdate { description }` works | Validated | Phase 2 (IssueRepo tests), Phase 5 (`znt issue update`) | `zen-db/src/repos/issue.rs` |
| Task-to-issue link | `create_task()` with `issue_id` FK links task to issue | Validated | Phase 2 (TaskRepo tests) | `zen-db/src/repos/task.rs` |
| Get tasks for issue | `get_tasks_for_issue(issue_id)` returns all linked tasks | Validated | Phase 2 (TaskRepo tests), Phase 5 (`znt issue get`) | `zen-db/src/repos/task.rs:324` |
| Entity linking | `create_link()` with `DependsOn` creates relationship | Validated | Phase 2 (LinkRepo tests), Phase 5 (`znt link`) | `zen-db/src/repos/link.rs` |
| Get outbound links | `get_links_from(EntityType, id)` returns linked entities | Validated | Phase 2 (LinkRepo tests) | `zen-db/src/repos/link.rs:157` |
| Status transitions | `transition_issue()` enforces Open→InProgress→Done | Validated | Phase 2, zen-core `IssueStatus::allowed_next_states()` | `zen-core/src/enums.rs:217` |
| Clap subcommands | Nested `znt prd <action>` with `#[command(subcommand)]` | Validated | spike 0.9, Phase 5 (24 command modules) | `zen-cli/src/cli/root_commands.rs` |
| Active session helper | `require_active_session_id()` returns error if no session | Validated | Phase 5 (used by all 24 command modules) | `zen-cli/src/commands/shared/session.rs` |
| Output formatting | `output()` handles json/table/raw for all `Serialize` types | Validated | Phase 5 (output module tests) | `zen-cli/src/output/mod.rs` |
| Client-side type filter | `list_issues(limit)` + `.retain()` filters by type | Validated | Phase 5 (`issue/list.rs` uses same pattern for status/type) | `zen-cli/src/commands/issue/list.rs` |

---

## 10. Post-Review Mismatch Log

**Review date**: 2026-02-18
**Reviewed against**: Actual source code in `zenith/crates/zen-db/`, `zenith/crates/zen-cli/`, `zenith/crates/zen-core/`
**Total mismatches found**: 15

Categories: **A** = API signature mismatch, **B** = non-existent method/type, **E** = structural/convention, **F** = incorrect assumption, **H** = missing param/field mismatch

---

### A1. `IssueUpdateBuilder.description()` Signature Mismatch — **A** (API Signature)

**Plan says** (§A5 update.rs, line 395):
```rust
IssueUpdateBuilder::new().description(content.to_string()).build()
```

**Actual** (`zen-db/src/updates/issue.rs:37`):
```rust
pub fn description(mut self, description: Option<String>) -> Self {
    self.0.description = Some(description);
    self
}
```

**Problem**: `description()` takes `Option<String>`, not `String`. The `IssueUpdate.description` field is `Option<Option<String>>` (outer Option = "was this field updated?", inner Option = "set to value or NULL"). Passing `content.to_string()` directly won't compile.

**Resolution**: Change to `.description(Some(content.to_string()))`.

---

### A2. `list_sessions()` Signature in `require_active_session_id()` — Documentation Accuracy — **F** (Incorrect Assumption)

**Plan says** (§3 Upstream Dependencies table, line 145):
> `require_active_session_id()` — Shared helper — zen-cli — **DONE**

**Actual** (`zen-cli/src/commands/shared/session.rs:6`):
```rust
pub async fn require_active_session_id(ctx: &AppContext) -> anyhow::Result<String>
```

**Verdict**: ✅ **Correct**. The plan's usage is accurate. The function takes `&AppContext` (not `&mut`), which aligns with the PRD handlers. No mismatch.

---

### E1. Dispatch Function Signature: `&mut AppContext` vs Plan's `&AppContext` — **E** (Convention)

**Plan says** (§A3 prd.rs dispatch, §7.7):
> PRD handlers should accept `&AppContext` in their individual function signatures

**Actual** (`zen-cli/src/commands/dispatch.rs:9`):
```rust
pub async fn dispatch(command: Commands, ctx: &mut AppContext, flags: &GlobalFlags) -> anyhow::Result<()>
```

**All existing handlers** (issue.rs, task.rs, etc.) take `ctx: &mut AppContext` in their `handle()` function:
```rust
pub async fn handle(action: &IssueCommands, ctx: &mut AppContext, flags: &GlobalFlags) -> anyhow::Result<()>
```

**Problem**: The plan's `prd.rs` handler dispatch (§A3) shows `ctx: &mut AppContext`, which is correct. But this is inconsistent with §7.7 which says handlers should accept `&AppContext`. Actually, looking at the inner `run()` functions (e.g., `issue/get.rs:15`, `issue/create.rs:16`), they take `ctx: &AppContext` (immutable borrow). The `handle()` wrapper takes `&mut AppContext` to match dispatch, and Rust auto-reborrows.

**Resolution**: The plan's code is **correct** as written — `handle()` takes `&mut AppContext`, inner `run()` functions take `&AppContext`. §7.7 is also correct. No code change needed, but §7.7 wording could clarify this is about `run()` signatures, not `handle()`.

---

### E2. Missing `study.rs` / `finding.rs` / `hypothesis.rs` Command Modules in Plan — **E** (Convention)

**Observation**: Some existing command modules (e.g., `finding.rs`, `hypothesis.rs`, `study.rs`) are referenced indirectly by the plan's `prd/get.rs` via `ctx.service.get_finding()` and `ctx.service.get_hypothesis()`. These are upstream ZenService methods, NOT new code — the plan correctly calls them. No mismatch.

---

### F1. `IssueUpdate { description: Some(content) }` Direct Construction — **F** (Incorrect Assumption)

**Plan says** (§3 Data Flow, line 157):
```
znt prd update <iss-xxx> --content "<PRD markdown>"
  → update_issue(session_id, iss-xxx, IssueUpdate { description: Some(content) })
```

**Actual**: `IssueUpdate.description` is `Option<Option<String>>`. The data flow annotation should be:
```
→ update_issue(session_id, iss-xxx, IssueUpdate { description: Some(Some(content)), ..Default::default() })
```

**Resolution**: The data flow comment is imprecise but the actual code in §A5 uses `IssueUpdateBuilder`, which is the correct approach. Only the prose description is misleading. Update data flow to:
```
→ IssueUpdateBuilder::new().description(Some(content)).build()
```

---

### F2. `TaskStatus::Blocked` Not Counted in Plan's Data Flow — **F** (Incorrect Assumption)

**Plan says** (§3 Data Flow, line 170):
```
→ get_tasks_for_issue(iss-xxx) → all tasks → aggregate by status
```

**Actual code** (§A6 get.rs, lines 440-443) correctly counts `Done`, `InProgress`, `Blocked`, and `Open`. The `TaskStatus` enum has 4 variants: `Open`, `InProgress`, `Done`, `Blocked`. The plan's `PrdDetailResponse` struct includes all 4 counts.

**Verdict**: ✅ **Correct in code, imprecise in prose**. No code change needed.

---

### E3. `prd.rs` Module Structure: File vs Directory Conflict — **E** (Convention)

**Plan says** (§3 Module Structure):
```
├── commands/
│   ├── prd.rs              # NEW — handler dispatch
│   ├── prd/
│   │   ├── create.rs       # NEW
│   │   └── ...
```

**Actual convention** (existing code): `commands/issue.rs` coexists with `commands/issue/` directory. The `issue.rs` file uses `#[path = "issue/create.rs"] mod create;` to reference sub-files. This is the established pattern.

**Verdict**: ✅ **Correct**. The plan follows the exact same pattern (§A3 shows `#[path = "prd/create.rs"]`). No mismatch.

---

### F3. `IssueRepo` Referenced as Crate Abstraction — **F** (Incorrect Assumption)

**Plan says** (§ Scope block, line 8):
> All underlying entity operations already exist in upstream crates (issue CRUD via `IssueRepo`, task CRUD via `TaskRepo`, entity linking via `LinkRepo`, audit via `AuditRepo`)

**Actual**: There are no `IssueRepo`, `TaskRepo`, `LinkRepo`, or `AuditRepo` structs. All methods are implemented directly on `ZenService` via `impl ZenService` blocks in `repos/issue.rs`, `repos/task.rs`, `repos/link.rs`, `repos/audit.rs`. The "Repo" suffix is a conceptual grouping, not a type name.

**Resolution**: The plan should say "issue CRUD via `ZenService` (repos/issue.rs)", etc. This is a naming precision issue that won't affect implementation but could confuse implementers searching for `IssueRepo` type.

---

### A3. `transition_issue()` Returns `Result<Issue, DatabaseError>` Not `Issue` — **A** (API Signature)

**Plan says** (§B3 complete.rs, line 731-734):
```rust
let completed = ctx.service.transition_issue(&session_id, id, IssueStatus::Done).await?;
output(&completed, flags.format)
```

**Actual** (`zen-db/src/repos/issue.rs:272-270`):
```rust
pub async fn transition_issue(&self, session_id: &str, issue_id: &str, new_status: IssueStatus) -> Result<Issue, DatabaseError>
```

**Verdict**: ✅ **Correct**. The `?` unwraps the `Result`, giving `Issue`. The plan code is accurate.

---

### F4. Two-Step Transition Assumption in `complete.rs` — **F** (Incorrect Assumption)

**Plan says** (§B3 complete.rs, lines 718-728, §7.2):
> If still open, transition to in_progress first, then to done.

**Actual state machine** (`zen-core/src/enums.rs:217-223`):
```rust
Self::Open => &[Self::InProgress],
Self::InProgress => &[Self::Done, Self::Blocked, Self::Abandoned],
```

`transition_issue()` validates via `can_transition_to()` and returns `DatabaseError::InvalidState` on violation. The plan's two-step transition (Open→InProgress→Done) **is required** because `Open` cannot transition directly to `Done`.

**Verdict**: ✅ **Correct**. The plan correctly identifies this requirement and handles it in `complete.rs`. But there's a subtle issue: if the epic is `Blocked`, the plan doesn't handle it. `Blocked` → `InProgress` → `Done` would be needed.

**Resolution**: Add a `Blocked` case to `complete.rs`:
```rust
if issue.status == IssueStatus::Open || issue.status == IssueStatus::Blocked {
    ctx.service.transition_issue(&session_id, id, IssueStatus::InProgress).await?;
}
```

---

### H1. `PrdCommands` Missing `#[arg(value_parser)]` for Priority — **H** (Missing Param)

**Plan says** (§A1 PrdCommands, line 198-252): `PrdCommands::Create` only has `title` and `description`. No `--priority` flag.

**Actual convention** (`cli/subcommands/issue.rs:12`): `IssueCommands::Create` has `#[arg(long, value_parser = value_parser!(u8).range(1..=5))] priority: Option<u8>`.

**Verdict**: The plan intentionally omits priority (defaults to 3 in create.rs). This is a **design decision**, not a mismatch — PRDs always start at priority 3. Acceptable for MVP, but noted: if users want different priorities on PRDs, they'll need `znt issue update --priority`.

---

### E4. `PrdCommands` Missing `#[arg(long)]` on `List.limit` — **E** (Convention)

**Plan says** (§A1, line 250):
```rust
List {
    #[arg(long)]
    limit: Option<u32>,
}
```

**Verdict**: ✅ **Correct**. Matches convention from `TaskCommands::List` (`cli/subcommands/task.rs:41`).

---

### E5. `prd/list.rs` Always Over-Fetches — **E** (Convention)

**Plan says** (§A7 list.rs, line 517):
```rust
let fetch_limit = limit.saturating_mul(5).min(500);
```

**Actual convention** (`commands/issue/list.rs:40-46`):
```rust
fn compute_fetch_limit(limit: u32, status: Option<&str>, issue_type: Option<&str>) -> u32 {
    if status.is_some() || issue_type.is_some() {
        limit.saturating_mul(5).min(500)
    } else { limit }
}
```

**Problem**: The existing issue/list.rs only over-fetches when filters are present. The plan's prd/list.rs **always** over-fetches (5x) because it always filters by `type == Epic`. This is correct behavior for PRD list since epic filtering is always applied, but differs from the issue/list.rs pattern description in §7.3 which says "same pattern used in `issue/list.rs`".

**Resolution**: The plan's approach is **correct** (always over-fetch because epic filtering is always active), but §7.3 should clarify this is an adaptation, not an exact copy of the issue/list.rs pattern.

---

### F5. `task complete` Uses `update_task()`, Not `transition_task()` — **F** (Incorrect Assumption)

**Plan says** (§B4 Tests, line 374):
> `znt task complete <task-id>` (use existing command)

**Actual** (`commands/task/complete.rs:9-13`):
```rust
let update = TaskUpdateBuilder::new().status(TaskStatus::Done).build();
let task = ctx.service.update_task(&session_id, id, update).await?;
```

**Problem**: The existing `task complete` uses `update_task()` with `TaskUpdateBuilder`, NOT `transition_task()`. This means it bypasses the state machine validation — it directly sets `status = done` regardless of current status. This is a pre-existing design choice in zen-cli.

**Impact on PRD**: The plan's integration test (step 7: `znt task complete <task-id>`) will work as written. However, the plan's `prd complete` command correctly uses `transition_issue()` which DOES enforce the state machine. This asymmetry (tasks skip validation, issues enforce it) is worth noting.

**Resolution**: No change needed for Phase 6, but document this asymmetry. The `task complete` shortcut bypasses `transition_task()` intentionally.

---

### F6. Plan References `IssueDetailResponse { issue, children, tasks }` from Phase 5 — **F** (Documentation Accuracy)

**Plan says** (§2.3, line 79):
> Phase 5 (`znt issue get` already returns `IssueDetailResponse { issue, children, tasks }`)

**Actual** (`commands/issue/get.rs:9-13`):
```rust
struct IssueDetailResponse {
    issue: Issue,
    children: Vec<Issue>,
    tasks: Vec<Task>,
}
```

**Verdict**: ✅ **Correct**. The plan accurately describes the existing response struct.

---

### E6. Finding Import Path in `prd/get.rs` — **E** (Convention)

**Plan says** (§A6 get.rs, line 406):
```rust
use zen_core::entities::{Finding, Hypothesis, Issue, Task};
```

**Actual** (`zen-core/src/entities/`): Entities are in separate files under `zen-core/src/entities/` directory: `finding.rs`, `hypothesis.rs`, `issue.rs`, `task.rs`. But the module's `mod.rs` re-exports them, so the import path `zen_core::entities::{Finding, Hypothesis, Issue, Task}` is valid if `mod.rs` uses `pub use`.

**Verification needed at implementation time**: Confirm `zen-core/src/entities/mod.rs` has `pub use finding::Finding; pub use hypothesis::Hypothesis;` etc.

**Resolution**: Likely correct, but verify re-exports during implementation.

---

### A4. `prd/tasks.rs` Missing Epic Verification Against `get_issue()` Return Type — **A** (API)

**Plan says** (§B1 tasks.rs, line 601-603):
```rust
let issue = ctx.service.get_issue(epic_id).await?;
if issue.issue_type != IssueType::Epic {
    anyhow::bail!("...");
}
```

**Actual** (`zen-db/src/repos/issue.rs:107`):
```rust
pub async fn get_issue(&self, id: &str) -> Result<Issue, DatabaseError>
```

**Verdict**: ✅ **Correct**. `get_issue()` returns `Result<Issue, DatabaseError>`, `Issue` has `issue_type: IssueType` field. The comparison works.

---

### F7. `prd subtasks` Does Not Validate Epic ID — **F** (Incorrect Assumption)

**Plan says** (§B2 subtasks.rs): Validates parent task exists via `get_task(parent_task_id)`, but does **not** verify that the `--epic` argument refers to an existing epic issue.

**Problem**: If the user passes a garbage `--epic` value, `create_task(..., issue_id=Some(epic_id), ...)` will either:
- Fail with a DB FK constraint error (if enforced), or
- Silently create orphaned tasks that never appear in `get_tasks_for_issue()` results.

**Resolution**: Add epic validation at the start of `subtasks.rs`:
```rust
let epic = ctx.service.get_issue(epic_id).await?;
if epic.issue_type != IssueType::Epic {
    anyhow::bail!("Issue '{epic_id}' is not an epic.");
}
```

---

### F8. `prd complete` Does Not Handle Terminal States (`Done`, `Abandoned`) — **F** (Incorrect Assumption)

**Plan says** (§B3 complete.rs): Handles `Open` and `Blocked` by transitioning to `InProgress` first, then calls `transition_issue(..., Done)`.

**Problem**: If the epic is already `Done`, calling `transition_issue(..., Done)` will return `DatabaseError::InvalidState` since `Done` has no allowed next states. If `Abandoned`, same issue. The user gets an opaque error instead of a clear message.

**Resolution**: Add guards for terminal states:
```rust
if issue.status == IssueStatus::Done {
    // Already done — return the issue as-is
    return output(&issue, flags.format);
}
if issue.status == IssueStatus::Abandoned {
    anyhow::bail!("PRD '{id}' is abandoned and cannot be completed. Use 'znt issue update --status in_progress' to reopen it first.");
}
```

---

### E7. `Relation::DependsOn` and Entity Re-Exports — Confirmed ✅ — **E** (Convention)

**Oracle flagged**: `Relation::DependsOn` might not exist; `Finding`/`Hypothesis` might not be re-exported from `zen_core::entities`.

**Verified against source**:
- `Relation::DependsOn` exists at `zen-core/src/enums.rs:576` ✅
- `zen_core::entities::Finding` re-exported at `entities/mod.rs:23` ✅
- `zen_core::entities::Hypothesis` re-exported at `entities/mod.rs:24` ✅

No mismatch. All import paths in the plan are valid.

---

### Summary

| # | Category | Description | Severity | Action |
|---|----------|-------------|----------|--------|
| A1 | A | `IssueUpdateBuilder.description()` takes `Option<String>`, plan passes `String` | **High** | ✅ Fixed: `.description(Some(content.to_string()))` |
| F4 | F | `complete.rs` doesn't handle `Blocked` status | **Medium** | ✅ Fixed: Added `Blocked` guard |
| F7 | F | `prd subtasks` doesn't validate `--epic` is an existing epic | **High** | Add epic validation |
| F8 | F | `prd complete` doesn't handle terminal states (`Done`, `Abandoned`) | **Medium** | Add terminal-state guards |
| F1 | F | Data flow prose describes `IssueUpdate` construction imprecisely | Low | Clarify prose |
| F3 | F | Plan uses "IssueRepo/TaskRepo" names — actual code is `impl ZenService` | Low | Clarify naming |
| F5 | F | `task complete` uses `update_task()` not `transition_task()` — asymmetry | Low | Document asymmetry |
| E1 | E | §7.7 slightly ambiguous about `handle()` vs `run()` signatures | Low | Clarify |
| E5 | E | §7.3 says "same pattern as issue/list.rs" but prd/list.rs always over-fetches | Low | Clarify |
| E6 | E | Entity re-export path — verified ✅ | None | No action |
| E7 | E | `Relation::DependsOn` + entity re-exports — verified ✅ | None | No action |

**High severity items requiring code change**: 3 (A1 ✅ fixed, F4 ✅ fixed, F7 needs fix in §B2)
**Medium severity items**: 1 (F8 needs fix in §B3)
**Low/None items**: 7

**Overall assessment**: The plan is **high quality** with 4 code-level issues total. Two were fixed inline (A1, F4). Two remain for implementation: F7 (epic validation in subtasks.rs) and F8 (terminal-state handling in complete.rs). All upstream API signatures, method names, type references, enum variants, and import paths are verified correct against actual source. Module structure and conventions match existing Phase 5 patterns exactly.

---

## Cross-References

- PRD workflow design: [06-prd-workflow.md](./06-prd-workflow.md)
- CLI API design (PRD section): [04-cli-api-design.md](./04-cli-api-design.md) §prd commands
- Implementation plan (Phase 6 tasks): [07-implementation-plan.md](./07-implementation-plan.md) §8
- Phase 5 plan (patterns, conventions): [25-phase5-cli-shell-plan.md](./25-phase5-cli-shell-plan.md)
- Data model (issues/tasks tables): [01-turso-data-model.md](./01-turso-data-model.md)
- Crate designs (zen-cli §11): [05-crate-designs.md](./05-crate-designs.md)
