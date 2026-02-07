# bd-workflow CLI Reference

## Global Flags

| Flag | Default | Description |
|------|---------|-------------|
| `--config` | `$HOME/.bd-workflow.yaml` | Config file path |
| `--db-path` | `./data` | Database directory |
| `--verbose` | `false` | Enable verbose output |

## Commands

### `bd-workflow setup`

Initialize the workflow system (creates data directories).

### `bd-workflow migrate`

Run pending database migrations.

### `bd-workflow status`

Display system status (version, database connectivity).

---

### `bd-workflow workflow start <type> <title>`

Create a new workflow (does not execute it).

| Flag | Default | Description |
|------|---------|-------------|
| `--agent` | | Agent type to assign |
| `--priority` | `2` | Priority 0-3 (0 = highest) |
| `--variable` | | Key=value pairs (repeatable) |
| `--template` | | Template ID to use |

```bash
bd-workflow workflow start research "Analyze frameworks" --agent research --priority 1
```

### `bd-workflow workflow execute <type> <title>`

Create and run a workflow to completion. Same flags as `start`.

```bash
bd-workflow workflow execute research "Compare async runtimes" --variable "query=async runtimes"
bd-workflow workflow execute poc "Build auth" --agent poc --variable "language=rust"
bd-workflow workflow execute documentation "Generate docs" --template docs-comprehensive
bd-workflow workflow execute validation "Validate release"
```

### `bd-workflow workflow status <workflow-id>`

Show detailed workflow status including results.

### `bd-workflow workflow list`

List workflows with optional filtering.

| Flag | Default | Description |
|------|---------|-------------|
| `--status` | | Filter by status |
| `--agent-type` | | Filter by workflow type |
| `--limit` | `20` | Max results |

### `bd-workflow workflow cancel <workflow-id>`

Cancel a running workflow.

| Flag | Default | Description |
|------|---------|-------------|
| `--reason` | `User request` | Cancellation reason |

### `bd-workflow workflow results <workflow-id>`

Show workflow results.

| Flag | Default | Description |
|------|---------|-------------|
| `--format` | `table` | Output format (table, json) |

---

### `bd-workflow workflow template list`

List all available workflow templates.

| Flag | Default | Description |
|------|---------|-------------|
| `--type` | | Filter by agent type |

### `bd-workflow workflow template show <template-id>`

Show template details (steps, variables, config).

---

### `bd-workflow agent register <agent-id>`

Register a new agent.

| Flag | Default | Description |
|------|---------|-------------|
| `--type` | `research` | Agent type |
| `--max-workload` | `5` | Max concurrent workflows |

### `bd-workflow agent status <agent-id>`

Show agent status and active assignments.

### `bd-workflow agent list`

List all registered agents.

| Flag | Default | Description |
|------|---------|-------------|
| `--type` | | Filter by agent type |

---

### `bd-workflow analytics performance`

Show performance analytics.

| Flag | Default | Description |
|------|---------|-------------|
| `--period` | `7d` | Time period (1d, 7d, 30d) |
| `--agent-type` | | Filter by agent type |
| `--format` | `table` | Output format (table, json) |

### `bd-workflow analytics summary`

Show system summary (workflow counts, agent counts, recent activity).

## Templates

| ID | Agent Type | Steps | Description |
|----|-----------|-------|-------------|
| `research-basic` | research | 3 | Basic library research |
| `research-performance` | research | 4 | Performance-focused research |
| `poc-basic` | poc | 5 | Basic POC (build + test) |
| `poc-full` | poc | 6 | Full POC with benchmarks |
| `docs-basic` | documentation | 4 | README and basic API docs |
| `docs-comprehensive` | documentation | 6 | Full documentation suite |
| `validation-standard` | validation | 4 | Code quality + security |
| `validation-full` | validation | 6 | All validation categories |