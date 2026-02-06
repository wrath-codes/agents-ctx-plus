# TypeScript SDK

## Installation

```bash
npm install @agentfs/sdk
# or
yarn add @agentfs/sdk
# or
pnpm add @agentfs/sdk
```

## Quick Start

```typescript
import { AgentFS } from '@agentfs/sdk';

async function main() {
    // Initialize AgentFS
    const afs = new AgentFS('/path/to/project');
    
    // Create workspace
    const workspace = await afs.workspace.create('my-workspace');
    
    // Run command
    const result = await workspace.run('echo Hello from AgentFS');
    console.log(result.stdout);
    
    // Commit changes
    await workspace.commit('Initial setup');
}

main().catch(console.error);
```

## Core Classes

### AgentFS

Main entry point.

```typescript
import { AgentFS } from '@agentfs/sdk';

// Initialize
const afs = new AgentFS('/path/to/project');

// With options
const afs = new AgentFS('/path/to/project', {
    cacheSize: 256,
    auditEnabled: true
});
```

### Workspace

```typescript
// Create workspace
const workspace = await afs.workspace.create('my-workspace');

// Create from snapshot
const workspace = await afs.workspace.create('my-workspace', {
    fromSnapshot: 'checkpoint-v1'
});

// Get existing workspace
const workspace = await afs.workspace.get('my-workspace');

// List workspaces
const workspaces = await afs.workspace.list();
for (const ws of workspaces) {
    console.log(`${ws.name}: ${ws.description}`);
}

// Delete workspace
await afs.workspace.delete('my-workspace');
```

### Running Commands

```typescript
// Simple command
const result = await workspace.run('ls -la');
console.log(result.exitCode);
console.log(result.stdout);
console.log(result.stderr);

// With environment variables
const result = await workspace.run('python script.py', {
    env: { API_KEY: 'secret', DEBUG: '1' }
});

// With working directory
const result = await workspace.run('make test', {
    workdir: '/src'
});

// With timeout
const result = await workspace.run('./long-running-task', {
    timeout: 300000 // 5 minutes
});

// Streaming output
const stream = await workspace.runStream('./build.sh');
for await (const line of stream) {
    process.stdout.write(line);
}
```

### File Operations

```typescript
// Read file
const content = await workspace.readFile('/path/to/file.txt');
const text = content.toString('utf-8');

// Write file
await workspace.writeFile('/path/to/file.txt', 'Hello, World!');

// Check if exists
const exists = await workspace.exists('/path/to/file.txt');
if (exists) {
    console.log('File exists');
}

// List directory
const files = await workspace.listDir('/src');
for (const file of files) {
    console.log(`${file.name} (${file.size} bytes)`);
}

// Copy file
await workspace.copy('/src/old.txt', '/src/new.txt');

// Move file
await workspace.move('/src/temp.txt', '/dst/final.txt');

// Delete file
await workspace.delete('/path/to/file.txt');

// Get file info
const info = await workspace.stat('/path/to/file.txt');
console.log(`Size: ${info.size}, Modified: ${info.mtime}`);
```

### Snapshots

```typescript
// Create snapshot
const snapshot = await workspace.snapshot.create('checkpoint', {
    description: 'Before major refactor'
});

// List snapshots
const snapshots = await workspace.snapshot.list();
for (const snap of snapshots) {
    console.log(`${snap.name}: ${snap.createdAt}`);
}

// Restore snapshot
await workspace.snapshot.restore('checkpoint');

// Delete snapshot
await workspace.snapshot.delete('checkpoint');

// Compare snapshots
const diff = await workspace.snapshot.diff('checkpoint-v1', 'checkpoint-v2');
for (const change of diff.changes) {
    console.log(`${change.type}: ${change.path}`);
}
```

### Status and Diff

```typescript
// Get workspace status
const status = await workspace.status();
for (const file of status.modified) {
    console.log(`Modified: ${file.path}`);
}
for (const file of status.added) {
    console.log(`Added: ${file.path}`);
}
for (const file of status.deleted) {
    console.log(`Deleted: ${file.path}`);
}

// Show diff
const diff = await workspace.diff();
console.log(diff.text);

// Diff against specific snapshot
const diff = await workspace.diff({ against: 'checkpoint' });
```

### Commit

```typescript
// Commit all changes
await workspace.commit('Implemented feature X');

// Commit with author
await workspace.commit('Fixed bug in parser', {
    author: 'Developer <dev@example.com>'
});

// Commit specific files
await workspace.commit('Updated documentation', {
    include: ['*.md', 'docs/**']
});

// Dry run
const changes = await workspace.commit('Test commit', {
    dryRun: true
});
console.log(`Would commit ${changes.length} files`);
```

## Audit Logging

```typescript
// Get audit log
const logs = await workspace.audit.logs();

// Filter by operation
const logs = await workspace.audit.logs({
    operation: 'write'
});

// Filter by time range
const logs = await workspace.audit.logs({
    from: new Date(Date.now() - 24 * 60 * 60 * 1000),
    to: new Date()
});

// Filter by path
const logs = await workspace.audit.logs({
    path: '/src/main.ts'
});

// Export to file
await workspace.audit.export('/path/to/audit.json', {
    format: 'json'
});

// Process logs
for (const entry of logs) {
    console.log(`${entry.timestamp}: ${entry.operation} ${entry.path}`);
    if (entry.details) {
        console.log(`  Size: ${entry.details.sizeAfter}`);
    }
}
```

## Cloud Sync

```typescript
// Enable sync
await workspace.sync.enable({
    tursoDb: 'libsql://mydb-org.turso.io',
    token: 'your-auth-token'
});

// Configure sync mode
await workspace.sync.config({
    mode: 'real-time'
});

// Manual sync
await workspace.sync.push();
await workspace.sync.pull();

// Check sync status
const status = await workspace.sync.status();
console.log(`Last sync: ${status.lastSync}`);
console.log(`Pending changes: ${status.pendingChanges}`);

// Disable sync
await workspace.sync.disable();
```

## Error Handling

```typescript
import { 
    AgentFS, 
    WorkspaceNotFoundError, 
    SnapshotNotFoundError, 
    SyncError,
    CommitError 
} from '@agentfs/sdk';

try {
    const workspace = await afs.workspace.create('existing-workspace');
} catch (error) {
    if (error instanceof WorkspaceNotFoundError) {
        console.log(`Workspace not found: ${error.message}`);
    } else if (error instanceof WorkspaceExistsError) {
        console.log('Workspace already exists');
        const workspace = await afs.workspace.get('existing-workspace');
    }
}

try {
    await workspace.snapshot.restore('nonexistent');
} catch (error) {
    if (error instanceof SnapshotNotFoundError) {
        console.log(`Snapshot not found: ${error.message}`);
    }
}

try {
    await workspace.sync.push();
} catch (error) {
    if (error instanceof SyncError) {
        console.log(`Sync failed: ${error.message}`);
        if (error.conflicts) {
            for (const conflict of error.conflicts) {
                console.log(`Conflict: ${conflict.path}`);
            }
        }
    }
}
```

## Configuration

```typescript
import { AgentFS, Config } from '@agentfs/sdk';

// Load configuration
const config = await Config.fromFile('/path/to/config.toml');

// Create programmatically
const config = new Config({
    cacheSize: 512,
    auditEnabled: true,
    sync: {
        defaultMode: 'periodic',
        interval: 300
    }
});

// Apply to AgentFS
const afs = new AgentFS('/path/to/project', config);
```

## Event Handling

```typescript
// Define event handlers
afs.on('workspaceCreate', (workspace) => {
    console.log(`Created workspace: ${workspace.name}`);
});

afs.on('commit', (workspace, message) => {
    console.log(`Committed: ${message}`);
});

// Once handlers
afs.once('sync', (workspace) => {
    console.log(`First sync for ${workspace.name}`);
});

// Remove handler
const handler = (workspace) => console.log('Created');
afs.on('workspaceCreate', handler);
afs.off('workspaceCreate', handler);
```

## TypeScript Types

```typescript
import { 
    Workspace, 
    Snapshot, 
    AuditEntry,
    FileInfo,
    WorkspaceStatus 
} from '@agentfs/sdk';

async function processWorkspace(workspace: Workspace): Promise<string[]> {
    const status: WorkspaceStatus = await workspace.status();
    return status.modified.map(f => f.path);
}

async function createSnapshot(
    workspace: Workspace, 
    name: string
): Promise<Snapshot | null> {
    const status = await workspace.status();
    if (status.hasChanges) {
        return workspace.snapshot.create(name);
    }
    return null;
}

function formatAuditEntry(entry: AuditEntry): string {
    return `${entry.timestamp}: ${entry.operation} ${entry.path}`;
}
```

## Advanced Usage

### Batch Operations

```typescript
// Batch file operations
await workspace.batch([
    { type: 'write', path: '/file1.txt', content: 'content1' },
    { type: 'write', path: '/file2.txt', content: 'content2' },
    { type: 'delete', path: '/old-file.txt' }
]);
// All operations applied atomically

// Batch with callback
const results = await workspace.batchProcess(files, async (file) => {
    return processFile(file);
});
for (const result of results) {
    console.log(`Processed: ${result.path}`);
}
```

### Streaming

```typescript
// Stream file content
const stream = await workspace.createReadStream('/large-file.bin');
stream.on('data', (chunk) => {
    processChunk(chunk);
});
stream.on('end', () => {
    console.log('Finished reading');
});

// Async iterator
for await (const chunk of workspace.readStream('/large-file.bin')) {
    processChunk(chunk);
}
```

### Async Generators

```typescript
// Stream audit log
for await (const entry of workspace.audit.stream()) {
    console.log(`${entry.timestamp}: ${entry.operation}`);
}

// Stream command output
for await (const line of workspace.runStream('./build.sh')) {
    process.stdout.write(line);
}
```

## Testing

```typescript
import { AgentFS } from '@agentfs/sdk';
import { TemporaryWorkspace } from '@agentfs/sdk/testing';
import { describe, it, expect, beforeEach } from 'vitest';
import { tmpdir } from 'os';
import { mkdtemp } from 'fs/promises';
import { join } from 'path';

describe('AgentFS', () => {
    let afs: AgentFS;
    let tmpDir: string;
    
    beforeEach(async () => {
        tmpDir = await mkdtemp(join(tmpdir(), 'agentfs-'));
        afs = new AgentFS(tmpDir);
    });
    
    it('should create workspace', async () => {
        const workspace = await afs.workspace.create('test');
        expect(workspace.name).toBe('test');
    });
    
    it('should handle file operations', async () => {
        const workspace = await afs.workspace.create('test');
        await workspace.writeFile('/test.txt', 'Hello');
        const content = await workspace.readFile('/test.txt');
        expect(content.toString()).toBe('Hello');
    });
    
    it('should execute commands', async () => {
        const workspace = await afs.workspace.create('test');
        const result = await workspace.run('echo test');
        expect(result.exitCode).toBe(0);
        expect(result.stdout.trim()).toBe('test');
    });
});

// Use TemporaryWorkspace for isolated tests
describe('TemporaryWorkspace', () => {
    it('should auto-cleanup', async () => {
        const tmpWs = await TemporaryWorkspace.create();
        const workspace = tmpWs.workspace;
        
        await workspace.writeFile('/data.txt', 'data');
        expect(await workspace.exists('/data.txt')).toBe(true);
        
        await tmpWs.cleanup();
        // Workspace automatically cleaned up
    });
});
```

## Best Practices

1. **Use async/await** for all operations
2. **Handle errors explicitly** for better debugging
3. **Use TypeScript types** for compile-time safety
4. **Commit frequently** with descriptive messages
5. **Use snapshots** before risky operations
6. **Enable audit logging** for production use

## API Reference

See [API documentation](https://docs.agentfs.dev/typescript) for complete reference.

## Next Steps

- [Python SDK](./python-sdk.md)
- [Rust SDK](./rust-sdk.md)
- [MCP Integration](../07-mcp-integration.md)