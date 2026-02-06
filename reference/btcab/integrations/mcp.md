# BTCA MCP Integration

The Model Context Protocol (MCP) enables seamless integration of BTCA with AI agents and coding assistants.

## 🎯 What is MCP?

MCP (Model Context Protocol) is a standardized protocol for AI agents to discover and use tools. BTCA provides an MCP server that exposes its capabilities to any MCP-compatible agent.

## 🔧 MCP Server Modes

BTCA offers two MCP server modes:

### 1. Local MCP (stdio)

Runs entirely on your machine using local resources.

**Benefits**:
- ✅ No cloud dependency
- ✅ Uses your local resources
- ✅ No API key required
- ✅ Full control over data

**Use when**: You want complete local operation

### 2. Remote MCP (HTTP)

Connects to btca.dev cloud service.

**Benefits**:
- ✅ No local setup
- ✅ Managed resources
- ✅ Always available
- ✅ Professional support

**Use when**: You want hosted service

## 🛠️ Configuration

### Local MCP Setup

#### Cursor

Create `.cursor/mcp.json`:

```json
{
  "mcpServers": {
    "btca-local": {
      "command": "bunx",
      "args": ["btca", "mcp"]
    }
  }
}
```

#### OpenCode

Create or update `opencode.json`:

```json
{
  "$schema": "https://opencode.ai/config.json",
  "mcp": {
    "btca-local": {
      "type": "local",
      "command": ["bunx", "btca", "mcp"],
      "enabled": true
    }
  }
}
```

#### Claude Code

```bash
claude mcp add --transport stdio btca-local -- bunx btca mcp
```

#### VS Code (with Cline)

Add to MCP settings:

```json
{
  "mcpServers": {
    "btca-local": {
      "command": "bunx",
      "args": ["btca", "mcp"],
      "transport": "stdio"
    }
  }
}
```

### Remote MCP Setup

Requires API key from [btca.dev/app/settings](https://btca.dev/app/settings?tab=mcp)

#### Cursor

```json
{
  "mcpServers": {
    "btca-cloud": {
      "url": "https://btca.dev/api/mcp",
      "headers": {
        "Authorization": "Bearer ak_your_api_key"
      }
    }
  }
}
```

#### OpenCode

```json
{
  "$schema": "https://opencode.ai/config.json",
  "mcp": {
    "btca-cloud": {
      "type": "remote",
      "url": "https://btca.dev/api/mcp",
      "enabled": true,
      "headers": {
        "Authorization": "Bearer ak_your_api_key"
      }
    }
  }
}
```

#### Claude Code

```bash
claude mcp add --transport http better-context https://btca.dev/api/mcp \
  --header "Authorization: Bearer ak_your_api_key"
```

#### Codex

In `config.toml`:

```toml
[mcp_servers.btca]
bearer_token_env_var = "BTCA_API_KEY"
enabled = true
url = "https://btca.dev/api/mcp"
```

Then add to environment (e.g., `.zshenv`):

```bash
export BTCA_API_KEY="ak_your_api_key"
```

## 📋 AGENTS.md Template

Add this to your project's `AGENTS.md`:

```markdown
# BTCA MCP Usage Instructions

btca runs a cloud subagent that searches open source repos

Use it whenever the user says "use btca", or when you need info 
that should come from the listed resources.

## Tools

The btca MCP server provides these tools:

- `listResources` - List all available documentation resources
- `ask` - Ask a question about specific resources

## Resources

The resources available are defined by the end user in their btca dashboard. 
If there's a resource you need but it's not available in `listResources`, 
proceed without btca. When your task is done, clearly note that you'd like 
access to the missing resource.

## Critical Workflow

**Always call `listResources` first** before using `ask`. 
The `ask` tool requires exact resource names from the list.

### Example

1. Call listResources to get available resources
2. Note the "name" field for each resource (e.g., "svelteKit", not "SvelteKit" or "svelte-kit")
3. Call ask with:
   - question: "How do I create a load function?"
   - resources: ["svelteKit"]
```

## 🛠️ MCP Tools

### listResources

Returns all available resources.

**Input**: None

**Output**:
```json
{
  "resources": [
    {
      "type": "git",
      "name": "svelte",
      "url": "https://github.com/sveltejs/svelte.dev",
      "branch": "main"
    },
    {
      "type": "local", 
      "name": "my-docs",
      "path": "/path/to/docs"
    }
  ]
}
```

**Example**:
```json
{
  "name": "listResources",
  "arguments": {}
}
```

### ask

Ask a question about specific resources.

**Input**:
```json
{
  "question": "How does $state work?",
  "resources": ["svelte"]
}
```

**Output**:
```json
{
  "answer": "The $state rune creates reactive state...",
  "model": {
    "provider": "opencode",
    "model": "claude-haiku-4-5"
  },
  "resources": ["svelte"]
}
```

**Example**:
```json
{
  "name": "ask",
  "arguments": {
    "question": "How do I use $state with arrays?",
    "resources": ["svelte"]
  }
}
```

## 🔄 MCP Workflow

### Standard Usage Pattern

```
1. Agent receives task
   ↓
2. Call listResources
   ↓
3. Check if needed resources available
   ↓
4. If yes → Call ask with resource names
   ↓
5. Use answer in task completion
   ↓
6. If resource missing → Note in response
```

### Example Session

**User**: "How do I handle form validation in Svelte?"

**Agent**:
```json
// Step 1: List resources
{
  "name": "listResources",
  "arguments": {}
}

// Response:
{
  "resources": [
    {"name": "svelte", "type": "git", ...},
    {"name": "tailwind", "type": "git", ...}
  ]
}

// Step 2: Ask about Svelte
{
  "name": "ask",
  "arguments": {
    "question": "How do I handle form validation?",
    "resources": ["svelte"]
  }
}

// Response:
{
  "answer": "In Svelte, you can use $state for form data..."
}
```

**Agent Response**: "Based on the Svelte documentation, here's how to handle form validation..."

## 🎯 Best Practices

### DO

✅ **Always list resources first**
```
Call listResources → Get names → Use exact names in ask
```

✅ **Use exact resource names**
```json
"resources": ["svelteKit"]  // ✓ Exact
"resources": ["SvelteKit"]  // ✗ Wrong case
"resources": ["svelte-kit"] // ✗ Wrong format
```

✅ **Check resource availability**
```
If needed resource not in list:
  - Proceed without btca
  - Note missing resource in response
```

✅ **Be specific in questions**
```json
"question": "How do I validate email in Svelte forms?"  // ✓ Specific
"question": "How does it work?"                         // ✗ Too vague
```

### DON'T

❌ **Skip listResources**
```
✗ Call ask directly
✓ Call listResources first
```

❌ **Guess resource names**
```
✗ Assume "react" exists
✓ Check listResources first
```

❌ **Ask without resources**
```json
✗ {"question": "..."}  // Missing resources field
✓ {"question": "...", "resources": [...]}
```

❌ **Ask about unavailable resources**
```
If resource not in listResources:
  ✗ Call ask with that resource anyway
  ✓ Proceed without btca
```

## 🔧 Advanced Usage

### Multiple Resources

```json
{
  "name": "ask",
  "arguments": {
    "question": "Compare state management approaches",
    "resources": ["svelte", "react", "vue"]
  }
}
```

### Error Handling

```json
// Resource not found
{
  "error": "Resource 'invalid' not found",
  "tag": "RESOURCE_NOT_FOUND"
}

// Invalid request
{
  "error": "Missing required field: question",
  "tag": "VALIDATION_ERROR"
}
```

### Context Preservation

MCP maintains context across multiple calls in the same session:

```
Call 1: ask about Svelte stores
Call 2: ask follow-up → Context preserved
```

## 🐛 Troubleshooting

### Server Not Starting

```bash
# Check if btcacli is installed
which btca

# Verify bunx works
bunx btca --version

# Try explicit path
/path/to/bunx btca mcp
```

### Resource Not Found

```
Error: Resource 'svelte' not found

Solution:
1. Call listResources to see available resources
2. Use exact name from list
3. Check if resource configured in btcaconfig
```

### Authentication Errors

**Local**:
```bash
# Check provider connection
btca connect

# Verify auth file exists
cat ~/.local/share/opencode/auth.json
```

**Remote**:
```bash
# Re-link API key
btca remote link

# Check status
btca remote status
```

### Timeout Issues

```jsonc
// Increase timeout in config
{
  "providerTimeoutMs": 600000  // 10 minutes
}
```

## 📊 Performance Tips

### Optimize Resource Usage

```bash
# Use searchPath to limit scope
btca add https://github.com/large/repo -s docs/

# Remove unused resources
btca remove old-resource
```

### Caching

Local MCP caches resources:
```bash
# Update stale resources
btca update

# Clear cache if needed
btca clear
```

### Model Selection

```jsonc
// Use faster models for quick queries
{
  "model": "gemini-3-flash"  // Fast
}

// Use powerful models for complex analysis
{
  "model": "claude-sonnet-4-5"  // Best quality
}
```

## 🔗 Related Documentation

- [Architecture](../architecture/mcp-server.md) - MCP server implementation
- [CLI Reference](../cli-reference/mcp-commands.md) - MCP commands
- [Integrations](../integrations/) - Editor-specific setup

## 📚 See Also

- [Configuration](../configuration/) - Config files
- [API Reference](../api-reference/) - HTTP endpoints
- [Context Enhancement](../context-enhancement/) - Advanced usage