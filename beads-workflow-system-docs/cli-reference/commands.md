# CLI Command Reference

## Global Flags

All commands support these flags:

```
--config, -c        Config file path (default: $HOME/.workflow.yaml)
--verbose, -v       Verbosity level (debug, info, warn, error)
--output, -o        Output format (table, json, yaml)
--help, -h          Show help
```

## Commands

### workflow start

Start a new workflow.

**Usage:**
```bash
workflow start <type> <title> [flags]
```

**Arguments:**
- `type` - Workflow type (research, poc, documentation, validation)
- `title` - Workflow title

**Flags:**
- `-p, --priority` - Priority (0-3, 0=highest)
- `-a, --agent` - Agent type
- `-t, --template` - Template ID
- `-v, --variable` - Workflow variables (key=value)
- `-w, --wait` - Wait for completion

**Examples:**
```bash
# Start a research workflow
workflow start research "Analyze tokio performance"

# Start with specific agent
workflow start research "Compare async libraries" \
  --agent research \
  --priority 0 \
  --variable "focus=performance" \
  --variable "languages=rust,go"

# Use template
workflow start poc "Implement feature" \
  --template web-server \
  --variable "port=8080"

# Wait for completion
workflow start validation "Run tests" --wait
```

### workflow status

Get workflow status.

**Usage:**
```bash
workflow status <workflow-id>
```

**Output:**
```
📋 Workflow: wf-research-001
Status:       in_progress
Agent:        research-agent-01
Progress:     65%
Current Step: documentation_analysis
Started:      2026-02-07T10:30:00Z
Estimated:    2026-02-07T10:45:00Z
```

### workflow list

List workflows.

**Usage:**
```bash
workflow list [flags]
```

**Flags:**
- `-s, --status` - Filter by status
- `-t, --type` - Filter by type
- `-a, --agent` - Filter by agent
- `-l, --limit` - Limit results (default: 50)
- `--offset` - Offset for pagination

**Examples:**
```bash
# List active workflows
workflow list --status active

# List research workflows
workflow list --type research --limit 10

# JSON output
workflow list -o json
```

### workflow cancel

Cancel a workflow.

**Usage:**
```bash
workflow cancel <workflow-id> [flags]
```

**Flags:**
- `-r, --reason` - Cancellation reason

### workflow results

Get workflow results.

**Usage:**
```bash
workflow results <workflow-id> [flags]
```

**Flags:**
- `-f, --format` - Output format (json, yaml)
- `-o, --output` - Output file

### workflow logs

View workflow logs.

**Usage:**
```bash
workflow logs <workflow-id> [flags]
```

**Flags:**
- `-f, --follow` - Follow logs in real-time
- `-n, --lines` - Number of lines (default: 100)
- `--since` - Show logs since time

## Agent Commands

### agent register

Register an agent.

**Usage:**
```bash
agent register [flags]
```

**Flags:**
- `-c, --config` - Agent configuration file
- `-t, --type` - Agent type
- `-i, --id` - Agent ID

### agent status

Get agent status.

**Usage:**
```bash
agent status <agent-id> [flags]
```

### agent list

List registered agents.

**Usage:**
```bash
agent list [flags]
```

## Analytics Commands

### analytics performance

View performance analytics.

**Usage:**
```bash
analytics performance [flags]
```

**Flags:**
- `-p, --period` - Time period (default: 7d)
- `-t, --type` - Workflow type filter

## Configuration

The CLI uses configuration files in these locations:

1. Command line flag: `--config`
2. Environment variable: `WORKFLOW_CONFIG`
3. Current directory: `./.workflow.yaml`
4. Home directory: `~/.workflow.yaml`

**Example config:**
```yaml
api:
  host: localhost
  port: 8080
  timeout: 30s

output:
  format: table
  color: true

logging:
  level: info
```