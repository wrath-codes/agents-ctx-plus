# Architecture Overview

This document explains Beads' three-layer architecture and how the layers work together to provide a robust, git-backed issue tracking system.

## 🏗️ Three-Layer Architecture

Beads uses a sophisticated three-layer architecture where each layer serves a specific purpose in the system:

```
flowchart TD
    subgraph GIT["🗂️ Layer 1: Git Repository"]
        G[(".beads/*.jsonl<br/><i>Historical Source of Truth</i>")]
    end
    
    subgraph JSONL["📄 Layer 2: JSONL Files"]
        J[("issues.jsonl<br/><i>Operational Source of Truth</i>")]
    end
    
    subgraph SQL["⚡ Layer 3: SQLite"]
        D[("beads.db<br/><i>Fast Queries / Derived State</i>")]
    end
    
    G <-->|"bd sync"| J
    J -->|"rebuild"| D
    D -->|"append"| J
    U((👤 User)) -->|"bd create<br/>bd update"| D
    D -->|"bd list<br/>bd show"| U
    
    style GIT fill:#2d5a27,stroke:#4a9c3e,color:#fff
    style JSONL fill:#1a4a6e,stroke:#3a8ac4,color:#fff
    style SQL fill:#6b3a6b,stroke:#a45ea4,color:#fff
```

### Layer 1: Git Repository - Historical Source of Truth

**Purpose**: Long-term storage and version history
**Location**: `.beads/*.jsonl` files (committed to git)
**Characteristics**:
- Issues travel with code in the same repository
- Full Git history of all issue changes
- Branch and merge support like source code
- Works offline and syncs when connected
- No external service dependency

**Why Git?**
- **Issues travel with code**: Context is always with the relevant codebase
- **No external dependency**: No server downtime, vendor lock-in, or network requirements
- **Full history**: Git log preserves complete issue evolution
- **Branch support**: Feature branches can have their own issues
- **Offline-first**: Full functionality without internet connection

### Layer 2: JSONL Files - Operational Source of Truth

**Purpose**: Append-only, mergeable operational data
**Location**: `.beads/issues.jsonl` (git-tracked)
**Format**: JSON Lines (one JSON object per line)

**Sample JSONL Structure**:
```jsonl
{"id": "bd-a1b2", "type": "create", "timestamp": "2026-02-06T10:00:00Z", "data": {"title": "Set up database", "priority": 1}}
{"id": "bd-a1b2", "type": "update", "timestamp": "2026-02-06T10:30:00Z", "data": {"status": "in_progress"}}
{"id": "bd-c3d4", "type": "create", "timestamp": "2026-02-06T11:00:00Z", "data": {"title": "Create API", "priority": 2}}
```

**Why JSONL?**
- **Human-readable**: Easy to inspect and understand
- **Git-mergeable**: Append-only format minimizes merge conflicts
- **Portable**: Works across all platforms and systems
- **Recoverable**: Can be restored from Git history
- **Incremental**: New operations append, don't modify existing data

**Merge Conflict Prevention**:
```bash
# Branch A adds issue: bd-a1b2
{"id": "bd-a1b2", "type": "create", ...}

# Branch B adds issue: bd-c3d4  
{"id": "bd-c3d4", "type": "create", ...}

# Git merges cleanly - just concatenates additions
```

### Layer 3: SQLite Database - Fast Queries / Derived State

**Purpose**: High-performance queries and complex operations
**Location**: `.beads/beads.db` (gitignored)
**Characteristics**:
- Indexed lookups in milliseconds
- Complex filtering and sorting
- Derived from JSONL (always rebuildable)
- Safe to delete and rebuild

**Why SQLite?**
- **Instant queries**: No network latency, local indexes
- **Complex filtering**: SQL supports sophisticated queries
- **Rebuildable**: Always can regenerate from JSONL
- **Lightweight**: No separate database server needed

**Table Structure** (simplified):
```sql
-- Issues table
CREATE TABLE issues (
    id TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    description TEXT,
    status TEXT DEFAULT 'open',
    priority INTEGER DEFAULT 2,
    type TEXT DEFAULT 'task',
    created_at TIMESTAMP,
    updated_at TIMESTAMP
);

-- Dependencies table  
CREATE TABLE dependencies (
    parent_id TEXT,
    child_id TEXT,
    type TEXT, -- 'blocks', 'parent-child', 'discovered-from', 'related'
    FOREIGN KEY (parent_id) REFERENCES issues(id),
    FOREIGN KEY (child_id) REFERENCES issues(id)
);

-- Labels table
CREATE TABLE labels (
    issue_id TEXT,
    label TEXT,
    FOREIGN KEY (issue_id) REFERENCES issues(id)
);
```

## 🔄 Data Flow

### Write Path (User Operations)
```
User runs: bd create "New issue"
    ↓
SQLite updated immediately
    ↓  
JSONL appended with new operation
    ↓
Git commit (on sync)
```

### Read Path (Queries)
```
User runs: bd list --status open
    ↓
SQLite queried with indexed filters
    ↓
Results returned immediately (milliseconds)
```

### Sync Path (Synchronization)
```
User runs: bd sync
    ↓
Git pull (get remote changes)
    ↓
JSONL merged (resolve conflicts if any)
    ↓
SQLite rebuilt if JSONL changed
    ↓
Git push (share local changes)
```

## 🔄 Sync Modes

### Standard Sync
```bash
bd sync
```
Normal bidirectional sync: pulls remote changes, merges JSONL, rebuilds SQLite if needed, pushes local changes.

### Import-Only Mode
```bash
bd sync --import-only
```
Rebuilds SQLite database from JSONL without pushing changes. Use when:
- SQLite is corrupted or missing
- Recovering from a fresh clone
- Rebuilding after database migration issues

**This is the safest recovery option when JSONL is intact.**

### Force Rebuild Mode
```bash
bd sync --force-rebuild
```
Forces complete SQLite rebuild from JSONL, discarding any SQLite-only state. Use with caution:
- More aggressive than `--import-only`
- May lose any uncommitted database state
- Recommended when standard sync fails repeatedly

## 🛡️ Recovery Model

The three-layer architecture makes recovery straightforward because each layer can rebuild from the one above it:

### Recovery Hierarchy
1. **Lost SQLite?** → Rebuild from JSONL: `bd sync --import-only`
2. **Lost JSONL?** → Recover from Git history: `git checkout HEAD~1 -- .beads/issues.jsonl`
3. **Conflicts?** → Git merge, then rebuild

### Universal Recovery Sequence
```bash
# This sequence resolves the majority of reported issues
bd daemons killall           # Stop daemons (prevents race conditions)
git worktree prune           # Clean orphaned worktrees
rm .beads/beads.db*         # Remove potentially corrupted database
bd sync --import-only        # Rebuild from JSONL source of truth
```

## 🎯 Design Trade-offs

### Benefits of Three-Layer Architecture

| Benefit | Explanation |
|----------|-------------|
| **Works offline** | Git provides full functionality without internet |
| **Git-native history** | Complete issue evolution preserved in commits |
| **Fast queries** | SQLite provides instant local lookups |
| **Merge-friendly** | Append-only JSONL minimizes conflicts |
| **No server dependency** | No downtime, latency, or vendor lock-in |
| **Portable** | Issues travel with code across machines |

### Trade-offs and Limitations

| Trade-off | Impact |
|------------|---------|
| **Manual sync required** | No real-time collaboration between machines |
| **Git knowledge needed** | Users need basic Git understanding |
| **Append-only growth** | JSONL files grow over time (needs compaction) |
| **Single-repo scope** | Issues don't cross repository boundaries naturally |
| **Text-focused** | Not optimized for binary attachments |

## 🔧 The Daemon System

### Background Synchronization
The Beads daemon (`bd daemon`) handles background synchronization:

- **File watching**: Monitors `.beads/` for changes
- **Auto-sync**: Triggers sync on changes (5-second debounce)
- **Lock management**: Prevents concurrent database access
- **Performance**: Keeps SQLite in sync with JSONL automatically

### Daemon Lifecycle
```bash
# Start daemon (runs in background)
bd daemon start

# Check status
bd daemon status

# Stop daemon
bd daemon stop

# Kill all daemons (useful for recovery)
bd daemons killall
```

### Running Without Daemon
For CI/CD pipelines, containers, and single-use scenarios:
```bash
# Commands work without spawning daemon
bd --no-daemon create "CI-generated issue"
bd --no-daemon sync
```

**When to use `--no-daemon`:**
- CI/CD pipelines (Jenkins, GitHub Actions)
- Docker containers
- Ephemeral environments  
- Scripts that should not leave background processes
- Debugging daemon-related issues

## 🏢 Multi-Machine Considerations

### Race Conditions in Multi-Clone Workflows
When multiple git clones of the same repository run daemons simultaneously:

**Common Scenarios:**
- Multi-agent AI workflows (multiple Claude/GPT instances)
- Developer workstations with multiple checkouts
- Worktree-based development workflows

**Prevention:**
1. Use `bd daemons killall` before switching between clones
2. Ensure only one clone's daemon is active at a time
3. Consider `--no-daemon` mode for automated workflows

### Safe Multi-Machine Workflow
```bash
# Machine A: Before switching
bd sync                    # Push changes
bd daemon stop             # Stop daemon

# Machine B: After switching  
git pull                    # Get latest changes
bd sync --import-only       # Rebuild local DB
bd daemon start             # Start daemon locally

# Work on Machine B...
bd create "New issue"       # Creates locally
bd sync                    # Sync when done
```

## 🔗 Related Documentation

- [Git Layer](git-layer.md) - Git integration details
- [JSONL Layer](jsonl-layer.md) - Operational format specifics  
- [SQLite Layer](sqlite-layer.md) - Database schema and queries
- [Daemon System](daemon-system.md) - Background sync implementation
- [Data Flow](data-flow.md) - Detailed flow diagrams
- [Recovery Overview](../recovery/) - Complete recovery procedures

## 📚 See Also

- [Core Features](../core-features/) - Issue management capabilities
- [CLI Reference](../cli-reference/) - Complete command documentation
- [Recovery](../recovery/) - Troubleshooting and disaster recovery
- [Multi-Agent](../multi-agent/) - Coordination across machines and agents