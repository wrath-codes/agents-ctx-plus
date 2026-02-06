# Cloud Sync

## Overview

AgentFS Cloud Sync enables synchronization of workspaces with Turso Cloud, providing persistent storage, multi-agent coordination, and backup capabilities.

## Sync Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    Cloud Sync Architecture                   │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  Local AgentFS                    Turso Cloud               │
│  ┌──────────────────┐            ┌──────────────────┐      │
│  │  Workspace       │            │  Remote Database │      │
│  │  ┌────────────┐  │   Sync     │  ┌────────────┐  │      │
│  │  │  files     │  │◄──────────►│  │  files     │  │      │
│  │  │  metadata  │  │   HTTP/2   │  │  metadata  │  │      │
│  │  │  audit     │  │            │  │  audit     │  │      │
│  │  └────────────┘  │            │  └────────────┘  │      │
│  └──────────────────┘            └──────────────────┘      │
│                                                             │
│  Sync modes:                                                │
│  • Real-time: Immediate sync on change                      │
│  • Periodic: Batch sync every N seconds                     │
│  • Manual: On-demand sync                                   │
└─────────────────────────────────────────────────────────────┘
```

## Enabling Sync

### CLI Setup

```bash
# Enable sync for workspace
agentfs sync enable my-workspace \
  --turso-db libsql://mydb-org.turso.io \
  --token $TURSO_TOKEN

# Configure sync mode
agentfs sync config my-workspace --mode real-time

# Manual sync
agentfs sync push my-workspace
agentfs sync pull my-workspace
```

### SDK Setup

**Python:**
```python
workspace.sync.enable(
    turso_db="libsql://mydb-org.turso.io",
    token="your-auth-token"
)
```

**Rust:**
```rust
workspace.sync()
    .enable("libsql://mydb-org.turso.io", "your-auth-token")
    .await?;
```

**TypeScript:**
```typescript
await workspace.sync.enable({
    tursoDb: 'libsql://mydb-org.turso.io',
    token: 'your-auth-token'
});
```

## Sync Modes

### Real-Time Sync
Changes are synced immediately.

```bash
# CLI
agentfs sync config my-workspace --mode real-time

# SDK
workspace.sync.config(mode='real-time')
```

**Characteristics:**
- Latency: < 1 second
- Best for: Multi-agent coordination
- Overhead: Higher (more API calls)

### Periodic Sync
Changes are batched and synced at intervals.

```bash
# CLI
agentfs sync config my-workspace \
  --mode periodic \
  --interval 300  # 5 minutes

# SDK
workspace.sync.config(mode='periodic', interval=300)
```

**Characteristics:**
- Latency: 5-300 seconds (configurable)
- Best for: Batch updates, lower API usage
- Overhead: Lower (batched operations)

### Manual Sync
Sync only when explicitly requested.

```bash
# CLI
agentfs sync config my-workspace --mode manual

# Push changes
agentfs sync push my-workspace

# Pull changes
agentfs sync pull my-workspace

# SDK
workspace.sync.config(mode='manual')
workspace.sync.push()
workspace.sync.pull()
```

**Characteristics:**
- Latency: Manual control
- Best for: Development, controlled sync
- Overhead: Minimal

## Conflict Resolution

### Conflict Types

1. **Local changes, remote changes** - Both modified same file
2. **Local delete, remote modify** - Deleted locally, modified remotely
3. **Local modify, remote delete** - Modified locally, deleted remotely

### Resolution Strategies

**Last-Write-Wins:**
```bash
# Most recent change wins
agentfs sync config my-workspace \
  --conflict-resolution last-write-wins
```

**Local-Wins:**
```bash
# Local changes always take precedence
agentfs sync config my-workspace \
  --conflict-resolution local-wins
```

**Remote-Wins:**
```bash
# Remote changes always take precedence
agentfs sync config my-workspace \
  --conflict-resolution remote-wins
```

**Manual:**
```bash
# Conflicts must be resolved manually
agentfs sync config my-workspace \
  --conflict-resolution manual
```

### Handling Conflicts in SDK

**Python:**
```python
from agentfs import SyncConflictError

try:
    workspace.sync.push()
except SyncConflictError as e:
    for conflict in e.conflicts:
        print(f"Conflict: {conflict.path}")
        # Resolve manually
        if resolve_strategy(conflict) == 'local':
            workspace.sync.resolve(conflict, use_local=True)
        else:
            workspace.sync.resolve(conflict, use_remote=True)
```

**Rust:**
```rust
match workspace.sync().push().await {
    Ok(_) => { /* success */ },
    Err(SyncError::Conflicts(conflicts)) => {
        for conflict in conflicts {
            match resolve_strategy(&conflict) {
                Strategy::Local => {
                    workspace.sync().resolve(&conflict, Resolution::Local).await?;
                }
                Strategy::Remote => {
                    workspace.sync().resolve(&conflict, Resolution::Remote).await?;
                }
            }
        }
    }
    Err(e) => return Err(e.into()),
}
```

## Multi-Agent Workflows

### Shared Workspace

```python
# Agent A creates and syncs workspace
workspace_a = afs_a.workspace.create("shared-task")
workspace_a.sync.enable(turso_db=DB_URL, token=TOKEN)
# ... does work ...
workspace_a.sync.push()

# Agent B pulls and continues
workspace_b = afs_b.workspace.create("shared-task")
workspace_b.sync.enable(turso_db=DB_URL, token=TOKEN)
workspace_b.sync.pull()
# ... continues work ...
workspace_b.sync.push()
```

### Work Distribution

```python
# Master agent distributes work
work_items = [
    {"id": 1, "file": "/src/module1.py"},
    {"id": 2, "file": "/src/module2.py"},
    {"id": 3, "file": "/src/module3.py"},
]

for item in work_items:
    # Create workspace for each task
    ws = master.workspace.create(f"task-{item['id']}")
    ws.sync.enable(turso_db=DB_URL, token=TOKEN)
    
    # Copy task info
    ws.write_file("/task.json", json.dumps(item))
    ws.sync.push()
    
    # Worker agents pick up tasks
    # ... workers process and sync back ...

# Collect results
for item in work_items:
    ws = master.workspace.get(f"task-{item['id']}")
    ws.sync.pull()
    result = ws.read_file("/result.json")
```

## Backup and Recovery

### Automated Backups

```bash
# Configure automatic sync as backup
agentfs sync enable my-workspace \
  --turso-db libsql://backup-org.turso.io \
  --token $BACKUP_TOKEN \
  --mode periodic \
  --interval 3600  # Hourly backup
```

### Point-in-Time Recovery

```python
# Sync to specific point in time
workspace.sync.pull(at="2024-01-15T10:30:00Z")

# Or restore from specific version
workspace.sync.pull(version="checkpoint-v1")
```

### Cross-Region Sync

```bash
# Primary in US
turso db create primary-db --location iad

# Backup replica in EU
turso db replicate primary-db lhr

# AgentFS syncs to primary, replicated to EU
agentfs sync enable my-workspace \
  --turso-db libsql://primary-db-org.turso.io
```

## Performance Optimization

### Batch Sync

```python
# Configure batch size
workspace.sync.config(
    mode='periodic',
    interval=60,
    batch_size=100  # Sync 100 files at a time
)
```

### Selective Sync

```python
# Only sync specific file types
workspace.sync.config(
    include=['*.py', '*.md'],
    exclude=['*.pyc', '__pycache__/', '.git/']
)
```

### Compression

```bash
# Enable compression for large files
agentfs sync config my-workspace --compression true
```

## Monitoring

### Sync Status

```bash
# Check sync status
agentfs sync status my-workspace

# Output:
# Workspace: my-workspace
# Mode: real-time
# Last sync: 2024-01-15T10:30:00Z
# Pending changes: 0
# Sync status: synced
```

### SDK Status

**Python:**
```python
status = workspace.sync.status()
print(f"Last sync: {status.last_sync}")
print(f"Pending: {status.pending_changes}")
print(f"Conflicts: {status.pending_conflicts}")
```

### Metrics

```bash
# Get sync metrics
agentfs sync metrics my-workspace

# Output:
# Sync operations: 1,234
# Data synced: 45.6 MB
# Average latency: 120ms
# Conflicts resolved: 3
```

## Troubleshooting

### Sync Failures

```bash
# Check network connectivity
curl -I https://mydb-org.turso.io/health

# Verify token
agentfs sync verify my-workspace

# Reset sync state
agentfs sync reset my-workspace
```

### Large File Issues

```bash
# Increase timeout for large files
agentfs sync config my-workspace --timeout 300

# Use compression
agentfs sync config my-workspace --compression true
```

### Conflict Resolution

```bash
# View conflicts
agentfs sync conflicts my-workspace

# Resolve all with local
agentfs sync resolve my-workspace --strategy local

# Resolve all with remote
agentfs sync resolve my-workspace --strategy remote

# Interactive resolution
agentfs sync resolve my-workspace --interactive
```

## CLI Reference

```bash
# Enable/disable sync
agentfs sync enable <workspace> [options]
agentfs sync disable <workspace>

# Manual sync
agentfs sync push <workspace>
agentfs sync pull <workspace>

# Configuration
agentfs sync config <workspace> [options]
agentfs sync status <workspace>

# Conflict resolution
agentfs sync conflicts <workspace>
agentfs sync resolve <workspace> [options]

# Troubleshooting
agentfs sync verify <workspace>
agentfs sync reset <workspace>
agentfs sync metrics <workspace>
```

## Next Steps

- [NFS Export](./09-nfs-export.md)
- [Security](./10-security.md)
- [Turso Cloud Integration](../../turso-cloud/01-overview.md)