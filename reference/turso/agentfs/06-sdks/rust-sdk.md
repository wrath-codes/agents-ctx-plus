# Rust SDK

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
agentfs = "0.1"
tokio = { version = "1", features = ["full"] }
```

## Quick Start

```rust
use agentfs::AgentFS;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize AgentFS
    let afs = AgentFS::new("/path/to/project").await?;
    
    // Create workspace
    let workspace = afs.workspace().create("my-workspace").await?;
    
    // Run command
    let result = workspace.run("echo Hello from AgentFS").await?;
    println!("{}", result.stdout);
    
    // Commit changes
    workspace.commit("Initial setup").await?;
    
    Ok(())
}
```

## Core Types

### AgentFS

Main entry point.

```rust
use agentfs::AgentFS;

// Initialize
let afs = AgentFS::new("/path/to/project").await?;

// With configuration
let afs = AgentFS::builder("/path/to/project")
    .cache_size(256)
    .audit_enabled(true)
    .build()
    .await?;
```

### Workspace

```rust
// Create workspace
let workspace = afs.workspace().create("my-workspace").await?;

// Create from snapshot
let workspace = afs.workspace()
    .create("my-workspace")
    .from_snapshot("checkpoint-v1")
    .await?;

// Get existing workspace
let workspace = afs.workspace().get("my-workspace").await?;

// List workspaces
let workspaces = afs.workspace().list().await?;
for ws in workspaces {
    println!("{}: {}", ws.name(), ws.description());
}

// Delete workspace
afs.workspace().delete("my-workspace").await?;
```

### Running Commands

```rust
// Simple command
let result = workspace.run("ls -la").await?;
println!("Exit code: {}", result.status.code().unwrap_or(-1));
println!("Stdout: {}", result.stdout);
println!("Stderr: {}", result.stderr);

// With environment
let result = workspace
    .run("python script.py")
    .env("API_KEY", "secret")
    .env("DEBUG", "1")
    .await?;

// With working directory
let result = workspace
    .run("make test")
    .current_dir("/src")
    .await?;

// With timeout
let result = workspace
    .run("./long-task")
    .timeout(Duration::from_secs(300))
    .await?;

// Streaming output
let mut child = workspace
    .run("./build.sh")
    .stdout(std::process::Stdio::piped())
    .spawn()?;

if let Some(stdout) = child.stdout.take() {
    let reader = BufReader::new(stdout);
    for line in reader.lines() {
        println!("{}", line?);
    }
}
```

### File Operations

```rust
use tokio::fs;

// Read file
let content = workspace.read_file("/path/to/file.txt").await?;
let text = String::from_utf8(content)?;

// Write file
workspace.write_file("/path/to/file.txt", b"Hello, World!").await?;

// Check if exists
if workspace.exists("/path/to/file.txt").await? {
    println!("File exists");
}

// List directory
let entries = workspace.read_dir("/src").await?;
for entry in entries {
    println!("{} ({} bytes)", entry.name(), entry.size());
}

// Copy file
workspace.copy("/src/old.txt", "/src/new.txt").await?;

// Move file
workspace.rename("/src/temp.txt", "/dst/final.txt").await?;

// Delete file
workspace.remove_file("/path/to/file.txt").await?;

// Get metadata
let metadata = workspace.metadata("/path/to/file.txt").await?;
println!("Size: {}, Modified: {:?}", metadata.size(), metadata.modified());
```

### Snapshots

```rust
// Create snapshot
workspace.snapshot()
    .create("checkpoint")
    .description("Before major refactor")
    .await?;

// List snapshots
let snapshots = workspace.snapshot().list().await?;
for snap in snapshots {
    println!("{}: {:?}", snap.name(), snap.created_at());
}

// Restore snapshot
workspace.snapshot().restore("checkpoint").await?;

// Delete snapshot
workspace.snapshot().delete("checkpoint").await?;

// Compare snapshots
let diff = workspace.snapshot()
    .diff("checkpoint-v1", "checkpoint-v2")
    .await?;
for change in diff.changes() {
    println!("{:?}: {}", change.kind(), change.path());
}
```

### Status and Diff

```rust
// Get workspace status
let status = workspace.status().await?;
for file in status.modified() {
    println!("Modified: {}", file.path());
}
for file in status.added() {
    println!("Added: {}", file.path());
}
for file in status.deleted() {
    println!("Deleted: {}", file.path());
}

// Show diff
let diff = workspace.diff().await?;
println!("{}", diff.text());

// Diff against snapshot
let diff = workspace.diff_against("checkpoint").await?;
```

### Commit

```rust
// Commit all changes
workspace.commit("Implemented feature X").await?;

// Commit with author
workspace.commit("Fixed bug in parser")
    .author("Developer", "dev@example.com")
    .await?;

// Commit specific files
workspace.commit("Updated documentation")
    .include("*.md")
    .include("docs/**")
    .await?;

// Dry run
let changes = workspace.commit("Test commit")
    .dry_run()
    .await?;
println!("Would commit {} files", changes.len());
```

## Audit Logging

```rust
// Get audit log
let logs = workspace.audit().logs().await?;

// Filter by operation
let logs = workspace.audit()
    .logs()
    .operation(OperationType::Write)
    .await?;

// Filter by time range
use chrono::{Duration, Utc};
let logs = workspace.audit()
    .logs()
    .from(Utc::now() - Duration::hours(24))
    .to(Utc::now())
    .await?;

// Filter by path
let logs = workspace.audit()
    .logs()
    .path("/src/main.rs")
    .await?;

// Process logs
for entry in logs {
    println!("{}: {:?} {}", 
        entry.timestamp(), 
        entry.operation(), 
        entry.path()
    );
    if let Some(details) = entry.details() {
        println!("  Size: {:?}", details.size_after());
    }
}
```

## Cloud Sync

```rust
// Enable sync
workspace.sync()
    .enable("libsql://mydb-org.turso.io", "your-auth-token")
    .await?;

// Configure sync mode
workspace.sync()
    .config()
    .mode(SyncMode::RealTime)
    .await?;

// Manual sync
workspace.sync().push().await?;
workspace.sync().pull().await?;

// Check sync status
let status = workspace.sync().status().await?;
println!("Last sync: {:?}", status.last_sync());
println!("Pending changes: {}", status.pending_changes());

// Disable sync
workspace.sync().disable().await?;
```

## Error Handling

```rust
use agentfs::{Error, WorkspaceError, SnapshotError, SyncError};

match afs.workspace().create("existing-workspace").await {
    Ok(workspace) => { /* use workspace */ },
    Err(Error::Workspace(WorkspaceError::AlreadyExists(name))) => {
        eprintln!("Workspace {} already exists", name);
        let workspace = afs.workspace().get(&name).await?;
    }
    Err(e) => return Err(e.into()),
}

match workspace.snapshot().restore("nonexistent").await {
    Ok(_) => { /* success */ },
    Err(Error::Snapshot(SnapshotError::NotFound(name))) => {
        eprintln!("Snapshot {} not found", name);
    }
    Err(e) => return Err(e.into()),
}

match workspace.sync().push().await {
    Ok(_) => { /* success */ },
    Err(Error::Sync(SyncError::Conflict(conflicts))) => {
        eprintln!("Sync conflicts:");
        for conflict in conflicts {
            eprintln!("  {}", conflict.path());
        }
    }
    Err(e) => return Err(e.into()),
}
```

## Configuration

```rust
use agentfs::Config;

// Load from file
let config = Config::from_file("/path/to/config.toml").await?;

// Build programmatically
let config = Config::builder()
    .cache_size(512)
    .audit_enabled(true)
    .sync_mode(SyncMode::Periodic)
    .sync_interval(Duration::from_secs(300))
    .build();

// Apply to AgentFS
let afs = AgentFS::builder("/path/to/project")
    .config(config)
    .build()
    .await?;
```

## Builder Pattern

```rust
// Workspace with options
let workspace = afs.workspace()
    .create("my-workspace")
    .description("Testing new feature")
    .tag("experiment")
    .tag("feature-x")
    .agent_id("agent-123")
    .await?;

// Snapshot with options
workspace.snapshot()
    .create("checkpoint")
    .description("Before major refactor")
    .tag("stable")
    .await?;

// Commit with options
workspace.commit("Implemented feature")
    .author("Developer", "dev@example.com")
    .include("*.rs")
    .exclude("target/**")
    .await?;
```

## Streaming

```rust
use tokio::io::{AsyncReadExt, AsyncWriteExt};

// Stream file content
let mut file = workspace.open("/large-file.bin").await?;
let mut buffer = vec![0u8; 8192];
loop {
    let n = file.read(&mut buffer).await?;
    if n == 0 {
        break;
    }
    process_chunk(&buffer[..n]);
}

// Stream command output
let mut child = workspace
    .run("./build.sh")
    .stdout(std::process::Stdio::piped())
    .spawn()?;

if let Some(stdout) = child.stdout.take() {
    let mut reader = BufReader::new(stdout);
    let mut line = String::new();
    while reader.read_line(&mut line).await? > 0 {
        print!("{}", line);
        line.clear();
    }
}
```

## Testing

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use agentfs::testing::TempWorkspace;
    use tempfile::TempDir;

    async fn setup() -> (TempDir, AgentFS) {
        let tmp = TempDir::new().unwrap();
        let afs = AgentFS::new(tmp.path()).await.unwrap();
        (tmp, afs)
    }

    #[tokio::test]
    async fn test_workspace_creation() {
        let (_tmp, afs) = setup().await;
        let workspace = afs.workspace().create("test").await.unwrap();
        assert_eq!(workspace.name(), "test");
    }

    #[tokio::test]
    async fn test_file_operations() {
        let (_tmp, afs) = setup().await;
        let workspace = afs.workspace().create("test").await.unwrap();
        
        workspace.write_file("/test.txt", b"Hello").await.unwrap();
        let content = workspace.read_file("/test.txt").await.unwrap();
        assert_eq!(content, b"Hello");
    }

    #[tokio::test]
    async fn test_command_execution() {
        let (_tmp, afs) = setup().await;
        let workspace = afs.workspace().create("test").await.unwrap();
        
        let result = workspace.run("echo test").await.unwrap();
        assert!(result.status.success());
        assert_eq!(result.stdout.trim(), "test");
    }

    // Use TempWorkspace for isolated tests
    #[tokio::test]
    async fn test_with_temp_workspace() {
        let tmp_workspace = TempWorkspace::new().await.unwrap();
        let workspace = tmp_workspace.workspace();
        
        workspace.write_file("/data.txt", b"data").await.unwrap();
        assert!(workspace.exists("/data.txt").await.unwrap());
        // Workspace automatically cleaned up
    }
}
```

## Type Safety

```rust
use agentfs::{Workspace, Snapshot};

async fn process_workspace(workspace: &Workspace) -> Vec<String> {
    let status = workspace.status().await.unwrap();
    status.modified()
        .iter()
        .map(|f| f.path().to_string())
        .collect()
}

async fn create_snapshot(
    workspace: &Workspace, 
    name: &str
) -> Option<Snapshot> {
    let status = workspace.status().await.unwrap();
    if status.has_changes() {
        Some(workspace.snapshot().create(name).await.unwrap())
    } else {
        None
    }
}
```

## Best Practices

1. **Use `?` operator** for error propagation
2. **Leverage builder pattern** for complex operations
3. **Use streaming** for large files
4. **Commit atomically** with meaningful messages
5. **Enable audit logging** in production
6. **Handle sync conflicts** explicitly

## API Reference

See [API documentation](https://docs.agentfs.dev/rust) for complete reference.

## Next Steps

- [Python SDK](./python-sdk.md)
- [TypeScript SDK](./typescript-sdk.md)
- [MCP Integration](../07-mcp-integration.md)