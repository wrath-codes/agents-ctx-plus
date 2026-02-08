# Zenith: PRD Workflow

**Version**: 2026-02-08
**Status**: Design Document
**Purpose**: Product Requirement Document workflow integrated into Zenith, adapted from [snarktank/ai-dev-tasks](https://github.com/snarktank/ai-dev-tasks)

---

## Table of Contents

1. [Overview](#1-overview)
2. [Workflow Steps](#2-workflow-steps)
3. [Step 1: Create PRD](#3-step-1-create-prd)
4. [Step 2: Generate Tasks](#4-step-2-generate-tasks)
5. [Step 3: Execute Tasks](#5-step-3-execute-tasks)
6. [Data Model Integration](#6-data-model-integration)
7. [CLI Commands](#7-cli-commands)
8. [PRD Template](#8-prd-template)
9. [Task Generation Template](#9-task-generation-template)
10. [Zenith Adaptations](#10-zenith-adaptations)

---

## 1. Overview

The PRD workflow provides a structured approach to feature development:

```
Idea → PRD (clarify scope) → Task List (plan implementation) → Execute (one task at a time)
```

This is adapted from [ai-dev-tasks](https://github.com/snarktank/ai-dev-tasks) and integrated into Zenith's data model. Instead of standalone markdown files, PRDs and their generated tasks are stored in Zenith's Turso database, linked to issues, research items, findings, and the audit trail.

### What Changes From ai-dev-tasks

| ai-dev-tasks | Zenith |
|-------------|--------|
| PRD saved as `/tasks/prd-*.md` | PRD stored in Turso as an issue (type: `epic`) with content in description |
| Tasks saved as `/tasks/tasks-*.md` | Tasks stored in Turso, linked to the PRD epic via `issue_id` and `entity_links` |
| Checkboxes in markdown for progress | Task status in database (`open` → `in_progress` → `done`) |
| No audit trail | Every status change, task creation, and completion logged in audit trail |
| No linking to research/findings | Tasks and PRD can link to research items, findings, hypotheses, assumptions |
| Manual file management | LLM calls `znt prd create`, `znt prd tasks`, `znt task complete` |

### What Stays The Same

- The two-phase approach: PRD first, then task generation
- Clarifying questions before writing the PRD
- User confirmation ("Go") before generating sub-tasks
- One task at a time execution with verification
- Task numbering (0.0, 1.0, 1.1, 1.2, 2.0, etc.)
- Relevant files identification
- Branch creation as task 0.0

---

## 2. Workflow Steps

```
1. User describes a feature to the LLM
2. LLM calls `znt prd create --title "Feature Name"`
   → Zenith creates an epic issue and returns its ID
3. LLM asks clarifying questions (3-5, numbered, with lettered options)
4. User answers (e.g., "1A, 2C, 3B")
5. LLM generates PRD content and calls `znt prd update <id> --content <prd-markdown>`
   → PRD stored as the epic's description
6. LLM calls `znt prd tasks <id>`
   → Zenith creates parent tasks (high-level) linked to the epic
   → Returns task list, pauses for confirmation
7. User says "Go"
8. LLM generates sub-tasks and calls `znt prd subtasks <id>`
   → Zenith creates sub-tasks linked to their parent tasks
   → Identifies relevant files
9. LLM works through tasks one at a time:
   a. `znt task update <id> --status in_progress`
   b. Implements the task
   c. `znt task complete <id>`
   d. `znt log <file#lines> --task <id>` for each file changed
   e. Moves to next task
10. When all tasks done, LLM wraps up the PRD:
    `znt prd complete <id>`
```

---

## 3. Step 1: Create PRD

### LLM Prompt (stored in Zenith as a skill/template)

When the user asks to build a feature, the LLM follows this process:

**Phase 1: Clarify**

1. Create the PRD epic: `znt prd create --title "Feature Name"`
2. Ask 3-5 clarifying questions focusing on:
   - **Problem/Goal**: What problem does this solve?
   - **Core Functionality**: What are the key actions?
   - **Scope/Boundaries**: What should this NOT do?
   - **Success Criteria**: How do we know it's done?
3. Format questions with numbered items and lettered options for easy response

**Phase 2: Write**

After receiving answers, generate the PRD with these sections:

1. **Introduction/Overview** - Feature description and problem statement
2. **Goals** - Specific, measurable objectives
3. **User Stories** - User narratives
4. **Functional Requirements** - Numbered, explicit requirements
5. **Non-Goals (Out of Scope)** - What this does NOT include
6. **Design Considerations** - UI/UX notes, mockup links
7. **Technical Considerations** - Constraints, dependencies, architecture notes
8. **Success Metrics** - How success is measured
9. **Open Questions** - Remaining unknowns

Save: `znt prd update <epic-id> --content "<prd-markdown>"`

**Phase 3: Link**

If this PRD relates to existing research, findings, or hypotheses:

```bash
znt link <epic-id> <research-id> relates-to
znt link <epic-id> <finding-id> derived-from
```

---

## 4. Step 2: Generate Tasks

### Phase 1: Parent Tasks

The LLM analyzes the PRD and generates high-level parent tasks:

```bash
# Create parent tasks linked to the epic
znt task create --title "Create feature branch" --issue <epic-id> --description "git checkout -b feature/<name>"
znt task create --title "Set up data models" --issue <epic-id>
znt task create --title "Implement core logic" --issue <epic-id>
znt task create --title "Build API endpoints" --issue <epic-id>
znt task create --title "Add tests" --issue <epic-id>
znt task create --title "Integration testing and cleanup" --issue <epic-id>
```

The LLM presents these to the user and asks: "I've generated the high-level tasks. Ready to generate sub-tasks? Respond with 'Go'."

### Phase 2: Sub-Tasks (after user confirms)

Break each parent task into actionable sub-tasks:

```bash
# Sub-tasks for "Set up data models" (tsk-a2b3c4)
znt task create --title "Define User schema" --issue <epic-id> --description "Parent: tsk-a2b3c4"
znt task create --title "Add migrations" --issue <epic-id> --description "Parent: tsk-a2b3c4"
znt task create --title "Create repository trait" --issue <epic-id> --description "Parent: tsk-a2b3c4"

# Link sub-tasks to parent
znt link <subtask-id> <parent-task-id> depends-on
```

### Relevant Files

After generating tasks, the LLM creates findings for relevant files:

```bash
znt finding create --content "Files to create/modify for this feature" \
  --source "prd-analysis" \
  --tag "relevant-files" \
  --research <epic-id-or-research-id>
```

The finding content lists:

```
Relevant Files:
- src/models/user.rs - New data model for user profiles
- src/models/user_test.rs - Unit tests for user model
- src/api/users.rs - API endpoint handlers
- src/api/users_test.rs - Integration tests for API
- migrations/002_add_users.sql - Database migration
```

---

## 5. Step 3: Execute Tasks

### Task-by-Task Execution

The LLM works through tasks sequentially:

```bash
# 1. Start task
znt task update <task-id> --status in_progress

# 2. Implement the task (LLM writes code)

# 3. Log implementation locations
znt log src/models/user.rs#1-45 --task <task-id> --description "User struct with validation"
znt log src/models/user.rs#47-82 --task <task-id> --description "UserRepository trait implementation"

# 4. Complete the task
znt task complete <task-id>

# 5. If findings discovered during implementation:
znt finding create --content "User model needs email uniqueness constraint at DB level" \
  --tag "needs-verification" --source "src/models/user.rs"

# 6. If hypotheses to track:
znt hypothesis create --content "Using CHECK constraint for email format may be slower than app-level validation"

# 7. Move to next task
```

### Progress Tracking

At any point the LLM can check progress:

```bash
# See all tasks for this PRD
znt task list --issue <epic-id>

# See what's next
znt whats-next

# See audit trail for this epic
znt audit --entity-id <epic-id> --limit 20
```

### Completion

When all tasks are done:

```bash
# Mark the epic as done
znt issue update <epic-id> --status done

# Create a summary insight
znt insight create --content "Feature X implemented. 12 tasks completed. Key decisions: ..." \
  --research <research-id-if-any>

# Wrap up the session
znt wrap-up
```

---

## 6. Data Model Integration

The PRD workflow maps to existing Zenith entities:

| PRD Concept | Zenith Entity | Details |
|-------------|--------------|---------|
| PRD document | `issues` (type: `epic`) | PRD markdown stored in `description` |
| Parent tasks | `tasks` | Linked to epic via `issue_id` |
| Sub-tasks | `tasks` | Linked to parent via `entity_links` (depends-on) |
| Relevant files | `findings` | Tagged `relevant-files`, linked to epic |
| Task progress | `tasks.status` | `open` → `in_progress` → `done` |
| Implementation record | `implementation_log` | `file_path#start_line-end_line` per task |
| Discoveries during implementation | `findings` | Tagged appropriately, linked to task/epic |
| Open questions from PRD | `hypotheses` (status: `unverified`) | Tracked until resolved |
| Feature branch | `tasks` (task 0.0) | Standard first task |

### No New Tables Needed

The PRD workflow uses existing entities:
- `issues` with `type = 'epic'` for the PRD itself
- `tasks` for the task list
- `entity_links` for parent-child task relationships and PRD-to-task links
- `findings` for relevant files and discoveries
- `hypotheses` for open questions and assumptions
- `implementation_log` for tracking what was built where
- `audit_trail` for complete history

---

## 7. CLI Commands

### `znt prd create`

Create a new PRD (epic issue).

```bash
znt prd create --title <title> [--description <initial-desc>]
```

**Implementation**: Creates an issue with `type = 'epic'` and returns its ID.

**Output:**

```json
{
    "prd": {
        "id": "iss-a3f8b2c1",
        "title": "User Profile Editing",
        "type": "epic",
        "status": "open"
    }
}
```

### `znt prd update`

Update the PRD content (the generated PRD markdown goes into the description).

```bash
znt prd update <id> --content <prd-markdown>
```

**Implementation**: Updates the issue's `description` field.

### `znt prd get`

Get the full PRD with all linked tasks, findings, and progress.

```bash
znt prd get <id>
```

**Output:**

```json
{
    "prd": {
        "id": "iss-a3f8b2c1",
        "title": "User Profile Editing",
        "type": "epic",
        "status": "in_progress",
        "description": "# User Profile Editing\n\n## Introduction\n..."
    },
    "tasks": {
        "total": 15,
        "done": 8,
        "in_progress": 1,
        "open": 6,
        "blocked": 0,
        "items": [
            {"id": "tsk-b2c4d1e5", "title": "Create feature branch", "status": "done"},
            {"id": "tsk-c3d5e2f6", "title": "Set up data models", "status": "done"},
            {"id": "tsk-d4e6f3a7", "title": "Implement core logic", "status": "in_progress"}
        ]
    },
    "findings": [
        {"id": "fnd-e5f7a4b8", "content": "Relevant files: ...", "tags": ["relevant-files"]}
    ],
    "open_questions": [
        {"id": "hyp-f6a8b5c9", "content": "Should email changes require re-verification?", "status": "unverified"}
    ]
}
```

### `znt prd tasks`

Generate parent tasks for a PRD. Called by the LLM after writing the PRD.

```bash
znt prd tasks <id> --tasks '["Create feature branch", "Set up data models", "Implement core logic", "Add tests"]'
```

**Implementation**: Creates tasks linked to the epic, returns the list, and includes a message for the LLM to pause and ask the user to confirm before proceeding to sub-tasks.

**Output:**

```json
{
    "tasks": [
        {"id": "tsk-b2c4d1e5", "title": "Create feature branch", "status": "open"},
        {"id": "tsk-c3d5e2f6", "title": "Set up data models", "status": "open"},
        {"id": "tsk-d4e6f3a7", "title": "Implement core logic", "status": "open"},
        {"id": "tsk-e5f7a4b8", "title": "Add tests", "status": "open"}
    ],
    "message": "High-level tasks generated. Ask the user to confirm before generating sub-tasks."
}
```

### `znt prd subtasks`

Generate sub-tasks for a parent task.

```bash
znt prd subtasks <parent-task-id> --tasks '["Define User schema", "Add migrations", "Create repository trait"]'
```

**Implementation**: Creates tasks linked to the epic, creates `depends-on` links to the parent task.

### `znt prd complete`

Mark a PRD as completed.

```bash
znt prd complete <id>
```

**Implementation**: Sets the epic issue's status to `done`, creates a summary audit entry.

### `znt prd list`

List all PRDs (epic issues).

```bash
znt prd list [--status open|in_progress|done] [--limit 20]
```

---

## 8. PRD Template

This template is used by the LLM when generating a PRD. It mirrors the ai-dev-tasks `create-prd.md` structure.

### Clarifying Questions Format

```
1. What is the primary problem this feature solves?
   A. [Option A]
   B. [Option B]
   C. [Option C]
   D. Other (please specify)

2. Who is the target user?
   A. New users only
   B. Existing users only
   C. All users
   D. Admin users

3. What is the scope boundary?
   A. [Minimal scope]
   B. [Standard scope]
   C. [Extended scope]
```

### PRD Document Structure

```markdown
# PRD: [Feature Name]

## Introduction/Overview
[Brief description of the feature and the problem it solves]

## Goals
- [ ] Goal 1: [Specific, measurable objective]
- [ ] Goal 2: [Specific, measurable objective]

## User Stories
- As a [user type], I want to [action] so that [benefit]
- As a [user type], I want to [action] so that [benefit]

## Functional Requirements
1. [The system must...]
2. [The system must...]
3. [The system must...]

## Non-Goals (Out of Scope)
- [What this feature will NOT include]
- [Explicit boundary]

## Design Considerations
- [UI/UX notes]
- [Mockup links if any]

## Technical Considerations
- [Architecture constraints]
- [Dependencies]
- [Performance requirements]

## Success Metrics
- [How success is measured]
- [Specific metric with target]

## Open Questions
- [Remaining unknowns → these become hypotheses in Zenith]
```

---

## 9. Task Generation Template

This template mirrors the ai-dev-tasks `generate-tasks.md` structure, adapted for Zenith.

### Task Numbering Convention

```
0.0 - Feature branch creation (always first)
  0.1 - Create and checkout branch

1.0 - [First major area]
  1.1 - [Sub-task]
  1.2 - [Sub-task]

2.0 - [Second major area]
  2.1 - [Sub-task]
  2.2 - [Sub-task]
  2.3 - [Sub-task]

3.0 - [Testing]
  3.1 - [Unit tests]
  3.2 - [Integration tests]
```

### Execution Rules

1. **Always task 0.0 first**: Create the feature branch unless explicitly told not to
2. **One sub-task at a time**: Complete, verify, then move to next
3. **Log every file change**: `znt log <file#lines> --task <id>` after each implementation
4. **Mark completion immediately**: `znt task complete <id>` right after verifying
5. **Capture discoveries**: If something unexpected is found, `znt finding create` before moving on
6. **Track assumptions**: If an implementation choice is made based on an assumption, `znt hypothesis create`
7. **Update status before starting**: `znt task update <id> --status in_progress` before working on a task
8. **Link blocking relationships**: If task B can't start until task A is done, `znt link <B> <A> depends-on`

### Relevant Files Convention

After generating sub-tasks, identify relevant files as a finding:

```bash
znt finding create \
  --content "Relevant files for [Feature]:
- path/to/new_file.rs - [Why this file is needed]
- path/to/new_file_test.rs - Unit tests
- path/to/existing_file.rs - [What modification is needed]
- migrations/NNN_description.sql - Database migration" \
  --tag relevant-files \
  --source "prd-task-generation"
```

---

## 10. Zenith Adaptations

### How This Differs From Standalone ai-dev-tasks

**1. Persistence across sessions**

With ai-dev-tasks, task progress is tracked by checkboxes in a markdown file. If the chat session ends, the LLM has to re-read the file. With Zenith, task status is in the database. `znt whats-next` instantly tells the LLM where to continue.

**2. Linking to knowledge graph**

A PRD task that discovers a dependency conflict can immediately create a finding tagged `deps-conflict`, which can trigger a hypothesis, which can spawn a compatibility check. Everything is connected via entity_links.

**3. Audit trail**

Every task status change, every file modified, every finding discovered during implementation is logged in the audit trail. At wrap-up, the LLM can generate a summary of everything that happened during the PRD execution.

**4. Multi-session PRDs**

A PRD can span multiple sessions. Each session picks up where the last one left off:

```bash
# Session 2: LLM checks where we are
znt prd get <epic-id>
# → Shows 8/15 tasks done, 1 in progress, 6 open
# → Shows recent findings and unresolved hypotheses

znt whats-next
# → "Last session completed tasks 1.0-2.3. Task 2.4 is in progress. 
#    Hypothesis hyp-f6a8 about email validation is still unverified."
```

**5. Multiple PRDs in parallel**

Unlike standalone markdown files that can get confusing, Zenith tracks multiple PRDs as separate epic issues. Each has its own task tree, findings, and audit trail.

```bash
znt prd list --status in_progress
# → Shows all active PRDs with progress percentages
```

**6. Integration with package indexing**

If a PRD requires a new library:

```bash
# During task execution, LLM discovers need for a library
znt install serde_json --ecosystem rust
# → Indexed, now searchable

znt search "json serialization error handling" --package serde_json
# → Returns relevant API docs to help with implementation

znt finding create --content "Using serde_json::from_str for parsing. Returns Result<T, serde_json::Error>" \
  --source "package:serde_json" --tag verified
```

**7. Research-driven PRDs**

A PRD can be created from research results:

```bash
# Research identified reqwest as the best HTTP client
znt research get res-c4e2d1
# → Shows findings, confirmed hypotheses

# Create a PRD to implement the HTTP client layer using reqwest
znt prd create --title "HTTP Client Layer with reqwest"
znt link <epic-id> <research-id> derived-from
# → PRD is now connected to the research that justified it
```

---

## Cross-References

- Turso data model: [01-turso-data-model.md](./01-turso-data-model.md) (issues table for PRD epics, tasks table)
- CLI API design: [04-cli-api-design.md](./04-cli-api-design.md) (task and issue commands)
- Architecture overview: [03-architecture-overview.md](./03-architecture-overview.md) (data flow)
- Original source: [snarktank/ai-dev-tasks](https://github.com/snarktank/ai-dev-tasks) (create-prd.md, generate-tasks.md)
