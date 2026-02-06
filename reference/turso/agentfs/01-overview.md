# AgentFS Overview

## What is AgentFS?

AgentFS is a copy-on-write (CoW) filesystem built on SQLite for AI agents. It provides workspace isolation, built-in auditing, and cloud synchronization—designed specifically for AI agent workflows.

## Key Value Propositions

### 1. Workspace Isolation
Each agent gets an isolated workspace where changes don't affect the base system until explicitly committed. This enables:
- Safe experimentation without risk
- Parallel agent execution
- Easy rollback of changes
- Reproducible workflows

### 2. Built-in Auditing
Every operation is automatically logged:
- Who made the change
- What was changed
- When it happened
- Full change history

### 3. Cloud Synchronization
Workspaces can sync with Turso Cloud:
- Multi-agent coordination
- Persistent state across sessions
- Team collaboration
- Backup and recovery

### 4. Copy-on-Write Efficiency
Storage-efficient workspace creation:
- Instant workspace creation
- Only changed data is duplicated
- Minimal storage overhead
- Fast branching

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                      AgentFS Architecture                    │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  ┌──────────────────────────────────────────────────────┐  │
│  │                    Base Filesystem                    │  │
│  │  ┌────────────┐  ┌────────────┐  ┌────────────┐     │  │
│  │  │   File A   │  │   File B   │  │   File C   │     │  │
│  │  │  (v1.0)    │  │  (v1.0)    │  │  (v1.0)    │     │  │
│  │  └────────────┘  └────────────┘  └────────────┘     │  │
│  └──────────────────────────────────────────────────────┘  │
│                          │                                  │
│          ┌───────────────┼───────────────┐                 │
│          │               │               │                  │
│  ┌───────▼──────┐ ┌──────▼──────┐ ┌──────▼──────┐          │
│  │ Workspace 1  │ │ Workspace 2 │ │ Workspace 3 │          │
│  │ (Agent A)    │ │ (Agent B)   │ │ (Agent C)   │          │
│  │              │ │             │ │             │          │
│  │ File A (CoW) │ │ File B (CoW)│ │ File C (CoW)│          │
│  │ File D (new) │ │             │ │ File D (CoW)│          │
│  └──────────────┘ └─────────────┘ └─────────────┘          │
│                                                             │
│  All stored in single SQLite database with:                 │
│  - Files table (content, metadata)                          │
│  - Snapshots table (versions)                               │
│  - Audit log (all operations)                               │
│  - Sync metadata (cloud integration)                        │
└─────────────────────────────────────────────────────────────┘
```

## Core Concepts

### Workspaces
Isolated environments for agents:
```
Base: /home/user/project
├── src/
├── tests/
└── config.json

Workspace "feature-x":
├── src/          (copy-on-write from base)
├── tests/        (copy-on-write from base)
└── config.json   (modified by agent)

Workspace "experiment-y":
├── src/          (copy-on-write from base)
├── tests/        (modified by agent)
└── config.json   (copy-on-write from base)
```

### Snapshots
Point-in-time captures of workspace state:
```bash
# Create snapshot
agentfs snapshot create my-workspace --name "before-refactor"

# List snapshots
agentfs snapshot list my-workspace

# Restore to snapshot
agentfs snapshot restore my-workspace before-refactor
```

### Copies (CoW)
When a file is modified in a workspace:
1. Original stays unchanged in base
2. Copy is created in workspace
3. Only the copy is modified
4. Minimal storage overhead

### Audit Trail
Every operation is logged:
```json
{
  "timestamp": "2024-01-15T10:30:00Z",
  "workspace": "agent-123",
  "operation": "write",
  "path": "/src/main.py",
  "size_before": 1024,
  "size_after": 1152,
  "checksum_before": "abc123...",
  "checksum_after": "def456..."
}
```

## Quick Start

### Installation
```bash
# macOS
brew install turso/tap/agentfs

# Linux
curl -sSfL https://get.tur.so/install.sh | bash -s agentfs

# Verify installation
agentfs --version
```

### Basic Usage
```bash
# Initialize AgentFS in a directory
cd ~/my-project
agentfs init

# Create workspace for agent
agentfs workspace create agent-1

# Run command in isolated workspace
agentfs run --workspace agent-1 ./build-script.sh

# Check what changed
agentfs status --workspace agent-1

# Commit changes to base
agentfs commit --workspace agent-1 -m "Build artifacts from agent-1"
```

## Use Cases

### 1. AI Code Agents
```bash
# Agent experiments with refactoring
agentfs workspace create refactor-agent

# Agent makes changes in isolation
agentfs run --workspace refactor-agent -- \
  python refactoring-agent.py --target src/

# Review changes
agentfs diff --workspace refactor-agent

# If good, commit; if bad, discard
agentfs commit --workspace refactor-agent -m "Automated refactoring"
# or
agentfs workspace delete refactor-agent
```

### 2. Multi-Agent Coordination
```bash
# Three agents working in parallel
agentfs workspace create agent-frontend
agentfs workspace create agent-backend
agentfs workspace create agent-tests

# Each agent works independently
agentfs run --workspace agent-frontend -- npm run build
agentfs run --workspace agent-backend -- cargo build
agentfs run --workspace agent-tests -- pytest

# Merge successful work
agentfs commit --workspace agent-frontend
agentfs commit --workspace agent-backend
agentfs commit --workspace agent-tests
```

### 3. Reproducible Experiments
```bash
# Create experiment workspace
agentfs workspace create experiment-v1

# Run experiment
agentfs run --workspace experiment-v1 -- python experiment.py --param 0.5

# Snapshot results
agentfs snapshot create experiment-v1 --name "param-0.5-results"

# Run with different params
agentfs workspace create experiment-v2 --from experiment-v1
agentfs run --workspace experiment-v2 -- python experiment.py --param 0.7

# Compare results
agentfs diff experiment-v1 experiment-v2
```

### 4. Safe System Administration
```bash
# Test configuration changes safely
agentfs workspace create config-test

# Make changes in workspace
agentfs run --workspace config-test -- bash -c "
  echo 'new_setting=true' >> /etc/app/config.conf
  systemctl restart app
  curl http://localhost:8080/health
"

# If tests pass, apply to real system
agentfs commit --workspace config-test
```

## Comparison with Traditional Approaches

| Approach | Isolation | Audit | Rollback | Overhead |
|----------|-----------|-------|----------|----------|
| Direct changes | ❌ | ❌ | ❌ | None |
| Git branches | ✅ | ⚠️ | ✅ | Medium |
| Docker containers | ✅ | ⚠️ | ✅ | High |
| VMs | ✅ | ⚠️ | ✅ | Very High |
| **AgentFS** | ✅ | ✅ | ✅ | **Low** |

## Storage Format

AgentFS stores everything in a single SQLite database:

```sql
-- Files table
CREATE TABLE files (
    id INTEGER PRIMARY KEY,
    path TEXT NOT NULL,
    content BLOB,
    mode INTEGER,
    uid INTEGER,
    gid INTEGER,
    mtime DATETIME,
    workspace_id INTEGER,
    snapshot_id INTEGER,
    checksum TEXT
);

-- Workspaces table
CREATE TABLE workspaces (
    id INTEGER PRIMARY KEY,
    name TEXT UNIQUE NOT NULL,
    base_snapshot_id INTEGER,
    created_at DATETIME,
    metadata JSON
);

-- Audit log
CREATE TABLE audit_log (
    id INTEGER PRIMARY KEY,
    timestamp DATETIME DEFAULT CURRENT_TIMESTAMP,
    workspace_id INTEGER,
    operation TEXT,
    path TEXT,
    details JSON
);

-- Sync metadata
CREATE TABLE sync_metadata (
    id INTEGER PRIMARY KEY,
    workspace_id INTEGER,
    turso_db_url TEXT,
    last_sync DATETIME,
    sync_status TEXT
);
```

## Next Steps

- **Core Concepts**: [02-core-concepts.md](./02-core-concepts.md)
- **Installation**: [03-installation.md](./03-installation.md)
- **CLI Reference**: [04-cli-reference.md](./04-cli-reference.md)
- **SDKs**: [06-sdks/](./06-sdks/)