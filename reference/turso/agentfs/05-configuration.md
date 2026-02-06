# Configuration

## Configuration File

AgentFS uses TOML format for configuration.

### Location
```
~/.config/agentfs/config.toml
```

### Example Configuration
```toml
# AgentFS Configuration

[core]
# Base directory for workspaces (default: current directory)
default_base = "/home/user/projects"

# Maximum number of workspaces (0 = unlimited)
max_workspaces = 100

# Enable audit logging globally
audit_enabled = true

# Audit log retention (days)
audit_retention_days = 90

[storage]
# Cache size for frequently accessed files
cache_size_mb = 256

# Storage backend: sqlite, memory
backend = "sqlite"

# Database file location
database_path = "~/.local/share/agentfs/agentfs.db"

# Enable compression for stored files
compression = true
compression_level = 6

[sync]
# Default sync mode: real-time, periodic, manual
default_mode = "manual"

# Sync interval for periodic mode (seconds)
periodic_interval = 300

# Auto-sync on workspace commit
auto_sync_on_commit = false

# Conflict resolution strategy: local-wins, remote-wins, merge, manual
conflict_resolution = "manual"

[performance]
# Number of worker threads (0 = auto)
worker_threads = 0

# Max concurrent file operations
max_concurrent_ops = 10

# Enable parallel processing for large operations
parallel_processing = true

[security]
# Encrypt local database
encrypt_local_db = true

# Encryption key file (if not set, uses system keychain)
# encryption_key_file = "~/.config/agentfs/key"

# Require confirmation for destructive operations
confirm_destructive = true

# Allow execution of binaries from workspaces
allow_workspace_binaries = true

[logging]
# Log level: error, warn, info, debug, trace
level = "info"

# Log file location (if not set, logs to stderr)
# log_file = "~/.local/share/agentfs/agentfs.log"

# Maximum log file size (MB)
max_log_size_mb = 100

# Maximum number of log files to keep
max_log_files = 5

[ui]
# Default output format: auto, text, json
default_format = "auto"

# Enable colors in output
color = true

# Pager for long output
pager = "less -R"

[network]
# HTTP timeout (seconds)
timeout = 30

# Retry attempts for failed requests
retry_attempts = 3

# Enable HTTP/2
http2 = true
```

## Profiles

Create multiple configuration profiles for different scenarios.

### Profile Configuration
```toml
# ~/.config/agentfs/config.toml

[profile.development]
core.max_workspaces = 50
sync.default_mode = "manual"
performance.parallel_processing = false

[profile.production]
core.max_workspaces = 200
sync.default_mode = "real-time"
logging.level = "warn"
security.encrypt_local_db = true

[profile.ci]
core.audit_enabled = false
ui.color = false
logging.level = "error"
```

### Using Profiles
```bash
# Use profile via CLI
agentfs --profile production workspace list

# Set default profile
agentfs config set --profile production

# Show current profile
agentfs config get profile
```

## Environment-Specific Configuration

### Development Environment
```toml
# .agentfs/config.toml (in project directory)
[core]
default_base = "."
max_workspaces = 10

[sync]
default_mode = "manual"

[logging]
level = "debug"
```

### CI/CD Environment
```yaml
# .github/workflows/test.yml
- name: Configure AgentFS
  run: |
    mkdir -p .agentfs
    cat > .agentfs/config.toml << 'EOF'
    [core]
    audit_enabled = false
    
    [ui]
    color = false
    
    [logging]
    level = "error"
    EOF
```

## Workspace Templates

### Creating Templates
```toml
# ~/.config/agentfs/templates/python-project.toml
name = "python-project"
description = "Template for Python projects"

[workspace]
read_only = false
tags = ["python", "template"]

[ignore]
patterns = [
  "__pycache__/",
  "*.pyc",
  ".pytest_cache/",
  ".venv/",
  "*.egg-info/"
]

[scripts]
init = "pip install -e ."
test = "pytest"
lint = "flake8 src/"
```

### Using Templates
```bash
# Create workspace from template
agentfs workspace create my-project --template python-project

# List available templates
agentfs template list

# Show template details
agentfs template show python-project
```

## Sync Configuration

### Turso Cloud Integration
```toml
[sync.turso]
# Organization slug
organization = "acme-corp"

# Default database for sync
default_database = "agentfs-workspaces"

# Authentication
token = "env:TURSO_TOKEN"  # Read from environment
# or
token_file = "~/.config/agentfs/turso.token"

# Advanced sync settings
[sync.turso.advanced]
# Batch size for sync operations
batch_size = 100

# Compression for network traffic
compression = true

# Connection pool size
connection_pool_size = 5
```

### Multiple Sync Targets
```toml
[sync.targets]
[sync.targets.primary]
provider = "turso"
database = "workspaces-prod"
priority = 1

[sync.targets.backup]
provider = "turso"
database = "workspaces-backup"
priority = 2
mode = "periodic"
interval = 3600
```

## Hook Scripts

### Available Hooks
```toml
[hooks]
# Run before any agentfs command
pre_command = "~/.config/agentfs/hooks/pre-command.sh"

# Run after command completion
post_command = "~/.config/agentfs/hooks/post-command.sh"

# Run before workspace creation
pre_workspace_create = "~/.config/agentfs/hooks/pre-create.sh"

# Run after workspace creation
post_workspace_create = "~/.config/agentfs/hooks/post-create.sh"

# Run before commit
pre_commit = "~/.config/agentfs/hooks/pre-commit.sh"

# Run after commit
post_commit = "~/.config/agentfs/hooks/post-commit.sh"

# Run on sync
pre_sync = "~/.config/agentfs/hooks/pre-sync.sh"
post_sync = "~/.config/agentfs/hooks/post-sync.sh"
```

### Hook Example
```bash
# ~/.config/agentfs/hooks/post-workspace-create.sh
#!/bin/bash
WORKSPACE_NAME="$1"
WORKSPACE_PATH="$2"

echo "Workspace $WORKSPACE_NAME created at $WORKSPACE_PATH"

# Send notification
if command -v notify-send &> /dev/null; then
    notify-send "AgentFS" "Workspace $WORKSPACE_NAME created"
fi
```

## Advanced Configuration

### Custom Storage Backend
```toml
[storage]
backend = "custom"
custom_backend_path = "/path/to/custom-backend.so"

[storage.custom.options]
endpoint = "http://localhost:9000"
bucket = "agentfs-storage"
```

### Performance Tuning
```toml
[performance]
# Enable memory-mapped files for large reads
mmap_threshold_mb = 100

# I/O scheduling algorithm: noop, cfq, deadline
io_scheduler = "noop"

# File descriptor cache size
fd_cache_size = 1024

# Page cache size (SQLite)
page_cache_size_kb = 8192
```

### Security Hardening
```toml
[security]
# Enable SELinux/AppArmor integration
mac_integration = true

# Restrict file system operations
allow_symlinks = false
allow_hardlinks = false

# Sandbox workspace execution
sandbox_execution = true
sandbox_profile = "restricted"

# Audit all file access
audit_all_reads = true

# Require signed commits
require_signed_commits = false
```

## Configuration Validation

### Validate Config
```bash
# Check configuration syntax
agentfs config validate

# Show effective configuration
agentfs config show

# Show configuration with profile
agentfs config show --profile production
```

### Configuration Errors
```bash
# AgentFS will warn about:
# - Invalid paths
# - Permission issues
# - Missing dependencies
# - Deprecated options

# Test configuration
agentfs config test
```

## Migration

### Migrating Configuration
```bash
# Export configuration
agentfs config export > agentfs-config-backup.toml

# Import configuration
agentfs config import agentfs-config-backup.toml

# Reset to defaults
agentfs config reset
```

## Environment Variables Reference

| Variable | Config Section | Description |
|----------|----------------|-------------|
| `AGENTFS_CONFIG` | - | Config file path |
| `AGENTFS_BASE` | `core.default_base` | Default base directory |
| `AGENTFS_CACHE_SIZE` | `storage.cache_size_mb` | Cache size |
| `AGENTFS_LOG_LEVEL` | `logging.level` | Log level |
| `AGENTFS_PROFILE` | - | Active profile |
| `TURSO_TOKEN` | `sync.turso.token` | Turso API token |
| `TURSO_ORG` | `sync.turso.organization` | Turso organization |

## Next Steps

- **SDKs**: [06-sdks/](./06-sdks/)
- **MCP Integration**: [07-mcp-integration.md](./07-mcp-integration.md)
- **Cloud Sync**: [08-cloud-sync.md](./08-cloud-sync.md)