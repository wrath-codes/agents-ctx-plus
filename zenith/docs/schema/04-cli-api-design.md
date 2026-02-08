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
16. [Studies (Structured Learning)](#16-studies-structured-learning)
17. [Database & Trail Commands](#17-database--trail-commands)
18. [Workflow Commands](#18-workflow-commands)
19. [Output Formats](#19-output-formats)

---

## 1. Overview

All commands follow the pattern:

```
znt <domain> <action> [args] [flags]
```

All commands return JSON to stdout by default. Human-readable table format available via `--format table`.

### Command Map

```
znt
├── init                          # Initialize zenith for a project
├── onboard                       # Onboard existing project (detect + index deps)
├── session
│   ├── start                     # Start a new work session
│   ├── end                       # End session without full wrap-up
│   └── list                      # List sessions
├── install <package>             # Clone, parse, index a package
├── search <query>                # Search indexed documentation
├── grep <pattern> [path...]      # Regex search (package source or local files)
├── cache
│   ├── list                      # Show cached packages + sizes
│   ├── clean [package]           # Remove cached source
│   └── stats                     # Total cache size
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
├── rebuild                        # Rebuild DB from JSONL trail files
├── schema <type>                  # Dump JSON Schema for a registered type
├── study
│   ├── create                    # Create a structured learning study
│   ├── assume <id>               # Add an assumption (hypothesis) to study
│   ├── test <id>                 # Record test result against assumption
│   ├── get <id>                  # Full study state with progress
│   ├── conclude <id>             # Conclude the study
│   └── list                      # List all studies
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

### `znt init`

Initialize Zenith for a new or existing project.

```bash
znt init [--name <name>] [--ecosystem <ecosystem>]
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

### `znt onboard`

Onboard an existing project that doesn't have Zenith yet. Detects project type, parses manifest, indexes all dependencies.

```bash
znt onboard [--workspace] [--root <path>] [--skip-indexing]
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

### `znt session start`

Start a new work session. Detects and cleans up orphaned active sessions.

```bash
znt session start
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

### `znt session end`

End the current session without a full wrap-up (no sync, no summary).

```bash
znt session end [--abandon]
```

**Flags:**

| Flag | Description |
|------|-------------|
| `--abandon` | Mark as abandoned instead of wrapped_up |

### `znt session list`

List all sessions.

```bash
znt session list [--limit 10] [--status active|wrapped_up|abandoned]
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

### `znt install`

Clone a package repository, parse with tree-sitter, generate embeddings, store in DuckLake.

```bash
znt install <package> [--ecosystem <eco>] [--version <ver>]
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

### `znt search`

Search indexed package documentation. Supports filtering by package, ecosystem, symbol kind, and result limit.

```bash
znt search <query> [flags]
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

### `znt grep`

Regex or literal text search over indexed package source or local project files. See [13-zen-grep-design.md](./13-zen-grep-design.md) for full design.

```bash
znt grep <pattern> [path...] [flags]
```

**Modes** (one required):

```bash
znt grep <pattern> --package <pkg>        # Search one indexed package
znt grep <pattern> --all-packages         # Search all indexed packages
znt grep <pattern> <path...>              # Search local project files
```

**Args:**

| Arg | Description |
|-----|-------------|
| `<pattern>` | Regex pattern (or literal with `-F`) |
| `[path...]` | Local paths to search (local mode only) |

**Flags:**

| Flag | Short | Description | Default |
|------|-------|-------------|---------|
| `--package` | `-P` | Search cached source for this package (repeatable) | (none) |
| `--ecosystem` | `-e` | Ecosystem filter (with `--package`) | auto-detect |
| `--all-packages` | | Search all indexed packages | `false` |
| `--fixed-strings` | `-F` | Treat pattern as literal, not regex | `false` |
| `--ignore-case` | `-i` | Case-insensitive matching | `false` |
| `--smart-case` | `-S` | Auto case-insensitive if all lowercase | `true` |
| `--word-regexp` | `-w` | Whole word matching | `false` |
| `--context` | `-C` | Lines of context around matches | `2` |
| `--include` | | File glob to include (e.g., `"*.rs"`) | (all) |
| `--exclude` | | File glob to exclude | (none) |
| `--max-count` | `-m` | Max matches per file | (none) |
| `--count` | `-c` | Only show match counts per file | `false` |
| `--files-with-matches` | `-l` | Only show filenames with matches | `false` |
| `--skip-tests` | | Skip test files/dirs | `false` |
| `--no-symbols` | | Skip symbol correlation (package mode) | `false` |

**Output:**

```json
{
    "matches": [
        {
            "path": "tokio/src/runtime/blocking/pool.rs",
            "line_number": 142,
            "text": "    pub(crate) fn spawn_blocking(&self, func: ...) {",
            "context_before": ["    /// Spawns a blocking task.", "    ///"],
            "context_after": ["        let (task, handle) = ..."],
            "symbol": {
                "id": "abc123",
                "kind": "function",
                "name": "spawn_blocking",
                "signature": "pub(crate) fn spawn_blocking<F, R>(&self, func: F) -> JoinHandle<R>"
            }
        }
    ],
    "stats": {
        "files_searched": 284,
        "files_matched": 12,
        "matches_found": 37,
        "matches_with_symbol": 28,
        "elapsed_ms": 45
    }
}
```

### `znt cache`

Manage cached source files stored in DuckDB for `znt grep` package mode.

```bash
znt cache list                    # Show cached packages + sizes
znt cache clean                   # Remove all cached source
znt cache clean <package>         # Remove one package's cached source
znt cache stats                   # Total cache size, package count
```

**Output** (`znt cache list`):

```json
{
    "packages": [
        {"ecosystem": "rust", "name": "tokio", "version": "1.40.0", "file_count": 87, "size_bytes": 4200000},
        {"ecosystem": "rust", "name": "serde", "version": "1.0.210", "file_count": 32, "size_bytes": 1100000}
    ],
    "total_size_bytes": 5300000,
    "total_packages": 2
}
```

---

## 7. Research

### `znt research create`

Create a new research item.

```bash
znt research create --title <title> [--description <desc>]
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

### `znt research update <id>`

```bash
znt research update <id> [--title <title>] [--description <desc>] [--status <status>]
```

### `znt research list`

```bash
znt research list [--status open|in_progress|resolved|abandoned] [--limit 20]
```

### `znt research get <id>`

Returns research item with linked findings, hypotheses, tasks, and insights.

```bash
znt research get <id>
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

### `znt research registry <query>`

Query package registries directly (crates.io, npm, hex, pypi). Does not create any state -- pure lookup.

```bash
znt research registry <query> [--ecosystem rust|npm|hex|pypi|all] [--limit 10]
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

### `znt finding create`

```bash
znt finding create --content <content> [--research <id>] [--source <src>] [--confidence high|medium|low] [--tag <tag>...]
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

### `znt finding update <id>`

```bash
znt finding update <id> [--content <content>] [--confidence <conf>] [--source <src>]
```

### `znt finding list`

```bash
znt finding list [--research <id>] [--tag <tag>] [--confidence high|medium|low] [--limit 20] [--search <fts-query>]
```

The `--search` flag uses FTS5 full-text search on the content and source fields.

### `znt finding get <id>`

Returns finding with tags, linked entities, and related audit entries.

### `znt finding tag <id> <tag>`

Add a tag to a finding.

### `znt finding untag <id> <tag>`

Remove a tag from a finding.

---

## 9. Hypotheses

### `znt hypothesis create`

```bash
znt hypothesis create --content <content> [--research <id>] [--finding <id>]
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

### `znt hypothesis update <id>`

```bash
znt hypothesis update <id> [--status unverified|analyzing|confirmed|debunked|partially_confirmed|inconclusive] [--reason <reason>] [--content <content>]
```

When status changes, `reason` should explain why:

```bash
znt hypothesis update hyp-e1c4b2 --status confirmed \
    --reason "Tested in spike. reqwest::ClientBuilder supports tower::Layer via .layer() method."
```

### `znt hypothesis list`

```bash
znt hypothesis list [--status <status>] [--research <id>] [--limit 20]
```

### `znt hypothesis get <id>`

Returns hypothesis with linked research, findings, and related entities.

---

## 10. Insights

### `znt insight create`

```bash
znt insight create --content <content> [--research <id>] [--confidence high|medium|low]
```

### `znt insight update <id>`

```bash
znt insight update <id> [--content <content>] [--confidence <conf>]
```

### `znt insight list`

```bash
znt insight list [--research <id>] [--limit 20]
```

### `znt insight get <id>`

---

## 11. Tasks

### `znt task create`

```bash
znt task create --title <title> [--description <desc>] [--research <id>]
```

### `znt task update <id>`

```bash
znt task update <id> [--title <title>] [--description <desc>] [--status open|in_progress|done|blocked]
```

### `znt task list`

```bash
znt task list [--status <status>] [--research <id>] [--limit 20]
```

### `znt task get <id>`

Returns task with linked research, findings, implementation log entries, and blocking/blocked-by relationships.

### `znt task complete <id>`

Shorthand for `znt task update <id> --status done`.

---

## 12. Implementation Log

### `znt log`

Record an implementation location.

```bash
znt log <file#lines> [--task <id>] [--description <desc>]
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

### `znt compat check`

Create or update a compatibility check between two packages.

```bash
znt compat check <package-a> <package-b> [--status compatible|incompatible|conditional|unknown] [--conditions <text>] [--finding <id>]
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

### `znt compat list`

```bash
znt compat list [--package <name>] [--status <status>] [--limit 20]
```

### `znt compat get <id>`

---

## 14. Entity Links

### `znt link`

Create a relationship between any two entities.

```bash
znt link <source> <target> <relation>
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

### `znt unlink <link-id>`

Remove an entity link.

---

## 15. Audit Trail

### `znt audit`

View the audit trail.

```bash
znt audit [--limit 20] [--entity-type <type>] [--entity-id <id>] [--action <action>] [--session <id>] [--search <fts-query>]
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

## 16. Studies (Structured Learning)

### `znt study create`

Create a new structured learning study.

```bash
znt study create --topic <topic> [--library <lib>] [--methodology explore|test-driven|compare] [--research <id>]
```

**Flags:**

| Flag | Description | Default |
|------|-------------|---------|
| `--topic` | What to learn about | Required |
| `--library` | Library/framework being studied | None |
| `--methodology` | `explore`, `test-driven`, `compare` | `explore` |
| `--research` | Link to existing research item | Auto-created |

**Output:**

```json
{
    "study": {
        "id": "stu-a1b2c3d4",
        "topic": "How tokio::spawn works",
        "library": "tokio",
        "methodology": "explore",
        "status": "active",
        "research_id": "res-e5f6a7b8",
        "session_id": "ses-c4e2d1"
    }
}
```

### `znt study assume <id>`

Add an assumption (creates a hypothesis linked to the study).

```bash
znt study assume <study-id> --content <assumption>
```

**Output:**

```json
{
    "assumption": {
        "id": "hyp-d4e5f6a7",
        "study_id": "stu-a1b2c3d4",
        "content": "spawn requires Send + 'static bounds",
        "status": "unverified"
    }
}
```

### `znt study test <id>`

Record a test result against an assumption. Creates a finding tagged `test-result`, links it to the hypothesis, and updates hypothesis status.

```bash
znt study test <study-id> --assumption <hyp-id> --result validated|invalidated|inconclusive --evidence <text>
```

**Output:**

```json
{
    "test_result": {
        "finding_id": "fnd-b7c8d9e0",
        "assumption_id": "hyp-d4e5f6a7",
        "result": "validated",
        "evidence": "Compile error E0277 proves Send + 'static is required",
        "assumption_status": "confirmed"
    }
}
```

### `znt study get <id>`

Get full study state including all assumptions, findings, and progress.

```bash
znt study get <id>
```

**Output:**

```json
{
    "study": {
        "id": "stu-a1b2c3d4",
        "topic": "How tokio::spawn works",
        "library": "tokio",
        "methodology": "explore",
        "status": "active"
    },
    "progress": {
        "total": 3,
        "confirmed": 2,
        "debunked": 1,
        "unverified": 0,
        "analyzing": 0,
        "inconclusive": 0
    },
    "assumptions": [
        {"id": "hyp-d4e5f6a7", "content": "spawn requires Send + 'static", "status": "confirmed", "reason": "E0277 proves it"},
        {"id": "hyp-e5f6a7b8", "content": "panic doesn't crash runtime", "status": "confirmed", "reason": "JoinHandle catches it"},
        {"id": "hyp-f6a7b8c9", "content": "spawn is zero-cost", "status": "debunked", "reason": "Allocates ~200 bytes"}
    ],
    "findings": [
        {"id": "fnd-b7c8d9e0", "content": "Test: non-Send type -> E0277", "tags": ["test-result"]}
    ],
    "conclusions": []
}
```

### `znt study conclude <id>`

Conclude the study. Sets status to `completed`, stores summary, creates an insight.

```bash
znt study conclude <id> --summary <text>
```

**Output:**

```json
{
    "study": {
        "id": "stu-a1b2c3d4",
        "status": "completed",
        "summary": "Tokio's spawn requires Send + 'static. NOT zero-cost (~200B). Panics caught via JoinHandle."
    },
    "insight": {
        "id": "ins-c9d0e1f2",
        "content": "Tokio's spawn requires Send + 'static..."
    }
}
```

### `znt study list`

List all studies.

```bash
znt study list [--status active|concluding|completed|abandoned] [--library <lib>] [--limit 20]
```

---

## 17. Database & Trail Commands

### `znt rebuild`

Rebuild the SQLite database from JSONL trail files. Deletes the existing DB and replays all operations from `.zenith/trail/*.jsonl`.

```bash
znt rebuild [--trail-dir <path>]
```

**Flags:**

| Flag | Description | Default |
|------|-------------|---------|
| `--trail-dir` | Path to trail directory | `.zenith/trail/` |

**Process:**
1. Delete `zenith.db`, `zenith.db-wal`, `zenith.db-shm`
2. Create fresh database with full schema (tables, FTS5, triggers, indexes)
3. Read all `.jsonl` files from trail directory
4. Sort all operations by timestamp across all files
5. Replay each operation (INSERT/UPDATE/DELETE)
6. FTS5 triggers fire automatically on INSERT

**Output:**

```json
{
    "rebuilt": true,
    "trail_files": 3,
    "operations_replayed": 247,
    "entities_created": 42,
    "duration_ms": 150
}
```

**When to use:**
- DB corruption: `rm .zenith/zenith.db && znt rebuild`
- New machine: `git clone <repo> && znt rebuild`
- Schema migration: update schema, rebuild from trail

---

## 18. Workflow Commands

### `znt whats-next`

Returns current project state and suggested next actions. Reads the last session snapshot, open tasks, pending hypotheses, and recent audit entries.

```bash
znt whats-next [--limit 10] [--format json|table|raw]
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

### `znt wrap-up`

End the current session, generate summary, sync to cloud.

```bash
znt wrap-up [--auto-commit] [--message <msg>]
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

## 19. Output Formats

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
