# Better Context - Quick Introduction

> **Ask your AI agent questions about libraries and frameworks by searching the actual source code, not outdated docs.**

## 🎯 What is BTCA?

Better Context (BTCA) is a tool that helps AI coding agents get accurate, up-to-date information about libraries and technologies by **searching actual source code** rather than relying on potentially outdated documentation.

## ⚡ Quick Start

### 1. Install BTCA

```bash
# BTCA requires Bun
bun add -g btca opencode-ai

# Connect to AI provider
btca connect --provider opencode --model claude-haiku-4-5
```

### 2. Initialize Project

```bash
# From your repo root
btca init

# Choose CLI (local) or MCP (cloud)
# Creates btca.config.jsonc or btca.remote.config.jsonc
```

### 3. Add Resources

```bash
# Add a git repository
btca add -n svelte-dev https://github.com/sveltejs/svelte.dev

# Add local directory
btca add -n my-docs -t local /path/to/documentation
```

### 4. Ask Questions

```bash
# One-shot question
btca ask -r svelte-dev -q "How does the $state rune work?"

# Interactive TUI
btca

# Or launch chat mode
btca chat --resource svelte-dev
```

## 🏗️ Three Ways to Use BTCA

### 1. CLI Tool (Local)

```bash
# Interactive TUI
btca

# Direct questions
btca ask -r svelte -q "Question here"

# Local server
btca serve --port 8080
```

**Best for**: Direct usage, local development, quick queries

### 2. MCP Server (Agent Integration)

```bash
# Local MCP (stdio)
btca mcp local

# Remote MCP (HTTP)
btca mcp remote
```

**Best for**: AI agent integration, automated workflows

### 3. Web App (Cloud)

Visit [btca.dev/app](https://btca.dev/app) for the hosted web interface.

**Best for**: Research, exploration, team collaboration

## 📋 Essential Commands

| Command | Description | Example |
|---------|-------------|---------|
| `btca` | Launch TUI | `btca` |
| `btca ask` | Ask one question | `btca ask -r svelte -q "How?"` |
| `btca add` | Add resource | `btca add -n react https://github.com/facebook/react` |
| `btca remove` | Remove resource | `btca remove react` |
| `btca serve` | Start server | `btca serve --port 3000` |
| `btca init` | Initialize project | `btca init` |
| `btca connect` | Connect provider | `btca connect --provider opencode` |

## 🔧 Configuration

### Local Config (btca.config.jsonc)

```jsonc
{
  "$schema": "https://btca.dev/btca.schema.json",
  "provider": "opencode",
  "model": "claude-haiku-4-5",
  "dataDirectory": ".btca",
  "resources": [
    {
      "type": "git",
      "name": "svelte",
      "url": "https://github.com/sveltejs/svelte.dev",
      "branch": "main",
      "searchPath": "apps/svelte.dev"
    }
  ]
}
```

### Remote Config (btca.remote.config.jsonc)

```jsonc
{
  "$schema": "https://btca.dev/btca.remote.schema.json",
  "project": "my-project",
  "model": "claude-sonnet",
  "resources": [
    {
      "type": "git",
      "name": "svelte",
      "url": "https://github.com/sveltejs/svelte.dev",
      "branch": "main"
    }
  ]
}
```

## 🤖 For AI Agents

### AGENTS.md Template

```markdown
# BTCA Usage Instructions

Use btca when you need information about configured resources.

## Tools

- `listResources` - List available documentation resources
- `ask` - Ask a question about specific resources

## Critical Workflow

**Always call listResources first** before using ask.

### Example

1. Call listResources to get available resources
2. Note the exact "name" field (e.g., "svelteKit")
3. Call ask with:
   - question: "How do I create a load function?"
   - resources: ["svelteKit"]
```

### MCP Configuration

```json
// Cursor: .cursor/mcp.json (Local)
{
  "mcpServers": {
    "btca-local": {
      "command": "bunx",
      "args": ["btca", "mcp"]
    }
  }
}

// OpenCode: opencode.json (Local)
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

## 🎓 Example Workflows

### Learning a New Library

```bash
# 1. Add the library
btca add -n svelte https://github.com/sveltejs/svelte

# 2. Ask questions
btca ask -r svelte -q "What are stores and how do I use them?"

# 3. Follow up in TUI
btca
# Then ask follow-up questions interactively
```

### Comparing Libraries

```bash
# Add multiple libraries
btca add -n react https://github.com/facebook/react
btca add -n vue https://github.com/vuejs/core
btca add -n svelte https://github.com/sveltejs/svelte

# Compare approaches
btca ask -r react -r vue -r svelte \
  -q "Compare state management approaches"
```

### Debugging

```bash
# Search for specific implementation details
btca ask -r nextjs \
  -q "How does the App Router handle dynamic segments?"
```

## 💡 Why BTCA?

### Problem: Outdated Documentation

```
Traditional docs:
- Static content
- May be outdated
- Examples might not work
- Limited searchability
```

### Solution: Source Code Search

```
BTCA approach:
- Query actual source code
- Always up-to-date
- See real implementations
- Natural language queries
```

## 🌐 Resources

- **Website**: [btca.dev](https://btca.dev)
- **Docs**: [docs.btca.dev](https://docs.btca.dev)
- **GitHub**: [github.com/davis7dotsh/better-context](https://github.com/davis7dotsh/better-context)
- **App**: [btca.dev/app](https://btca.dev/app)

## 🔗 Learn More

- [Complete Documentation](index.md) - Full reference
- [Architecture](architecture/) - System design
- [CLI Reference](cli-reference/) - All commands
- [API Reference](api-reference/) - HTTP endpoints
- [Integrations](integrations/) - Editor setup