# Python SDK

## Installation

```bash
pip install agentfs
```

### Requirements
- Python 3.8+
- SQLite 3.35+

## Quick Start

```python
from agentfs import AgentFS

# Initialize AgentFS
afs = AgentFS("/path/to/project")

# Create workspace
workspace = afs.workspace.create("my-workspace")

# Run command
result = workspace.run("echo Hello from AgentFS")
print(result.stdout)

# Commit changes
workspace.commit("Initial setup")
```

## Core Classes

### AgentFS

Main entry point for the SDK.

```python
from agentfs import AgentFS

# Initialize
afs = AgentFS("/path/to/project")

# Or with options
afs = AgentFS(
    base_path="/path/to/project",
    config={
        "cache_size": 256,
        "audit_enabled": True
    }
)
```

### Workspace

Manage isolated workspaces.

```python
# Create workspace
workspace = afs.workspace.create("my-workspace")

# Create from snapshot
workspace = afs.workspace.create(
    "my-workspace",
    from_snapshot="checkpoint-v1"
)

# Get existing workspace
workspace = afs.workspace.get("my-workspace")

# List workspaces
workspaces = afs.workspace.list()
for ws in workspaces:
    print(f"{ws.name}: {ws.description}")

# Delete workspace
afs.workspace.delete("my-workspace")
```

### Running Commands

```python
# Simple command
result = workspace.run("ls -la")
print(result.returncode)
print(result.stdout)
print(result.stderr)

# With environment variables
result = workspace.run(
    "python script.py",
    env={"API_KEY": "secret", "DEBUG": "1"}
)

# With working directory
result = workspace.run(
    "make test",
    workdir="/src"
)

# Capture output
result = workspace.run(
    "./long-running-task",
    capture_output=True,
    timeout=300
)

# Streaming output
for line in workspace.run_stream("./build.sh"):
    print(line, end="")
```

### File Operations

```python
# Read file
content = workspace.read_file("/path/to/file.txt")

# Write file
workspace.write_file("/path/to/file.txt", "Hello, World!")

# Check if exists
if workspace.exists("/path/to/file.txt"):
    print("File exists")

# List directory
files = workspace.list_dir("/src")
for file in files:
    print(f"{file.name} ({file.size} bytes)")

# Copy file
workspace.copy("/src/old.txt", "/src/new.txt")

# Move file
workspace.move("/src/temp.txt", "/dst/final.txt")

# Delete file
workspace.delete("/path/to/file.txt")

# Get file info
info = workspace.stat("/path/to/file.txt")
print(f"Size: {info.size}, Modified: {info.mtime}")
```

### Snapshots

```python
# Create snapshot
snapshot = workspace.snapshot.create(
    "checkpoint",
    description="Before major refactor"
)

# List snapshots
snapshots = workspace.snapshot.list()
for snap in snapshots:
    print(f"{snap.name}: {snap.created_at}")

# Restore snapshot
workspace.snapshot.restore("checkpoint")

# Delete snapshot
workspace.snapshot.delete("checkpoint")

# Compare snapshots
diff = workspace.snapshot.diff("checkpoint-v1", "checkpoint-v2")
for change in diff:
    print(f"{change.type}: {change.path}")
```

### Status and Diff

```python
# Get workspace status
status = workspace.status()
for file in status.modified:
    print(f"Modified: {file.path}")
for file in status.added:
    print(f"Added: {file.path}")
for file in status.deleted:
    print(f"Deleted: {file.path}")

# Show diff
diff = workspace.diff()
print(diff.text)

# Diff against specific snapshot
diff = workspace.diff(against="checkpoint")
```

### Commit

```python
# Commit all changes
workspace.commit("Implemented feature X")

# Commit with author
workspace.commit(
    "Fixed bug in parser",
    author="Developer <dev@example.com>"
)

# Commit specific files
workspace.commit(
    "Updated documentation",
    include=["*.md", "docs/**"]
)

# Dry run
changes = workspace.commit(
    "Test commit",
    dry_run=True
)
print(f"Would commit {len(changes)} files")
```

## Audit Logging

```python
# Get audit log
logs = workspace.audit.logs()

# Filter by operation
logs = workspace.audit.logs(operation="write")

# Filter by time range
from datetime import datetime, timedelta
logs = workspace.audit.logs(
    from_time=datetime.now() - timedelta(hours=24),
    to_time=datetime.now()
)

# Filter by path
logs = workspace.audit.logs(path="/src/main.py")

# Export to file
workspace.audit.export("/path/to/audit.json", format="json")

# Process logs
for entry in logs:
    print(f"{entry.timestamp}: {entry.operation} {entry.path}")
    if entry.details:
        print(f"  Size: {entry.details.size_after}")
```

## Cloud Sync

```python
# Enable sync
workspace.sync.enable(
    turso_db="libsql://mydb-org.turso.io",
    token="your-auth-token"
)

# Configure sync mode
workspace.sync.config(mode="real-time")

# Manual sync
workspace.sync.push()
workspace.sync.pull()

# Check sync status
status = workspace.sync.status()
print(f"Last sync: {status.last_sync}")
print(f"Pending changes: {status.pending_changes}")

# Disable sync
workspace.sync.disable()
```

## Configuration

```python
# Load configuration
from agentfs import Config

config = Config.load("/path/to/config.toml")

# Or create programmatically
config = Config({
    "cache_size": 512,
    "audit_enabled": True,
    "sync": {
        "default_mode": "periodic",
        "interval": 300
    }
})

# Apply to AgentFS instance
afs = AgentFS("/path/to/project", config=config)
```

## Event Hooks

```python
# Define hooks
def on_workspace_create(workspace):
    print(f"Created workspace: {workspace.name}")

def on_commit(workspace, message):
    print(f"Committed: {message}")

# Register hooks
afs.hooks.on_workspace_create = on_workspace_create
afs.hooks.on_commit = on_commit

# Or use decorator
@afs.hooks.workspace_create
def my_hook(workspace):
    send_notification(f"Workspace {workspace.name} created")
```

## Error Handling

```python
from agentfs import (
    AgentFS,
    WorkspaceNotFoundError,
    WorkspaceExistsError,
    SnapshotNotFoundError,
    SyncError,
    CommitError
)

try:
    workspace = afs.workspace.create("existing-workspace")
except WorkspaceExistsError:
    print("Workspace already exists")
    workspace = afs.workspace.get("existing-workspace")

try:
    workspace.snapshot.restore("nonexistent")
except SnapshotNotFoundError:
    print("Snapshot not found")

try:
    workspace.sync.push()
except SyncError as e:
    print(f"Sync failed: {e}")
    if e.conflicts:
        for conflict in e.conflicts:
            print(f"Conflict: {conflict.path}")
```

## Advanced Usage

### Batch Operations

```python
# Batch file operations
with workspace.batch() as batch:
    batch.write_file("/file1.txt", "content1")
    batch.write_file("/file2.txt", "content2")
    batch.delete("/old-file.txt")
# All operations applied atomically

# Batch with callback
for result in workspace.batch_process(files, callback=process_file):
    print(f"Processed: {result.path}")
```

### Streaming

```python
# Stream file content
with workspace.open_stream("/large-file.bin", "rb") as stream:
    while chunk := stream.read(8192):
        process_chunk(chunk)

# Stream command output
async for line in workspace.run_stream_async("./build.sh"):
    print(line, end="")
```

### Context Managers

```python
# Auto-cleanup workspace
with afs.workspace.temp("temp-task") as workspace:
    workspace.run("./task.sh")
    # Workspace automatically deleted on exit

# Temporary snapshot
with workspace.snapshot.temp() as snapshot:
    workspace.run("./risky-operation.sh")
    if not success:
        workspace.snapshot.restore(snapshot.name)
```

## Testing

```python
import pytest
from agentfs import AgentFS
from agentfs.testing import TemporaryWorkspace

@pytest.fixture
def agentfs(tmp_path):
    return AgentFS(str(tmp_path))

@pytest.fixture
def workspace(agentfs):
    return agentfs.workspace.create("test-workspace")

def test_workspace_creation(workspace):
    assert workspace.name == "test-workspace"
    assert workspace.exists()

def test_file_operations(workspace):
    workspace.write_file("/test.txt", "Hello")
    assert workspace.read_file("/test.txt") == "Hello"

def test_command_execution(workspace):
    result = workspace.run("echo test")
    assert result.stdout.strip() == "test"
    assert result.returncode == 0

# Use TemporaryWorkspace for isolated tests
@ TemporaryWorkspace()
def test_isolated_operations(workspace):
    # Workspace is temporary and will be cleaned up
    workspace.write_file("/data.txt", "data")
    assert workspace.exists("/data.txt")
```

## Type Hints

```python
from agentfs import AgentFS, Workspace, Snapshot
from typing import List, Optional

def process_workspace(workspace: Workspace) -> List[str]:
    """Process workspace and return modified files."""
    status = workspace.status()
    return [f.path for f in status.modified]

def create_snapshot(workspace: Workspace, name: str) -> Optional[Snapshot]:
    """Create snapshot if workspace has changes."""
    if workspace.status().has_changes:
        return workspace.snapshot.create(name)
    return None
```

## Async Support

```python
import asyncio
from agentfs import AgentFS

async def main():
    afs = AgentFS("/path/to/project")
    
    # Create workspace
    workspace = await afs.workspace.create_async("my-workspace")
    
    # Run command
    result = await workspace.run_async("./build.sh")
    
    # File operations
    await workspace.write_file_async("/file.txt", "content")
    content = await workspace.read_file_async("/file.txt")
    
    # Sync
    await workspace.sync.push_async()

asyncio.run(main())
```

## Best Practices

1. **Use context managers** for automatic cleanup
2. **Handle errors explicitly** for better debugging
3. **Commit frequently** with descriptive messages
4. **Use snapshots** before risky operations
5. **Enable audit logging** for production use
6. **Configure sync** for multi-agent scenarios

## API Reference

See [API documentation](https://docs.agentfs.dev/python) for complete reference.

## Next Steps

- [Rust SDK](./rust-sdk.md)
- [TypeScript SDK](./typescript-sdk.md)
- [MCP Integration](../07-mcp-integration.md)