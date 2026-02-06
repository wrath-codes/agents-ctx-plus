# Beads - Quick Introduction

> **A memory upgrade for your coding agent**

Beads (`bd`) is a distributed, git-backed graph issue tracker specifically designed for AI agents. It provides persistent, structured memory that replaces messy markdown plans with a dependency-aware graph.

## 🎯 Why Beads?

Traditional issue trackers (Jira, GitHub Issues) weren't designed for AI agents. Beads was built from the ground up for:

- **AI-native workflows** - Hash-based IDs prevent collisions when multiple agents work concurrently
- **Git-backed storage** - Issues sync via JSONL files, enabling collaboration across branches  
- **Dependency-aware execution** - `bd ready` shows only unblocked work
- **Formula system** - Declarative templates for repeatable workflows
- **Multi-agent coordination** - Routing, gates, and molecules for complex workflows

## ⚡ Quick Start

```bash
# Install (system-wide - don't clone into project)
curl -fsSL https://raw.githubusercontent.com/steveyegge/beads/main/scripts/install.sh | bash

# Initialize in YOUR project
cd your-project
bd init --quiet

# Create first issue
bd create "Set up database" -p 1 -t task

# Tell your agent to use Beads
echo "Use 'bd' for task tracking" >> AGENTS.md
```

## 🔑 Essential Commands

| Command | Action | Example |
|---------|--------|---------|
| `bd ready` | List tasks with no open blockers | `bd ready --json` |
| `bd create` | Create new issue | `bd create "Fix bug" -p 0 -t bug` |
| `bd update` | Update issue fields | `bd update bd-42 --status in_progress` |
| `bd dep add` | Link tasks | `bd dep add bd-2 bd-1` |
| `bd show` | View issue details | `bd show bd-42 --json` |

## 🏗️ Core Architecture

Beads uses a three-layer system:

```
┌─────────────────┐
│   Git Repo     │ ← Historical Source of Truth
│ (issues.jsonl) │
└────────┬────────┘
         │
┌────────▼────────┐
│   JSONL Files  │ ← Operational Source of Truth  
│ (append-only)  │
└────────┬────────┘
         │
┌────────▼────────┐
│   SQLite DB    │ ← Fast Queries / Derived State
│  (beads.db)   │
└─────────────────┘
```

- **Git**: Full history, travels with code
- **JSONL**: Append-only, git-mergeable format
- **SQLite**: Fast local queries, rebuildable from JSONL

## 🧬 Workflow Chemistry Metaphor

| Phase | Storage | Synced | Use Case |
|--------|---------|---------|----------|
| **Proto** | Built-in | N/A | Reusable templates |
| **Mol** | `.beads/` | Yes | Persistent work |
| **Wisp** | `.beads-wisp/` | No | Ephemeral operations |

## 🤖 For AI Agents

### Why Beads is Perfect for Agents

1. **Structured Memory**: No more losing context between sessions
2. **Dependency Awareness**: Always know what to work on next
3. **JSON API**: All commands support `--json` output
4. **Zero Collisions**: Hash-based IDs prevent multi-agent conflicts
5. **Git Integration**: Context travels with codebase

### Essential Agent Workflow

```bash
# 1. Get ready work (JSON for parsing)
bd ready --json

# 2. Start work on first available task
TASK=$(bd ready --json | jq -r '.issues[0].id')
bd update $TASK --status in_progress --json

# 3. Track discovered work during implementation
bd create "Found bug in auth" \
  --description="User input not sanitized" \
  --deps discovered-from:bd-100 \
  --json

# 4. Complete and sync at session end
bd close $TASK --reason "Fixed in commit abc123" --json
bd sync
```

## 🎯 Integration Options

### CLI + Hooks (Recommended)
```bash
# Setup for Claude Code
bd setup claude

# Manual hook configuration
{
  "hooks": {
    "SessionStart": ["bd prime"],
    "PreCompact": ["bd sync"]
  }
}
```

**Benefits**:
- ~1-2k tokens vs 10-50k for MCP schemas
- Lower latency than server-based approaches
- Direct CLI access for all features

### MCP Server (Alternative)
```json
{
  "mcpServers": {
    "beads": {
      "command": "beads-mcp"
    }
  }
}
```

**Use when**: CLI unavailable (Claude Desktop, etc.)

## 🔄 Multi-Agent Coordination

### Work Assignment
```bash
# Assign work to specific agent
bd pin bd-42 --for agent-1 --start

# Check what's pinned to you
bd hook

# Check other agent's work
bd hook --agent agent-1
```

### Cross-Repository Dependencies
```bash
# Track dependencies across repos
bd dep add bd-42 external:other-repo/bd-100

# View cross-repo dependency tree
bd dep tree bd-42 --cross-repo
```

### Automatic Routing
Create `.beads/routes.jsonl`:
```jsonl
{"pattern": "frontend/**", "target": "frontend-repo", "priority": 10}
{"pattern": "backend/**", "target": "backend-repo", "priority": 10}  
{"pattern": "*", "target": "main-repo", "priority": 0}
```

## 🛠️ Recovery & Reliability

### Universal Recovery Sequence
```bash
# Fix most common issues with this sequence
bd daemons killall           # Stop daemons (prevents race conditions)
git worktree prune           # Clean orphaned worktrees
rm .beads/beads.db*         # Remove potentially corrupted database
bd sync --import-only        # Rebuild from JSONL source of truth
```

### Recovery Layers
1. **Lost SQLite?** → Rebuild from JSONL: `bd sync --import-only`
2. **Lost JSONL?** → Recover from Git history
3. **Conflicts?** → Git merge, then rebuild

> **⚠️ CRITICAL**: Never use `bd doctor --fix` - it frequently causes more damage than the original problem!

## 🔧 Extension Points

### Custom Formulas
Create `.beads/formulas/my-workflow.formula.toml`:
```toml
formula = "feature-workflow"
description = "Standard feature development workflow"
version = 1

[[steps]]
id = "design"
title = "Design {{feature_name}}"
type = "human"

[[steps]]
id = "implement" 
title = "Implement {{feature_name}}"
needs = ["design"]

[[steps]]
id = "test"
title = "Test {{feature_name}}"
needs = ["implement"]
```

### Usage
```bash
# Create molecule from formula
bd pour feature-workflow --var feature_name="dark-mode"

# View molecule structure
bd mol show dark-mode-xyz

# Work through steps
bd update dark-mode-xyz.1 --status in_progress
bd close dark-mode-xyz.1
bd ready  # Shows next ready step
```

## 🎭 Gates (Async Coordination)

### Human Gates
```toml
[[steps]]
id = "approval"
title = "Manager approval"
type = "human"

[steps.gate]
type = "human"
approvers = ["team-lead", "security"]
require_all = false
```

### Timer Gates  
```toml
[[steps]]
id = "cooldown"
title = "Wait for cooldown period"

[steps.gate]
type = "timer"
duration = "24h"
```

### GitHub Gates
```toml
[[steps]]
id = "wait-ci"
title = "Wait for CI to pass"

[steps.gate]
type = "github"
event = "check_suite"
status = "success"
```

## 📊 When NOT to Use Beads

Beads is NOT suitable for:

- **Large teams (10+)** - Git-based sync doesn't scale for high-frequency concurrent edits
- **Non-developers** - Requires Git and command-line familiarity
- **Real-time collaboration** - No live updates; requires explicit sync
- **Cross-repository tracking** - Issues are scoped to single repositories
- **Rich media attachments** - Designed for text-based issue tracking

## 🔗 Learn More

- **[Complete Reference](index.md)** - Comprehensive documentation
- **[Architecture](architecture/)** - Three-layer system details
- **[CLI Reference](cli-reference/)** - Complete command guide
- **[Workflows](workflows/)** - Formula/molecule/gate system
- **[Recovery](recovery/)** - Troubleshooting and disaster recovery
- **[Context Enhancement](context-enhancement/)** - Building context CLIs

## 🌟 Community

- **[GitHub Repository](https://github.com/steveyegge/beads)** - Source code and issues
- **[Documentation](https://steveyegge.github.io/beads/)** - Official docs
- **[Community Tools](https://steveyegge.github.io/beads/docs/COMMUNITY_TOOLS.md)** - Third-party integrations

---

**Beads transforms AI coding agent collaboration by replacing chat logs and markdown files with a version-controlled database, providing persistent memory for larger projects.**