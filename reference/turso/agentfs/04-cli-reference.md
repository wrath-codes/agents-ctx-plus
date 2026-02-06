# CLI Reference

## Global Options

These options apply to all commands:

```bash
agentfs [GLOBAL OPTIONS] <command> [command options]

Global Options:
  -c, --config <path>     Path to config file
  -v, --verbose           Enable verbose output
  -q, --quiet             Suppress output
  --version               Show version
  -h, --help              Show help
```

## Core Commands

### agentfs init
Initialize AgentFS in a directory.

```bash
agentfs init [OPTIONS]

Options:
  --setup                 Run interactive setup wizard
  --base <path>           Set base directory (default: current)
  --force                 Reinitialize even if already initialized
```

**Examples:**
```bash
# Initialize in current directory
agentfs init

# Initialize with setup wizard
agentfs init --setup

# Force reinitialize
agentfs init --force
```

### agentfs run
Execute a command in an isolated workspace.

```bash
agentfs run [OPTIONS] -- <command> [args...]

Options:
  -w, --workspace <name>  Workspace name (required)
  --base <path>           Base directory (default: current)
  --snapshot <name>       Run in snapshot instead of workspace
  --no-audit              Disable audit logging
  --env <key=value>       Set environment variables
  --workdir <path>        Set working directory in workspace
```

**Examples:**
```bash
# Run command in workspace
agentfs run --workspace my-workspace -- ./build.sh

# Run with environment variables
agentfs run --workspace my-workspace \
  --env API_KEY=secret \
  --env DEBUG=1 \
  -- python script.py

# Run in specific working directory
agentfs run --workspace my-workspace \
  --workdir /src \
  -- make test
```

### agentfs status
Show workspace status and changes.

```bash
agentfs status [OPTIONS]

Options:
  -w, --workspace <name>  Workspace name (required)
  --base <path>           Base directory
  --porcelain             Machine-readable output
  --short                 Short format
  --ignored               Show ignored files
```

**Examples:**
```bash
# Show workspace status
agentfs status --workspace my-workspace

# Machine-readable output
agentfs status --workspace my-workspace --porcelain

# Short format
agentfs status --workspace my-workspace --short
```

**Output Format:**
```
 M  modified.txt          # Modified
 A  added.txt             # Added
 D  deleted.txt           # Deleted
 R  renamed.txt -> new.txt # Renamed
??  untracked.txt         # Untracked
```

### agentfs commit
Commit workspace changes to base.

```bash
agentfs commit [OPTIONS]

Options:
  -w, --workspace <name>  Workspace name (required)
  -m, --message <text>    Commit message (required)
  --author <name>         Author name
  --include <pattern>     Include only matching files
  --exclude <pattern>     Exclude matching files
  --dry-run               Show what would be committed
```

**Examples:**
```bash
# Commit all changes
agentfs commit --workspace my-workspace -m "Initial implementation"

# Commit specific files
agentfs commit --workspace my-workspace \
  --include "*.py" \
  -m "Python changes"

# Dry run
agentfs commit --workspace my-workspace -m "Test" --dry-run
```

### agentfs diff
Show differences between workspaces or snapshots.

```bash
agentfs diff [OPTIONS] [source] [target]

Options:
  -w, --workspace <name>  Workspace name
  --base <path>           Base directory
  --stat                  Show statistics only
  --name-only             Show only filenames
  --color                 Colorize output
```

**Examples:**
```bash
# Diff workspace against base
agentfs diff --workspace my-workspace

# Diff between workspaces
agentfs diff workspace-a workspace-b

# Diff workspace against snapshot
agentfs diff my-workspace my-workspace/snapshot-name

# Statistics only
agentfs diff --workspace my-workspace --stat
```

## Workspace Commands

### agentfs workspace create
Create a new workspace.

```bash
agentfs workspace create <name> [OPTIONS]

Options:
  --base <path>           Base directory
  --from-snapshot <snap>  Create from snapshot
  --from-workspace <ws>   Create from workspace
  --description <text>    Workspace description
  --tag <tags>            Comma-separated tags
  --agent-id <id>         Associated agent ID
  --read-only             Create read-only workspace
```

**Examples:**
```bash
# Create simple workspace
agentfs workspace create my-workspace

# Create from snapshot
agentfs workspace create my-workspace \
  --from-snapshot stable-v1

# Create with metadata
agentfs workspace create my-workspace \
  --description "Testing new feature" \
  --tag "experiment,feature-x" \
  --agent-id "agent-123"
```

### agentfs workspace list
List all workspaces.

```bash
agentfs workspace list [OPTIONS]

Options:
  --base <path>           Base directory
  --format <format>       Output format: table, json, csv
  --filter <filter>       Filter by name pattern
  --tag <tag>             Filter by tag
  --older-than <days>     Filter by age
  --show-archived         Include archived workspaces
```

**Examples:**
```bash
# List all workspaces
agentfs workspace list

# JSON output
agentfs workspace list --format json

# Filter by tag
agentfs workspace list --tag experiment

# Find old workspaces
agentfs workspace list --older-than 30
```

### agentfs workspace show
Show workspace details.

```bash
agentfs workspace show <name> [OPTIONS]

Options:
  --base <path>           Base directory
  --stats                 Show detailed statistics
  --tree                  Show file tree
```

### agentfs workspace delete
Delete a workspace.

```bash
agentfs workspace delete <name> [OPTIONS]

Options:
  --base <path>           Base directory
  -y, --yes               Skip confirmation
  --force                 Force delete even with uncommitted changes
  --older-than <days>     Delete workspaces older than N days
```

**Examples:**
```bash
# Delete workspace
agentfs workspace delete my-workspace

# Force delete
agentfs workspace delete my-workspace --force --yes

# Delete old workspaces
agentfs workspace delete --older-than 30 --yes
```

### agentfs workspace rename
Rename a workspace.

```bash
agentfs workspace rename <old-name> <new-name>
```

### agentfs workspace config
Configure workspace settings.

```bash
agentfs workspace config <name> [OPTIONS]

Options:
  --description <text>    Set description
  --tag <tags>            Set tags
  --read-only <bool>      Set read-only mode
  --expires <datetime>    Set expiration date
  --agent-id <id>         Set agent ID
```

## Snapshot Commands

### agentfs snapshot create
Create a snapshot of workspace state.

```bash
agentfs snapshot create <workspace> [OPTIONS]

Options:
  -n, --name <name>       Snapshot name (required)
  -m, --message <text>    Snapshot description
  --tag <tags>            Comma-separated tags
```

**Example:**
```bash
agentfs snapshot create my-workspace \
  --name "before-major-change" \
  --message "Stable state before refactoring"
```

### agentfs snapshot list
List snapshots for a workspace.

```bash
agentfs snapshot list <workspace> [OPTIONS]

Options:
  --format <format>       Output format
  --tag <tag>             Filter by tag
```

### agentfs snapshot restore
Restore workspace to snapshot.

```bash
agentfs snapshot restore <workspace> <snapshot>

Options:
  --force                 Force restore (discard uncommitted changes)
```

### agentfs snapshot delete
Delete a snapshot.

```bash
agentfs snapshot delete <workspace> <snapshot>

Options:
  -y, --yes               Skip confirmation
```

### agentfs snapshot diff
Compare two snapshots.

```bash
agentfs snapshot diff <workspace> <snapshot-a> <snapshot-b>

Options:
  --stat                  Show statistics only
```

## Audit Commands

### agentfs audit
Show audit log for workspace.

```bash
agentfs audit <workspace> [OPTIONS]

Options:
  --from <datetime>       Start time
  --to <datetime>         End time
  --operation <op>        Filter by operation (create, read, write, delete)
  --path <pattern>        Filter by path
  --agent-id <id>         Filter by agent ID
  --format <format>       Output format: table, json, csv
  --output <file>         Write to file
```

**Examples:**
```bash
# Full audit log
agentfs audit my-workspace

# Filter by operation
agentfs audit my-workspace --operation write

# Time range
agentfs audit my-workspace \
  --from "2024-01-15T00:00:00Z" \
  --to "2024-01-15T23:59:59Z"

# Export to JSON
agentfs audit my-workspace --format json > audit.json
```

## Sync Commands

### agentfs sync enable
Enable cloud sync for workspace.

```bash
agentfs sync enable <workspace> [OPTIONS]

Options:
  --turso-db <url>        Turso database URL
  --token <token>         Turso auth token
  --mode <mode>           Sync mode: real-time, periodic, manual
  --interval <seconds>    Sync interval (for periodic mode)
```

**Example:**
```bash
agentfs sync enable my-workspace \
  --turso-db libsql://mydb-org.turso.io \
  --token $TURSO_TOKEN \
  --mode real-time
```

### agentfs sync disable
Disable cloud sync.

```bash
agentfs sync disable <workspace>
```

### agentfs sync push
Push local changes to cloud.

```bash
agentfs sync push <workspace>

Options:
  --force                 Force push (overwrite remote)
```

### agentfs sync pull
Pull changes from cloud.

```bash
agentfs sync pull <workspace>

Options:
  --force                 Force pull (overwrite local)
```

### agentfs sync status
Show sync status.

```bash
agentfs sync status <workspace>
```

## Utility Commands

### agentfs storage
Storage management commands.

```bash
agentfs storage <subcommand>

Subcommands:
  usage                   Show storage usage
  analyze                 Analyze storage efficiency
  compact                 Compact database
```

### agentfs gc
Run garbage collection.

```bash
agentfs gc [OPTIONS]

Options:
  --dry-run               Show what would be deleted
  -y, --yes               Skip confirmation
```

### agentfs config
Manage configuration.

```bash
agentfs config <subcommand>

Subcommands:
  get <key>               Get configuration value
  set <key> <value>       Set configuration value
  list                    List all configuration

Common keys:
  cache-size              Cache size in MB
  max-workspaces          Maximum workspaces
  default-sync-mode       Default sync mode
  audit-retention         Audit log retention days
```

**Examples:**
```bash
# Get config
agentfs config get cache-size

# Set config
agentfs config set cache-size 500

# List all
agentfs config list
```

### agentfs completions
Generate shell completions.

```bash
agentfs completions <shell>

Shells: bash, zsh, fish, powershell
```

### agentfs version
Show version information.

```bash
agentfs version [OPTIONS]

Options:
  --verbose               Show detailed version info
```

## Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Success |
| 1 | General error |
| 2 | Invalid arguments |
| 3 | Workspace not found |
| 4 | Permission denied |
| 5 | Network error |
| 6 | Sync conflict |
| 10 | Command in workspace failed |

## Environment Variables

| Variable | Description |
|----------|-------------|
| `AGENTFS_CONFIG` | Path to config file |
| `AGENTFS_BASE` | Default base directory |
| `AGENTFS_WORKSPACE` | Default workspace |
| `TURSO_TOKEN` | Turso API token |
| `AGENTFS_LOG_LEVEL` | Logging level (error, warn, info, debug) |

## Next Steps

- **Configuration**: [05-configuration.md](./05-configuration.md)
- **SDKs**: [06-sdks/](./06-sdks/)
- **MCP Integration**: [07-mcp-integration.md](./07-mcp-integration.md)