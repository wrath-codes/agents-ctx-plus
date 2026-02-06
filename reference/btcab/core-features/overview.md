# BTCA Core Features

Better Context provides a comprehensive set of features for AI agents and developers to query and understand codebases through natural language.

## 🎯 Feature Overview

BTCA's core capabilities center around **resource management** and **intelligent querying** of those resources.

## 📚 Resources

Resources are the fundamental building blocks of BTCA. They represent codebases or documentation that BTCA can search and query.

### Resource Types

#### Git Resources

Git repositories cloned from remote URLs:

```jsonc
{
  "type": "git",
  "name": "svelte",
  "url": "https://github.com/sveltejs/svelte.dev",
  "branch": "main",
  "searchPath": "apps/svelte.dev",
  "specialNotes": "Focus on documentation"
}
```

**Fields**:
- `type`: "git" (required)
- `name`: Unique identifier (required)
- `url`: HTTPS URL (required)
- `branch`: Git branch (default: "main")
- `searchPath`: Subdirectory to search (optional)
- `searchPaths`: Multiple paths (optional)
- `specialNotes`: Context hints (optional)

#### Local Resources

Local directories on your filesystem:

```jsonc
{
  "type": "local",
  "name": "my-docs",
  "path": "/absolute/path/to/docs",
  "specialNotes": "Internal documentation"
}
```

**Fields**:
- `type`: "local" (required)
- `name`: Unique identifier (required)
- `path`: Absolute path (required)
- `specialNotes`: Context hints (optional)

### Resource Management

```bash
# Add git resource
btca add https://github.com/sveltejs/svelte.dev -n svelte

# Add with specific branch
btca add https://github.com/sveltejs/svelte.dev -n svelte -b develop

# Add with search path
btca add https://github.com/sveltejs/svelte.dev \
  -n svelte \
  -s apps/svelte.dev

# Add local resource
btca add /path/to/docs -n my-docs -t local

# Interactive add
btca add

# Remove resource
btca remove svelte

# List resources
btca resources

# Clear all cloned resources
btca clear
```

## ❓ Questions & Answers

### One-Shot Questions

Ask a single question and get an immediate answer:

```bash
# Basic question
btca ask -r svelte -q "How does $state work?"

# Multiple resources
btca ask -r svelte -r tailwind \
  -q "How to combine these frameworks?"

# All resources
btca ask -q "What patterns do these libraries share?"

# JSON output (for scripting)
btca ask -r svelte -q "Question" --json

# Hide reasoning
btca ask -r svelte -q "Question" --no-thinking

# Clean output (for agents)
btca ask -r svelte -q "Question" --sub-agent
```

### Interactive Chat

Multi-turn conversations with context:

```bash
# Start chat with specific resource
btca chat --resource svelte

# Start chat with TUI
btca

# Chat supports:
# - Follow-up questions
# - Context retention
# - @resource mentions
```

### Streaming Responses

Real-time answer generation:

```bash
# Stream to terminal
btca ask -r svelte -q "Question" --stream

# Via API
POST /question/stream
{
  "question": "How does $state work?",
  "resources": ["svelte"]
}
```

## 🤖 AI Providers

BTCA supports multiple AI providers for answering questions.

### Supported Providers

| Provider | Type | Authentication |
|----------|------|----------------|
| **OpenCode** | API Key | `~/.local/share/opencode/auth.json` |
| **OpenRouter** | API Key | `~/.local/share/opencode/auth.json` |
| **OpenAI** | OAuth | Browser flow |
| **GitHub Copilot** | OAuth | Device flow |
| **Anthropic** | API Key | `~/.local/share/opencode/auth.json` |
| **Google** | API Key/OAuth | Config |
| **OpenAI-Compat** | Optional Key | Custom base URL |

### Provider Configuration

```bash
# Connect to provider
btca connect

# Connect with specific provider
btca connect --provider opencode

# Connect with model
btca connect --provider opencode --model claude-haiku-4-5

# Disconnect
btca disconnect

# Disconnect specific provider
btca disconnect --provider opencode
```

### OpenAI-Compatible Providers

For custom or local AI servers:

```bash
# Connect to OpenAI-compatible server
btca connect

# You'll be prompted for:
# - Base URL: http://localhost:1234/v1
# - Provider name: lmstudio
# - Model ID: your-model
# - API key (optional)
```

Configuration:

```jsonc
{
  "provider": "openai-compat",
  "model": "your-model",
  "providerOptions": {
    "openai-compat": {
      "baseURL": "http://localhost:1234/v1",
      "name": "lmstudio"
    }
  }
}
```

### Recommended Models

**OpenCode**:
- `claude-sonnet-4-5` (best quality)
- `claude-haiku-4-5` (faster)
- `gemini-3-flash` (fastest)
- `minimax-m2.1`

**OpenAI**:
- `gpt-5.2-codex`

**Cloud (Remote)**:
- `claude-sonnet` (best quality)
- `claude-haiku` (faster)
- `gpt-4o`
- `gpt-4o-mini`

## 🔍 Search Capabilities

### Semantic Search

BTCA uses AI-powered semantic search to find relevant code:

```
Question: "How do I handle form validation?"

BTCA searches:
1. Finds code related to forms
2. Locates validation logic
3. Returns actual implementation examples
4. Provides contextual explanation
```

### Context Preservation

Multi-turn conversations maintain context:

```
User: How does $state work?
AI: Explains $state rune...

User: What about $derived?
AI: Builds on previous context...
```

### Resource Mentions

Use @ to mention resources in chat:

```
@react How do hooks work?
Compare that to @svelte stores
```

## 📊 Response Format

### Standard Response

```json
{
  "answer": "Detailed explanation...",
  "model": {
    "provider": "opencode",
    "model": "claude-haiku-4-5"
  },
  "resources": ["svelte"],
  "collection": {
    "key": "uuid",
    "path": "/path/to/collection"
  }
}
```

### Streaming Response

```
data: {"type": "thinking", "content": "Analyzing..."}
data: {"type": "answer", "content": "The $state rune..."}
data: {"type": "complete"}
```

## 🎛️ Advanced Features

### Special Notes

Add context hints to resources:

```bash
btca add https://github.com/user/repo \
  -n mylib \
  --notes "Focus on the v2 API, not legacy"
```

### Search Paths

Limit search to specific directories:

```bash
# Single path
btca add https://github.com/sveltejs/svelte.dev \
  -n svelte \
  -s apps/svelte.dev

# Multiple paths
btca add https://github.com/sveltejs/svelte.dev \
  -n svelte \
  -s apps/svelte.dev \
  -s packages/svelte
```

### Quiet Mode

```bash
# Minimal output
btca ask -r svelte -q "Question" --quiet

# Via API
POST /question
{
  "question": "...",
  "quiet": true
}
```

## 🔗 Resource Validation

### Name Validation

Resource names must match:
- Max 64 characters
- Regex: `^@?[a-zA-Z0-9][a-zA-Z0-9._-]*(/[a-zA-Z0-9][a-zA-Z0-9._-]*)*$`
- No `..`
- No `//`
- No trailing `/`

### URL Validation

- HTTPS only
- No embedded credentials
- No localhost/private IPs
- GitHub URLs normalized to base repo

### Branch Validation

- Max 128 characters
- Regex: `^[a-zA-Z0-9/_.-]+$`
- Must not start with `-`

### Search Path Validation

- Max 256 characters
- No `..`
- No absolute paths
- No newlines

## 📈 Usage Limits

- **Question length**: Max 100,000 characters
- **Resources per request**: Max 20
- **Special notes**: Max 500 characters

## 🎯 Best Practices

### Resource Naming

**DO**:
```bash
# Use descriptive names
btca add ... -n svelte-latest

# Use version numbers
btca add ... -n react-v18

# Keep it simple
btca add ... -n docs
```

**DON'T**:
```bash
# Too long
btca add ... -n this-is-a-very-long-name-that-is-hard-to-type

# Special characters
btca add ... -n my@resource!

# Uppercase (use lowercase)
btca add ... -n Svelte
```

### Question Formulation

**DO**:
```bash
# Be specific
btca ask -r svelte -q "How do I use $state with arrays?"

# Include context
btca ask -r svelte -q "In Svelte 5, what's the difference between $state and $derived?"
```

**DON'T**:
```bash
# Too vague
btca ask -r svelte -q "How does it work?"

# Multiple questions in one
btca ask -r svelte -q "How do I use $state and $derived and $effect and also stores?"
```

## 🔗 Related Documentation

- [Architecture](../architecture/) - System design
- [Configuration](../configuration/) - Config files
- [CLI Reference](../cli-reference/) - Commands
- [API Reference](../api-reference/) - HTTP endpoints

## 📚 See Also

- [Integrations](../integrations/) - Editor setup
- [Context Enhancement](../context-enhancement/) - Advanced usage