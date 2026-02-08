# Zenith: CLI API Design

**Version**: 2026-02-07
**Status**: Design Document
**Purpose**: Complete CLI command reference, input/output formats, and usage patterns

---

## Table of Contents

1. [Overview](#1-overview)
2. [Global Flags](#2-global-flags)
3. [Project Management](#3-project-management)
4. [Session Management](#4-session-management)
5. [Package Indexing](#5-package-indexing)
6. [Search](#6-search)
7. [Research](#7-research)
8. [Findings](#8-findings)
9. [Hypotheses](#9-hypotheses)
10. [Insights](#10-insights)
11. [Tasks](#11-tasks)
12. [Implementation Log](#12-implementation-log)
13. [Compatibility](#13-compatibility)
14. [Entity Links](#14-entity-links)
15. [Audit Trail](#15-audit-trail)
16. [Workflow Commands](#16-workflow-commands)
17. [Output Formats](#17-output-formats)

---

## 1. Overview

All commands follow the pattern:

```
zen <domain> <action> [args] [flags]
```

All commands return JSON to stdout by default. Human-readable table format available via `--format table`.

### Command Map

```
zen
├── init                          # Initialize zenith for a project
├── onboard                       # Onboard existing project (detect + index deps)
├── session
│   ├── start                     # Start a new work session
│   ├── end                       # End session without full wrap-up
│   └── list                      # List sessions
├── install <package>             # Clone, parse, index a package
├── search <query>                # Search indexed documentation
├── research
│   ├── create                    # Create a research item
│   ├── update <id>               # Update a research item
│   ├── list                      # List research items
│   ├── get <id>                  # Get research item details
│   └── registry <query>          # Query package registries
├── finding
│   ├── create                    # Create a finding
│   ├── update <id>               # Update a finding
│   ├── list                      # List findings
│   ├── get <id>                  # Get finding details
│   ├── tag <id> <tag>            # Add tag
│   └── untag <id> <tag>          # Remove tag
├── hypothesis
│   ├── create                    # Create a hypothesis
│   ├── update <id>               # Update status/content
│   ├── list                      # List hypotheses
│   └── get <id>                  # Get hypothesis details
├── insight
│   ├── create                    # Create an insight
│   ├── update <id>               # Update an insight
│   ├── list                      # List insights
│   └── get <id>                  # Get insight details
├── issue
│   ├── create                    # Create an issue (bug, feature, spike, epic, request)
│   ├── update <id>               # Update an issue
│   ├── list                      # List issues
│   └── get <id>                  # Get issue with child issues and linked tasks
├── task
│   ├── create                    # Create a task
│   ├── update <id>               # Update a task
│   ├── list                      # List tasks
│   ├── get <id>                  # Get task details
│   └── complete <id>             # Mark task as done
├── log <file#lines> [--task id]  # Record implementation
├── compat
│   ├── check <pkg-a> <pkg-b>    # Create/update compatibility check
│   ├── list                      # List compatibility checks
│   └── get <id>                  # Get check details
├── link <src> <target> <rel>     # Create entity link
├── unlink <link-id>              # Remove entity link
├── prd
│   ├── create                    # Create a PRD (epic issue)
│   ├── update <id>               # Update PRD content
│   ├── get <id>                  # Full PRD with tasks and progress
│   ├── tasks <id>                # Generate parent tasks from PRD
│   ├── subtasks <parent-task>    # Generate sub-tasks for a parent
│   ├── complete <id>             # Mark PRD as done
│   └── list                      # List all PRDs
├── audit                         # View audit trail
├── whats-next                    # Project state + next steps
└── wrap-up                       # End session, sync, summarize
```

---

## 2. Global Flags

| Flag | Short | Description | Default |
|------|-------|-------------|---------|
| `--format` | `-f` | Output format: `json`, `table`, `raw` | `json` |
| `--limit` | `-l` | Max results to return | varies by command |
| `--quiet` | `-q` | Suppress non-essential output | `false` |
| `--verbose` | `-v` | Verbose output (debug info) | `false` |
| `--project` | `-p` | Path to project root (if not cwd) | `.` |

---

## 3. Project Management

### `zen init`

Initialize Zenith for a new or existing project.

```bash
zen init [--name <name>] [--ecosystem <ecosystem>]
```

**Flags:**

| Flag | Description | Default |
|------|-------------|---------|
| `--name` | Project name | Auto-detected from manifest |
| `--ecosystem` | Primary ecosystem | Auto-detected |
| `--no-index` | Skip dependency indexing | `false` |

**Output:**

```json
{
    "project": {
        "name": "my-app",
        "ecosystem": "rust",
        "language": "rust",
        "root_path": "/home/user/projects/my-app",
        "vcs": "git"
    },
    "dependencies": {
        "total": 42,
        "indexed": 0
    },
    "session": {
        "id": "ses-a3f8b2",
        "status": "active"
    }
}
```

### `zen onboard`

Onboard an existing project that doesn't have Zenith yet. Detects project type, parses manifest, indexes all dependencies.

```bash
zen onboard [--workspace] [--root <path>] [--skip-indexing]
```

**Flags:**

| Flag | Description | Default |
|------|-------------|---------|
| `--workspace` | Treat as monorepo workspace | Auto-detected |
| `--root` | Project root path | `.` |
| `--skip-indexing` | Register deps but don't index | `false` |
| `--ecosystem` | Override ecosystem detection | Auto-detected |

**Output:**

```json
{
    "project": {
        "name": "my-app",
        "ecosystem": "rust",
        "manifests_found": ["Cargo.toml"]
    },
    "dependencies": {
        "detected": 42,
        "already_indexed": 12,
        "newly_indexed": 30,
        "failed": 0
    },
    "session": {
        "id": "ses-b2c4d1",
        "status": "active"
    }
}
```

---

## 4. Session Management

### `zen session start`

Start a new work session. Detects and cleans up orphaned active sessions.

```bash
zen session start
```

**Output:**

```json
{
    "session": {
        "id": "ses-c4e2d1",
        "status": "active",
        "started_at": "2026-02-07T12:00:00Z"
    },
    "previous_session": {
        "id": "ses-a3f8b2",
        "status": "wrapped_up",
        "summary": "Researched HTTP client libraries. Confirmed reqwest compatibility with tower."
    }
}
```

### `zen session end`

End the current session without a full wrap-up (no sync, no summary).

```bash
zen session end [--abandon]
```

**Flags:**

| Flag | Description |
|------|-------------|
| `--abandon` | Mark as abandoned instead of wrapped_up |

### `zen session list`

List all sessions.

```bash
zen session list [--limit 10] [--status active|wrapped_up|abandoned]
```

**Output:**

```json
{
    "sessions": [
        {
            "id": "ses-c4e2d1",
            "status": "active",
            "started_at": "2026-02-07T12:00:00Z",
            "ended_at": null
        },
        {
            "id": "ses-a3f8b2",
            "status": "wrapped_up",
            "started_at": "2026-02-06T10:00:00Z",
            "ended_at": "2026-02-06T18:00:00Z",
            "summary": "..."
        }
    ]
}
```

---

## 5. Package Indexing

### `zen install`

Clone a package repository, parse with tree-sitter, generate embeddings, store in DuckLake.

```bash
zen install <package> [--ecosystem <eco>] [--version <ver>]
```

**Args:**

| Arg | Description |
|-----|-------------|
| `<package>` | Package name (e.g., `tokio`, `zod`, `phoenix`) |

**Flags:**

| Flag | Description | Default |
|------|-------------|---------|
| `--ecosystem` | Package ecosystem | Inferred from project |
| `--version` | Specific version to index | Latest |
| `--include-tests` | Include test files in indexing | `false` |
| `--force` | Re-index even if already indexed | `false` |

**Output:**

```json
{
    "package": {
        "ecosystem": "rust",
        "name": "tokio",
        "version": "1.40.0",
        "repo_url": "https://github.com/tokio-rs/tokio"
    },
    "indexing": {
        "files_parsed": 87,
        "symbols_extracted": 342,
        "doc_chunks_created": 56,
        "embeddings_generated": 398,
        "duration_ms": 4520
    }
}
```

---

## 6. Search

### `zen search`

Search indexed package documentation. Supports filtering by package, ecosystem, symbol kind, and result limit.

```bash
zen search <query> [flags]
```

**Args:**

| Arg | Description |
|-----|-------------|
| `<query>` | Search query (matched against symbol names, signatures, and doc comments) |

**Flags:**

| Flag | Description | Default |
|------|-------------|---------|
| `--package` | Scope to a specific package | All indexed |
| `--ecosystem` | Scope to ecosystem | All |
| `--kind` | Filter by symbol kind (function, struct, trait, etc.) | All |
| `--limit` | Max results | `20` |
| `--context-budget` | Max characters of content to return | `8000` |
| `--mode` | Search mode: `vector`, `fts`, `hybrid` | `hybrid` |

**Output:**

```json
{
    "results": [
        {
            "package": "tokio",
            "ecosystem": "rust",
            "version": "1.40.0",
            "kind": "function",
            "name": "spawn",
            "signature": "pub fn spawn<F>(future: F) -> JoinHandle<F::Output> where F: Future + Send + 'static",
            "doc_comment": "Spawns a new asynchronous task, returning a JoinHandle for it.",
            "file_path": "tokio/src/task/spawn.rs",
            "line_start": 120,
            "line_end": 145,
            "score": 0.94
        }
    ],
    "query": "spawn async task",
    "total_results": 15,
    "returned": 10,
    "search_mode": "hybrid"
}
```

---

## 7. Research

### `zen research create`

Create a new research item.

```bash
zen research create --title <title> [--description <desc>]
```

**Output:**

```json
{
    "research": {
        "id": "res-c4e2d1",
        "title": "Evaluate Rust HTTP client libraries",
        "description": "Compare reqwest, hyper, and ureq for production async HTTP",
        "status": "open",
        "session_id": "ses-a3f8b2",
        "created_at": "2026-02-07T12:00:00Z"
    }
}
```

### `zen research update <id>`

```bash
zen research update <id> [--title <title>] [--description <desc>] [--status <status>]
```

### `zen research list`

```bash
zen research list [--status open|in_progress|resolved|abandoned] [--limit 20]
```

### `zen research get <id>`

Returns research item with linked findings, hypotheses, tasks, and insights.

```bash
zen research get <id>
```

**Output:**

```json
{
    "research": {
        "id": "res-c4e2d1",
        "title": "Evaluate Rust HTTP client libraries",
        "status": "in_progress"
    },
    "findings": [
        {"id": "fnd-b7a3f9", "content": "reqwest supports connection pooling out of the box", "confidence": "high"}
    ],
    "hypotheses": [
        {"id": "hyp-e1c4b2", "content": "reqwest works with tower middleware", "status": "analyzing"}
    ],
    "tasks": [
        {"id": "tsk-f3b7c1", "title": "Test reqwest + tower integration", "status": "open"}
    ],
    "insights": [],
    "links": [
        {"source": "fnd-b7a3f9", "target": "hyp-e1c4b2", "relation": "triggers"}
    ]
}
```

### `zen research registry <query>`

Query package registries directly (crates.io, npm, hex, pypi). Does not create any state -- pure lookup.

```bash
zen research registry <query> [--ecosystem rust|npm|hex|pypi|all] [--limit 10]
```

**Output:**

```json
{
    "results": [
        {
            "name": "reqwest",
            "version": "0.12.9",
            "ecosystem": "rust",
            "description": "An ergonomic, batteries-included HTTP Client for Rust",
            "downloads": 98234567,
            "license": "MIT/Apache-2.0",
            "repository": "https://github.com/seanmonstar/reqwest"
        }
    ],
    "query": "http client",
    "ecosystem": "rust"
}
```

---

## 8. Findings

### `zen finding create`

```bash
zen finding create --content <content> [--research <id>] [--source <src>] [--confidence high|medium|low] [--tag <tag>...]
```

**Output:**

```json
{
    "finding": {
        "id": "fnd-b7a3f9",
        "content": "reqwest 0.12 does not support HTTP/3 yet",
        "research_id": "res-c4e2d1",
        "source": "https://github.com/seanmonstar/reqwest/issues/1653",
        "confidence": "high",
        "tags": ["needs-verification", "feature-gap"],
        "session_id": "ses-a3f8b2"
    }
}
```

### `zen finding update <id>`

```bash
zen finding update <id> [--content <content>] [--confidence <conf>] [--source <src>]
```

### `zen finding list`

```bash
zen finding list [--research <id>] [--tag <tag>] [--confidence high|medium|low] [--limit 20] [--search <fts-query>]
```

The `--search` flag uses FTS5 full-text search on the content and source fields.

### `zen finding get <id>`

Returns finding with tags, linked entities, and related audit entries.

### `zen finding tag <id> <tag>`

Add a tag to a finding.

### `zen finding untag <id> <tag>`

Remove a tag from a finding.

---

## 9. Hypotheses

### `zen hypothesis create`

```bash
zen hypothesis create --content <content> [--research <id>] [--finding <id>]
```

**Output:**

```json
{
    "hypothesis": {
        "id": "hyp-e1c4b2",
        "content": "reqwest tower middleware integration works via reqwest::Client::builder().layer()",
        "status": "unverified",
        "research_id": "res-c4e2d1",
        "finding_id": null,
        "session_id": "ses-a3f8b2"
    }
}
```

### `zen hypothesis update <id>`

```bash
zen hypothesis update <id> [--status unverified|analyzing|confirmed|debunked|partially_confirmed|inconclusive] [--reason <reason>] [--content <content>]
```

When status changes, `reason` should explain why:

```bash
zen hypothesis update hyp-e1c4b2 --status confirmed \
    --reason "Tested in spike. reqwest::ClientBuilder supports tower::Layer via .layer() method."
```

### `zen hypothesis list`

```bash
zen hypothesis list [--status <status>] [--research <id>] [--limit 20]
```

### `zen hypothesis get <id>`

Returns hypothesis with linked research, findings, and related entities.

---

## 10. Insights

### `zen insight create`

```bash
zen insight create --content <content> [--research <id>] [--confidence high|medium|low]
```

### `zen insight update <id>`

```bash
zen insight update <id> [--content <content>] [--confidence <conf>]
```

### `zen insight list`

```bash
zen insight list [--research <id>] [--limit 20]
```

### `zen insight get <id>`

---

## 11. Tasks

### `zen task create`

```bash
zen task create --title <title> [--description <desc>] [--research <id>]
```

### `zen task update <id>`

```bash
zen task update <id> [--title <title>] [--description <desc>] [--status open|in_progress|done|blocked]
```

### `zen task list`

```bash
zen task list [--status <status>] [--research <id>] [--limit 20]
```

### `zen task get <id>`

Returns task with linked research, findings, implementation log entries, and blocking/blocked-by relationships.

### `zen task complete <id>`

Shorthand for `zen task update <id> --status done`.

---

## 12. Implementation Log

### `zen log`

Record an implementation location.

```bash
zen log <file#lines> [--task <id>] [--description <desc>]
```

**Args:**

| Arg | Format | Example |
|-----|--------|---------|
| `<file#lines>` | `path#start-end` or `path#line` | `src/http/client.rs#45-82` |

**Output:**

```json
{
    "implementation": {
        "id": "imp-a8d3e2",
        "task_id": "tsk-f3b7c1",
        "file_path": "src/http/client.rs",
        "start_line": 45,
        "end_line": 82,
        "description": "Added exponential backoff retry with max 3 attempts",
        "session_id": "ses-a3f8b2"
    }
}
```

---

## 13. Compatibility

### `zen compat check`

Create or update a compatibility check between two packages.

```bash
zen compat check <package-a> <package-b> [--status compatible|incompatible|conditional|unknown] [--conditions <text>] [--finding <id>]
```

**Args:**

| Arg | Format | Example |
|-----|--------|---------|
| `<package-a>` | `ecosystem:name:version` | `rust:tokio:1.40.0` |
| `<package-b>` | `ecosystem:name:version` | `rust:axum:0.8.0` |

**Output:**

```json
{
    "compatibility": {
        "id": "cmp-c1f4b7",
        "package_a": "rust:tokio:1.40.0",
        "package_b": "rust:axum:0.8.0",
        "status": "compatible",
        "conditions": null,
        "finding_id": "fnd-d2e5a8"
    }
}
```

### `zen compat list`

```bash
zen compat list [--package <name>] [--status <status>] [--limit 20]
```

### `zen compat get <id>`

---

## 14. Entity Links

### `zen link`

Create a relationship between any two entities.

```bash
zen link <source> <target> <relation>
```

**Args:**

| Arg | Format | Example |
|-----|--------|---------|
| `<source>` | Entity ID (prefix determines type) | `fnd-b7a3f9` |
| `<target>` | Entity ID | `hyp-e1c4b2` |
| `<relation>` | Relation type | `validates` |

**Relations:** `blocks`, `validates`, `debunks`, `implements`, `relates-to`, `derived-from`, `triggers`, `supersedes`, `depends-on`

**Output:**

```json
{
    "link": {
        "id": "lnk-e5a2d9",
        "source_type": "finding",
        "source_id": "fnd-b7a3f9",
        "target_type": "hypothesis",
        "target_id": "hyp-e1c4b2",
        "relation": "validates"
    }
}
```

### `zen unlink <link-id>`

Remove an entity link.

---

## 15. Audit Trail

### `zen audit`

View the audit trail.

```bash
zen audit [--limit 20] [--entity-type <type>] [--entity-id <id>] [--action <action>] [--session <id>] [--search <fts-query>]
```

**Flags:**

| Flag | Description |
|------|-------------|
| `--limit` | Max entries to return (default: 20) |
| `--entity-type` | Filter by entity type |
| `--entity-id` | Filter by specific entity |
| `--action` | Filter by action (created, status_changed, etc.) |
| `--session` | Filter by session |
| `--search` | FTS5 search on action and detail fields |

**Output:**

```json
{
    "entries": [
        {
            "id": "aud-b3c8f1",
            "session_id": "ses-a3f8b2",
            "entity_type": "hypothesis",
            "entity_id": "hyp-e1c4b2",
            "action": "status_changed",
            "detail": {
                "from": "analyzing",
                "to": "confirmed",
                "reason": "Spike validated tower middleware compatibility"
            },
            "created_at": "2026-02-07T14:30:00Z"
        }
    ],
    "total": 156,
    "returned": 20
}
```

---

## 16. Workflow Commands

### `zen whats-next`

Returns current project state and suggested next actions. Reads the last session snapshot, open tasks, pending hypotheses, and recent audit entries.

```bash
zen whats-next [--limit 10] [--format json|table|raw]
```

**Output (JSON):**

```json
{
    "last_session": {
        "id": "ses-a3f8b2",
        "status": "wrapped_up",
        "ended_at": "2026-02-06T18:00:00Z",
        "summary": "Researched HTTP client libraries. Confirmed reqwest compatibility."
    },
    "snapshot": {
        "open_tasks": 3,
        "in_progress_tasks": 1,
        "pending_hypotheses": 2,
        "unverified_hypotheses": 1,
        "recent_findings": 5,
        "open_research": 1
    },
    "open_tasks": [
        {"id": "tsk-f3b7c1", "title": "Test reqwest + tower integration", "status": "in_progress"},
        {"id": "tsk-a2b3c4", "title": "Benchmark reqwest vs hyper", "status": "open"},
        {"id": "tsk-d4e5f6", "title": "Write error handling middleware", "status": "open"}
    ],
    "pending_hypotheses": [
        {"id": "hyp-g7h8i9", "content": "hyper is faster than reqwest for streaming", "status": "unverified"},
        {"id": "hyp-j1k2l3", "content": "tower-http provides built-in retry", "status": "analyzing"}
    ],
    "recent_audit": [
        {"action": "status_changed", "entity_type": "hypothesis", "entity_id": "hyp-e1c4b2", "detail": {"to": "confirmed"}, "created_at": "2026-02-06T17:45:00Z"},
        {"action": "created", "entity_type": "finding", "entity_id": "fnd-m4n5o6", "created_at": "2026-02-06T17:30:00Z"}
    ]
}
```

**Output (raw):**

When `--format raw`, returns the last N audit trail entries as-is (for the LLM to reason over directly).

### `zen wrap-up`

End the current session, generate summary, sync to cloud.

```bash
zen wrap-up [--auto-commit] [--message <msg>]
```

**Flags:**

| Flag | Description | Default |
|------|-------------|---------|
| `--auto-commit` | Git commit after sync | From config |
| `--message` | Commit message | Auto-generated |

**Output:**

```json
{
    "session": {
        "id": "ses-a3f8b2",
        "status": "wrapped_up",
        "summary": "Completed research on HTTP client libraries. Confirmed reqwest + tower compatibility. 3 tasks remain open."
    },
    "snapshot": {
        "open_tasks": 3,
        "pending_hypotheses": 1,
        "recent_findings": 7
    },
    "sync": {
        "status": "success",
        "turso_synced": true,
        "audit_exported": true,
        "git_committed": true,
        "commit_hash": "a3f8b2c"
    }
}
```

---

## 17. Output Formats

All commands support three output formats via `--format`:

### `json` (default)

Structured JSON. Best for LLM consumption.

### `table`

Human-readable aligned table. Best for terminal display.

```
ID          STATUS      TITLE
tsk-f3b7c1  in_progress Test reqwest + tower integration
tsk-a2b3c4  open        Benchmark reqwest vs hyper
tsk-d4e5f6  open        Write error handling middleware
```

### `raw`

Minimal output. For `audit`, returns newline-delimited JSON (one entry per line). For `whats-next`, returns the raw audit entries without summary.

---

## Cross-References

- Turso data model: [01-turso-data-model.md](./01-turso-data-model.md)
- DuckLake data model: [02-ducklake-data-model.md](./02-ducklake-data-model.md)
- Architecture overview: [03-architecture-overview.md](./03-architecture-overview.md)
