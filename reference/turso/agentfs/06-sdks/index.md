# SDKs Overview

AgentFS provides official SDKs for multiple programming languages, enabling programmatic workspace management and integration with your applications.

## Available SDKs

| Language | Package | Status | Documentation |
|----------|---------|--------|---------------|
| Python | `agentfs` | Stable | [Python SDK](./python-sdk.md) |
| Rust | `agentfs` | Stable | [Rust SDK](./rust-sdk.md) |
| TypeScript | `@agentfs/sdk` | Stable | [TypeScript SDK](./typescript-sdk.md) |

## Installation

### Python
```bash
pip install agentfs
```

### Rust
```toml
[dependencies]
agentfs = "0.1"
```

### TypeScript
```bash
npm install @agentfs/sdk
# or
yarn add @agentfs/sdk
```

## Common Patterns

All SDKs follow similar patterns for core operations:

### 1. Initialization
```python
# Python
from agentfs import AgentFS

afs = AgentFS(base_path="/path/to/project")
```

```rust
// Rust
use agentfs::AgentFS;

let afs = AgentFS::new("/path/to/project").await?;
```

```typescript
// TypeScript
import { AgentFS } from '@agentfs/sdk';

const afs = new AgentFS('/path/to/project');
```

### 2. Workspace Management
```python
# Python
workspace = afs.workspace.create("my-workspace")
workspace.run("./build.sh")
workspace.commit("Build completed")
```

```rust
// Rust
let workspace = afs.workspace().create("my-workspace").await?;
workspace.run("./build.sh").await?;
workspace.commit("Build completed").await?;
```

```typescript
// TypeScript
const workspace = await afs.workspace.create('my-workspace');
await workspace.run('./build.sh');
await workspace.commit('Build completed');
```

### 3. Snapshots
```python
# Python
snapshot = workspace.snapshot.create("checkpoint")
workspace.snapshot.restore("checkpoint")
```

```rust
// Rust
workspace.snapshot().create("checkpoint").await?;
workspace.snapshot().restore("checkpoint").await?;
```

```typescript
// TypeScript
await workspace.snapshot.create('checkpoint');
await workspace.snapshot.restore('checkpoint');
```

### 4. Sync
```python
# Python
workspace.sync.enable(
    turso_db="libsql://mydb-org.turso.io",
    token="your-token"
)
workspace.sync.push()
```

```rust
// Rust
workspace.sync()
    .enable("libsql://mydb-org.turso.io", "your-token")
    .await?;
workspace.sync().push().await?;
```

```typescript
// TypeScript
await workspace.sync.enable({
    tursoDb: 'libsql://mydb-org.turso.io',
    token: 'your-token'
});
await workspace.sync.push();
```

## Feature Comparison

| Feature | Python | Rust | TypeScript |
|---------|--------|------|------------|
| Workspace CRUD | ✅ | ✅ | ✅ |
| Snapshot Management | ✅ | ✅ | ✅ |
| Audit Logging | ✅ | ✅ | ✅ |
| Cloud Sync | ✅ | ✅ | ✅ |
| File Operations | ✅ | ✅ | ✅ |
| Batch Operations | ✅ | ✅ | ✅ |
| Streaming | ✅ | ✅ | ✅ |
| Async/Await | ✅ | ✅ | ✅ |
| Callbacks/Hooks | ✅ | ✅ | ✅ |
| Type Safety | Partial | ✅ | ✅ |

## Error Handling

All SDKs provide structured error handling:

### Python
```python
from agentfs import AgentFS, WorkspaceNotFoundError, SyncError

try:
    workspace = afs.workspace.get("nonexistent")
except WorkspaceNotFoundError as e:
    print(f"Workspace not found: {e}")
except SyncError as e:
    print(f"Sync failed: {e}")
```

### Rust
```rust
use agentfs::{AgentFS, Error};

match afs.workspace().get("nonexistent").await {
    Ok(workspace) => { /* use workspace */ },
    Err(Error::WorkspaceNotFound(name)) => {
        eprintln!("Workspace not found: {}", name);
    }
    Err(e) => {
        eprintln!("Error: {}", e);
    }
}
```

### TypeScript
```typescript
import { AgentFS, WorkspaceNotFoundError, SyncError } from '@agentfs/sdk';

try {
    const workspace = await afs.workspace.get('nonexistent');
} catch (error) {
    if (error instanceof WorkspaceNotFoundError) {
        console.error(`Workspace not found: ${error.message}`);
    } else if (error instanceof SyncError) {
        console.error(`Sync failed: ${error.message}`);
    } else {
        console.error(`Unexpected error: ${error}`);
    }
}
```

## Next Steps

- [Python SDK](./python-sdk.md)
- [Rust SDK](./rust-sdk.md)
- [TypeScript SDK](./typescript-sdk.md)