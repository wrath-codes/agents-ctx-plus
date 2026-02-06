# BTCA CLI Reference

Complete reference for all `btca` commands, options, and usage patterns.

## 🌍 Global Options

These options apply to most CLI commands:

| Option | Description | Example |
|--------|-------------|---------|
| `--server <url>` | Use existing server | `--server http://localhost:3000` |
| `--port <port>` | Port for auto-started server | `--port 8080` |
| `--no-tui` | Use REPL instead of TUI | `--no-tui` |
| `--no-thinking` | Hide reasoning output | `--no-thinking` |
| `--no-tools` | Hide tool traces | `--no-tools` |
| `--sub-agent` | Clean output for agents | `--sub-agent` |
| `--verbose` | Verbose logging | `--verbose` |

## 🎯 Core Commands

### `btca` (Default)

Launch the interactive TUI (Terminal User Interface).

```bash
# Launch TUI (default)
btca

# Launch REPL instead
btca --no-tui

# Connect to existing server
btca --server http://localhost:3000
```

**REPL Commands**:
- `/help` - Show help
- `/resources` - List resources
- `/clear` - Clear session
- `/quit` or `/exit` - Exit

**Features**:
- Multi-turn conversations
- @resource mentions
- Context preservation
- Command history

### `btca ask`

Ask a one-shot question with streaming output.

```bash
# Basic usage
btca ask -r svelte -q "How does $state work?"

# Multiple resources
btca ask -r svelte -r tailwind -q "Compare these"

# All resources
btca ask -q "What patterns are common?"

# JSON output
btca ask -r svelte -q "..." --json

# Hide reasoning
btca ask -r svelte -q "..." --no-thinking

# Clean output (for agents)
btca ask -r svelte -q "..." --sub-agent

# With resource mention in question
btca ask -q "@svelte How does $state work?"
```

**Options**:
- `-r, --resource <name>` - Resource to query (repeatable)
- `-q, --question <text>` - Question text (required)
- `--no-thinking` - Hide reasoning
- `--no-tools` - Hide tool traces
- `--sub-agent` - Clean output

### `btca add`

Add a git repository or local directory as a resource.

```bash
# Add git repo with auto-detected name
btca add https://github.com/sveltejs/svelte

# Add with specific name
btca add https://github.com/sveltejs/svelte -n svelte

# Add with branch
btca add https://github.com/sveltejs/svelte -b develop

# Add with search path
btca add https://github.com/sveltejs/svelte -s src

# Add with multiple search paths
btca add https://github.com/sveltejs/svelte -s src -s docs

# Add with notes
btca add https://github.com/sveltejs/svelte --notes "Focus on v4"

# Add local directory
btca add /path/to/docs -n my-docs -t local

# Interactive mode
btca add

# Global resource
btca add https://github.com/sveltejs/svelte -g
```

**Options**:
- `-n, --name <name>` - Resource name
- `-b, --branch <branch>` - Git branch (default: main)
- `-s, --search-path <path>` - Search path (repeatable)
- `--notes <notes>` - Special notes
- `-t, --type <git|local>` - Force resource type
- `-g, --global` - Add to global config

### `btca remove`

Remove a resource by name.

```bash
# Remove by name
btca remove svelte

# Interactive mode
btca remove

# Global resource
btca remove svelte -g
```

**Options**:
- `-g, --global` - Remove from global config

### `btca resources`

List all configured resources.

```bash
# List all
btca resources

# Verbose output
btca resources --verbose

# JSON output
btca resources --json
```

### `btca clear`

Clear all locally cloned resources.

```bash
# Clear everything
btca clear

# Confirm clearing
btca clear --force
```

### `btca serve`

Start the local HTTP server.

```bash
# Start on default port (8080)
btca serve

# Start on specific port
btca serve -p 3000

# Start in background
btca serve --daemon
```

**Options**:
- `-p, --port <port>` - Port number (default: 8080)

**Server Endpoints**:
- `GET /health` - Health check
- `GET /resources` - List resources
- `POST /question` - Ask question
- `POST /question/stream` - Streamed question
- `GET /config` - Get configuration
- `POST /config/resources` - Add resource

### `btca connect`

Configure AI provider and model.

```bash
# Interactive connection
btca connect

# Connect with provider
btca connect -p opencode

# Connect with provider and model
btca connect -p opencode -m claude-haiku-4-5

# Global config
btca connect -g
```

**Options**:
- `-p, --provider <id>` - Provider ID
- `-m, --model <id>` - Model ID
- `-g, --global` - Update global config

**Supported Providers**:
- `opencode` - OpenCode API
- `openrouter` - OpenRouter API
- `openai` - OpenAI (OAuth)
- `github-copilot` - GitHub Copilot (OAuth)
- `anthropic` - Anthropic API
- `google` - Google AI
- `openai-compat` - OpenAI-compatible

### `btca disconnect`

Disconnect AI provider credentials.

```bash
# Interactive disconnection
btca disconnect

# Disconnect specific provider
btca disconnect -p opencode
```

**Options**:
- `-p, --provider <id>` - Provider to disconnect

### `btca skill`

Install the btca CLI skill.

```bash
# Run skill installer
btca skill

# This runs skills.sh interactively
```

### `btca init`

Initialize a project with BTCA configuration.

```bash
# Initialize interactively
btca init

# Force overwrite existing config
btca init -f
```

**Setup Types**:
- **CLI**: Local resources, btca.config.jsonc
- **MCP**: Cloud resources, btca.remote.config.jsonc

**What it does**:
1. Detects project type
2. Prompts for setup mode
3. Creates config file
4. Sets up .btca/ directory
5. Updates .gitignore

### `btca mcp`

Run or configure MCP (Model Context Protocol) server.

```bash
# Run MCP server (stdio)
btca mcp

# Scaffold local MCP config
btca mcp local

# Scaffold remote MCP config
btca mcp remote
```

**MCP Tools**:
- `listResources` - List available resources
- `ask` - Ask a question

## ☁️ Remote Commands

All `btca remote` commands require API key authentication.

### `btca remote link`

Authenticate with btca cloud API.

```bash
# Interactive (prompts for key)
btca remote link

# With key directly
btca remote link --key btca_xxxxxxxxxxxx
```

### `btca remote unlink`

Remove stored cloud API key.

```bash
btca remote unlink
```

### `btca remote status`

Show sandbox state and plan info.

```bash
btca remote status

# JSON output
btca remote status --json
```

### `btca remote wake`

Pre-warm the sandbox.

```bash
btca remote wake
```

### `btca remote add`

Add a resource to remote config.

```bash
# Add git repo
btca remote add https://github.com/sveltejs/svelte -n svelte

# With options
btca remote add https://github.com/sveltejs/svelte \
  -n svelte \
  -b main \
  -s docs
```

**Options**:
- `-n, --name <name>` - Resource name
- `-b, --branch <branch>` - Git branch
- `-s, --search-path <path>` - Search path
- `--notes <notes>` - Special notes

### `btca remote sync`

Sync local remote config with cloud.

```bash
# Sync
btca remote sync

# Force overwrite cloud
btca remote sync --force
```

### `btca remote ask`

Ask a question via cloud sandbox.

```bash
# Basic
btca remote ask -q "How does $state work?"

# With specific resources
btca remote ask -r svelte -q "..."

# Multiple resources
btca remote ask -r svelte -r react -q "..."
```

### `btca remote grab`

Fetch full thread transcript.

```bash
# Get as markdown (default)
btca remote grab <thread-id>

# Get as JSON
btca remote grab <thread-id> --json

# Save to file
btca remote grab <thread-id> > transcript.md
```

### `btca remote init`

Create remote config file.

```bash
# Create btca.remote.config.jsonc
btca remote init

# With project name
btca remote init -p my-project
```

### `btca remote mcp`

Output MCP configuration snippet.

```bash
# For OpenCode
btca remote mcp opencode

# For Claude Code
btca remote mcp claude
```

## 📝 Command Examples

### Daily Workflow

```bash
# Start the day - check resources
btca resources

# Quick question
btca ask -r svelte -q "How do stores work?"

# Add new library
btca add https://github.com/tailwindlabs/tailwindcss -n tailwind

# Compare approaches
btca ask -r svelte -r tailwind -q "Compare styling approaches"
```

### Project Setup

```bash
# Initialize project
btca init

# Connect to AI provider
btca connect -p opencode -m claude-haiku-4-5

# Add project dependencies
btca add https://github.com/sveltejs/svelte -n svelte
btca add https://github.com/tailwindlabs/tailwindcss -n tailwind

# Verify setup
btca resources
```

### Agent Integration

```bash
# Check resources (for agent)
btca resources --json

# Ask with clean output
btca ask -r svelte -q "..." --sub-agent

# Get JSON response
btca ask -r svelte -q "..." --json
```

### Cloud Usage

```bash
# Link account
btca remote link

# Add cloud resources
btca remote add https://github.com/sveltejs/svelte -n svelte

# Sync config
btca remote sync

# Ask via cloud
btca remote ask -r svelte -q "..."

# Check status
btca remote status
```

## 🔄 Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Success |
| 1 | General error |
| 2 | Invalid arguments |
| 3 | Resource not found |
| 4 | Provider error |
| 5 | Network error |
| 6 | Config error |

## 🔗 Related Documentation

- [Configuration](../configuration/) - Config files
- [Architecture](../architecture/) - System design
- [API Reference](../api-reference/) - HTTP endpoints
- [Integrations](../integrations/) - Editor setup

## 📚 See Also

- [Core Features](../core-features/) - Feature details
- [Context Enhancement](../context-enhancement/) - Advanced usage