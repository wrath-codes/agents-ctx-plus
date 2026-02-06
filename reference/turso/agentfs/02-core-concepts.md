# Core Concepts

## Workspaces

Workspaces are isolated environments where agents can make changes without affecting the base filesystem.

### Creating Workspaces
```bash
# Simple workspace
agentfs workspace create my-workspace

# From specific snapshot
agentfs workspace create my-workspace --from-snapshot stable-v1

# From another workspace
agentfs workspace create my-workspace --from-workspace other-workspace

# With metadata
agentfs workspace create my-workspace \
  --description "Testing new feature" \
  --tag experiment \
  --agent-id agent-123
```

### Workspace Lifecycle
```bash
# List workspaces
agentfs workspace list

# Show workspace details
agentfs workspace show my-workspace

# Rename workspace
agentfs workspace rename my-workspace new-name

# Delete workspace
agentfs workspace delete my-workspace
```

### Workspace Isolation
```
Base Filesystem:
/home/user/project/
├── src/
│   ├── main.py
│   └── utils.py
├── config.json
└── data/
    └── input.csv

Workspace "experiment":
/home/user/project/  (viewed through workspace)
├── src/
│   ├── main.py      (copy-on-write - original)
│   └── utils.py     (modified - workspace copy)
├── config.json      (copy-on-write - original)
└── data/
    └── input.csv    (copy-on-write - original)
```

## Snapshots

Snapshots capture the state of a workspace at a point in time.

### Creating Snapshots
```bash
# Named snapshot
agentfs snapshot create my-workspace --name "before-refactor"

# With description
agentfs snapshot create my-workspace \
  --name "checkpoint-1" \
  --description "Working state before major changes"

# Tagged snapshot
agentfs snapshot create my-workspace \
  --name "experiment-results" \
  --tags "experiment,results,v1"
```

### Managing Snapshots
```bash
# List snapshots
agentfs snapshot list my-workspace

# Show snapshot details
agentfs snapshot show my-workspace snapshot-name

# Delete snapshot
agentfs snapshot delete my-workspace snapshot-name

# Compare snapshots
agentfs snapshot diff my-workspace snapshot-1 snapshot-2
```

### Restoring Snapshots
```bash
# Restore workspace to snapshot
agentfs snapshot restore my-workspace snapshot-name

# Restore to new workspace
agentfs workspace create recovered-workspace \
  --from-snapshot my-workspace/snapshot-name
```

## Copy-on-Write (CoW)

### How CoW Works

When a file is first accessed in a workspace:
```
1. File exists in base (not copied yet)
   Base: file.txt (v1)
   Workspace: (reference to base)

2. File is read
   Returns base version (no copy)

3. File is modified
   a. Copy base file to workspace
   b. Modify the copy
   Base: file.txt (v1)           (unchanged)
   Workspace: file.txt (v2)      (modified copy)

4. Subsequent reads
   Return workspace version
```

### Storage Efficiency
```
Scenario: 1000 files, agent modifies 10

Full Copy Approach:
- Copy all 1000 files: 1000 × 4KB = 4MB overhead

CoW Approach:
- Only copy 10 modified files: 10 × 4KB = 40KB overhead
- 99% storage savings
```

### Implementation Details
```rust
struct FileEntry {
    path: PathBuf,
    content_hash: String,
    storage_location: StorageLocation,
}

enum StorageLocation {
    Base,              // Reference to base filesystem
    Workspace(u64),    // Stored in workspace (id)
    Snapshot(u64),     // Stored in snapshot (id)
}

// When reading:
// 1. Check workspace for file
// 2. If not found, check base
// 3. Return content

// When writing:
// 1. If file in base, copy to workspace first
// 2. Modify workspace copy
// 3. Update metadata
```

## Audit Trail

Every operation is logged for complete traceability.

### What's Logged
```json
{
  "id": 12345,
  "timestamp": "2024-01-15T10:30:00.123Z",
  "workspace_id": 42,
  "workspace_name": "agent-1",
  "operation": "write",
  "path": "/src/main.py",
  "details": {
    "size_before": 1024,
    "size_after": 1152,
    "checksum_before": "sha256:abc123...",
    "checksum_after": "sha256:def456...",
    "lines_added": 5,
    "lines_removed": 2
  },
  "agent_id": "agent-123",
  "session_id": "sess-456"
}
```

### Operations Tracked
- `create` - File/directory creation
- `read` - File access
- `write` - File modification
- `delete` - File/directory deletion
- `rename` - File/directory rename
- `chmod` - Permission changes
- `snapshot` - Snapshot creation
- `commit` - Commit to base

### Querying Audit Log
```bash
# All operations in workspace
agentfs audit my-workspace

# Specific operation type
agentfs audit my-workspace --operation write

# Specific time range
agentfs audit my-workspace \
  --from "2024-01-15T00:00:00Z" \
  --to "2024-01-15T23:59:59Z"

# Specific file
agentfs audit my-workspace --path /src/main.py

# Export to file
agentfs audit my-workspace --format json > audit-log.json
```

## Sync and Cloud Integration

### Local-First Design
AgentFS works offline by default. Cloud sync is optional.

### Sync Architecture
```
┌─────────────────────────────────────────────────────┐
│              Cloud Synchronization                   │
├─────────────────────────────────────────────────────┤
│                                                     │
│  ┌──────────────┐         ┌──────────────────┐     │
│  │ Local AgentFS│         │  Turso Cloud     │     │
│  │              │◄───────►│                  │     │
│  │ ┌──────────┐ │  Sync   │  ┌────────────┐  │     │
│  │ │Workspace │ │         │  │  Remote    │  │     │
│  │ │  SQLite  │ │         │  │  Replica   │  │     │
│  │ └──────────┘ │         │  └────────────┘  │     │
│  └──────────────┘         └──────────────────┘     │
│                                                     │
│  Sync modes:                                        │
│  - Real-time (immediate)                            │
│  - Periodic (every N seconds)                       │
│  - Manual (on demand)                               │
└─────────────────────────────────────────────────────┘
```

### Configuring Sync
```bash
# Enable sync for workspace
agentfs sync enable my-workspace \
  --turso-db libsql://mydb-org.turso.io \
  --token $TURSO_TOKEN

# Configure sync mode
agentfs sync config my-workspace \
  --mode real-time

# Manual sync
agentfs sync push my-workspace
agentfs sync pull my-workspace
```

### Multi-Agent Coordination
```bash
# Agent A creates workspace and syncs
agentfs workspace create shared-task
agentfs sync enable shared-task --turso-db $DB_URL
# ... does work ...
agentfs sync push shared-task

# Agent B pulls and continues
agentfs workspace create shared-task --from-sync $DB_URL
agentfs sync pull shared-task
# ... continues work ...
```

## Workspace Relationships

### Parent-Child Relationships
```
Base (root)
  └── workspace-1
        ├── workspace-1a (branched from workspace-1)
        └── workspace-1b (branched from workspace-1)
  └── workspace-2
        └── workspace-2a (branched from workspace-2)
```

### Inheritance
Child workspaces inherit from parents:
- Files not modified in child come from parent
- Changes in child don't affect parent
- Can merge child changes to parent

### Merging
```bash
# Merge workspace changes to base
agentfs commit my-workspace -m "Completed feature"

# Merge between workspaces
agentfs workspace merge source-workspace target-workspace

# Handle conflicts
agentfs workspace merge source-workspace target-workspace \
  --strategy theirs  # or 'ours' or 'manual'
```

## Permissions and Security

### Workspace Permissions
```bash
# Make workspace read-only
agentfs workspace config my-workspace --read-only true

# Restrict to specific agent
agentfs workspace config my-workspace --agent-id agent-123

# Set expiration
agentfs workspace config my-workspace --expires "2024-02-01T00:00:00Z"
```

### Access Control
```bash
# Share workspace with team
agentfs workspace share my-workspace \
  --user alice@example.com \
  --permissions read-write

# Revoke access
agentfs workspace unshare my-workspace \
  --user alice@example.com
```

## Performance Considerations

### Lazy Loading
Files are only copied when modified:
- Workspace creation: O(1) - instant
- First read of unmodified file: O(1) - reference base
- First write: O(n) - copy file content
- Subsequent operations: O(1) - use workspace copy

### Storage Optimization
```bash
# Garbage collect unused data
agentfs gc

# Analyze storage usage
agentfs storage analyze

# Compress old snapshots
agentfs snapshot compress old-snapshot
```

### Caching
```bash
# Configure cache size
agentfs config set --cache-size 100MB

# Clear cache
agentfs cache clear
```

## Next Steps

- **Installation**: [03-installation.md](./03-installation.md)
- **CLI Reference**: [04-cli-reference.md](./04-cli-reference.md)
- **Configuration**: [05-configuration.md](./05-configuration.md)
- **SDKs**: [06-sdks/](./06-sdks/)