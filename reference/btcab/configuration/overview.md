# BTCA Configuration

BTCA uses JSONC (JSON with Comments) configuration files for flexible, user-friendly configuration management.

## 📁 Configuration Files

### Local Configuration

**File**: `btca.config.jsonc`

**Locations**:
- Project: `./btca.config.jsonc`
- Global: `~/.config/btca/btca.config.jsonc`

**Purpose**: Configure local CLI and server operation with local AI providers.

```jsonc
{
  "$schema": "https://btca.dev/btca.schema.json",
  "provider": "opencode",
  "model": "claude-haiku-4-5",
  "dataDirectory": ".btca",
  "providerTimeoutMs": 300000,
  "providerOptions": {
    "openai-compat": {
      "baseURL": "http://localhost:1234/v1",
      "name": "lmstudio"
    }
  },
  "resources": [
    {
      "type": "git",
      "name": "svelte",
      "url": "https://github.com/sveltejs/svelte.dev",
      "branch": "main",
      "searchPath": "apps/svelte.dev",
      "specialNotes": "Focus on documentation"
    },
    {
      "type": "local",
      "name": "my-docs",
      "path": "/absolute/path/to/docs"
    }
  ]
}
```

### Remote Configuration

**File**: `btca.remote.config.jsonc`

**Location**: Project root only

**Purpose**: Configure cloud service usage with btca.dev.

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
      "branch": "main",
      "searchPath": "apps/svelte.dev"
    }
  ]
}
```

## 🔧 Configuration Options

### Root Options

#### Local Config (`btca.config.jsonc`)

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `$schema` | string | - | Schema URL for validation |
| `provider` | string | "opencode" | AI provider ID |
| `model` | string | "claude-haiku-4-5" | Model identifier |
| `dataDirectory` | string | ".btca" | Data storage location |
| `providerTimeoutMs` | number | 300000 | Provider timeout (ms) |
| `providerOptions` | object | {} | Provider-specific options |
| `resources` | array | [] | Resource definitions |

#### Remote Config (`btca.remote.config.jsonc`)

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `$schema` | string | - | Schema URL for validation |
| `project` | string | - | Project name |
| `model` | string | "claude-sonnet" | Cloud model |
| `resources` | array | [] | Resource definitions |

### Provider Options

#### OpenAI-Compatible

```jsonc
{
  "providerOptions": {
    "openai-compat": {
      "baseURL": "http://localhost:1234/v1",
      "name": "lmstudio"
    }
  }
}
```

**Required Fields**:
- `baseURL`: Root URL of OpenAI-compatible server
- `name`: Provider identifier for AI SDK

**Note**: The AI SDK appends its own endpoint paths to the base URL.

### Resource Configuration

#### Git Resource

```jsonc
{
  "type": "git",
  "name": "svelte",
  "url": "https://github.com/sveltejs/svelte.dev",
  "branch": "main",
  "searchPath": "apps/svelte.dev",
  "searchPaths": ["apps/svelte.dev", "packages/svelte"],
  "specialNotes": "Focus on documentation content"
}
```

**Fields**:

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `type` | string | ✓ | Must be "git" |
| `name` | string | ✓ | Unique identifier |
| `url` | string | ✓ | HTTPS Git URL |
| `branch` | string | ✗ | Git branch (default: "main") |
| `searchPath` | string | ✗ | Single search path |
| `searchPaths` | array | ✗ | Multiple search paths |
| `specialNotes` | string | ✗ | Context hints |

#### Local Resource

```jsonc
{
  "type": "local",
  "name": "my-docs",
  "path": "/absolute/path/to/docs",
  "specialNotes": "Internal documentation"
}
```

**Fields**:

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `type` | string | ✓ | Must be "local" |
| `name` | string | ✓ | Unique identifier |
| `path` | string | ✓ | Absolute filesystem path |
| `specialNotes` | string | ✗ | Context hints |

## 📝 JSONC Features

BTCA supports JSONC format which allows:

### Comments

```jsonc
{
  // This is a comment
  "provider": "opencode",
  
  /* Multi-line
     comment */
  "model": "claude-haiku-4-5"
}
```

### Trailing Commas

```jsonc
{
  "resources": [
    {
      "name": "svelte",
      "url": "https://github.com/sveltejs/svelte",
    }, // ← Trailing comma allowed
  ], // ← Trailing comma allowed
}
```

## ✅ Validation Rules

### Resource Name Validation

```
Pattern: ^@?[a-zA-Z0-9][a-zA-Z0-9._-]*(/[a-zA-Z0-9][a-zA-Z0-9._-]*)*$
Max Length: 64 characters

Allowed:
- Alphanumeric characters
- Periods (.), underscores (_), hyphens (-)
- Forward slashes (/) for scoped names
- @ prefix for scoped names

Not Allowed:
- Double dots (..)
- Double slashes (//)
- Trailing slash
- Control characters
```

**Valid Names**:
- `svelte`
- `svelte-kit`
- `@scope/package`
- `my_lib-v2`

**Invalid Names**:
- `../path` (contains ..)
- `name//sub` (contains //)
- `name/` (trailing slash)
- `UPPERCASE` (should be lowercase)

### Branch Name Validation

```
Pattern: ^[a-zA-Z0-9/_.-]+$
Max Length: 128 characters
Must not start with: -

Allowed:
- Alphanumeric
- Forward slashes (/)
- Underscores (_)
- Periods (.)
- Hyphens (-)
```

### Search Path Validation

```
Max Length: 256 characters
Not Allowed:
- Double dots (..)
- Absolute paths (starting with /)
- Newlines

Must be:
- Relative path
- Within repository
```

### Special Notes Validation

```
Max Length: 500 characters
Not Allowed:
- Control characters (except whitespace)
```

### Question Validation

```
Max Length: 100,000 characters
```

### Resources Per Request

```
Maximum: 20 resources per question
```

### Git URL Validation

```
Requirements:
- HTTPS only
- No embedded credentials (user:pass@)
- No localhost
- No private IP addresses
- GitHub URLs normalized to base repo
```

**Valid URLs**:
- `https://github.com/user/repo`
- `https://gitlab.com/user/repo`

**Invalid URLs**:
- `http://github.com/...` (not HTTPS)
- `https://user:pass@github.com/...` (credentials)
- `https://localhost/...` (localhost)
- `https://192.168.1.1/...` (private IP)

## 🔄 Config Loading Order

### Resolution Strategy

1. **Check for project config** (`./btca.config.jsonc`)
2. **If exists**: Use as base, no global merge
3. **If missing**: Load global config (`~/.config/btca/btca.config.jsonc`)
4. **Apply defaults** for missing values

### Data Directory Resolution

```
If project config exists:
  dataDirectory resolves relative to project root

If using global config:
  dataDirectory resolves relative to home directory
```

## 🎨 Configuration Examples

### Basic Local Setup

```jsonc
{
  "$schema": "https://btca.dev/btca.schema.json",
  "provider": "opencode",
  "model": "claude-haiku-4-5",
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

### Advanced Local Setup

```jsonc
{
  "$schema": "https://btca.dev/btca.schema.json",
  "provider": "openai-compat",
  "model": "my-local-model",
  "dataDirectory": ".btca",
  "providerTimeoutMs": 600000,
  "providerOptions": {
    "openai-compat": {
      "baseURL": "http://localhost:1234/v1",
      "name": "lmstudio"
    }
  },
  "resources": [
    {
      "type": "git",
      "name": "svelte",
      "url": "https://github.com/sveltejs/svelte.dev",
      "branch": "main",
      "searchPath": "apps/svelte.dev",
      "specialNotes": "Focus on documentation"
    },
    {
      "type": "git",
      "name": "tailwind",
      "url": "https://github.com/tailwindlabs/tailwindcss.com",
      "branch": "main",
      "searchPath": "src/docs"
    },
    {
      "type": "local",
      "name": "project-docs",
      "path": "/home/user/projects/my-project/docs"
    }
  ]
}
```

### Remote Setup

```jsonc
{
  "$schema": "https://btca.dev/btca.remote.schema.json",
  "project": "my-awesome-project",
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

### Multi-Resource Setup

```jsonc
{
  "$schema": "https://btca.dev/btca.schema.json",
  "provider": "opencode",
  "model": "claude-haiku-4-5",
  "resources": [
    {
      "type": "git",
      "name": "svelte",
      "url": "https://github.com/sveltejs/svelte"
    },
    {
      "type": "git",
      "name": "svelte-kit",
      "url": "https://github.com/sveltejs/kit",
      "searchPath": "documentation"
    },
    {
      "type": "git",
      "name": "vite",
      "url": "https://github.com/vitejs/vite",
      "searchPath": "docs"
    },
    {
      "type": "local",
      "name": "internal-docs",
      "path": "/home/user/docs",
      "specialNotes": "Internal company documentation"
    }
  ]
}
```

## 🔍 Default Values

If global config is missing, these defaults apply:

```jsonc
{
  "provider": "opencode",
  "model": "claude-haiku-4-5",
  "providerTimeoutMs": 300000,
  "resources": [
    {
      "type": "git",
      "name": "svelte",
      "url": "https://github.com/sveltejs/svelte.dev"
    },
    {
      "type": "git",
      "name": "tailwindcss",
      "url": "https://github.com/tailwindlabs/tailwindcss.com"
    },
    {
      "type": "git",
      "name": "nextjs",
      "url": "https://github.com/vercel/next.js"
    }
  ]
}
```

## ⚠️ Known Limitations

### Global Flag Behavior

The `--global` flag exists on several commands, but its behavior depends on whether a project config exists:

```bash
# If ./btca.config.jsonc exists:
# --global may not strictly override to global

# If no project config:
# --global works as expected
```

### Remote Add Defaults

```bash
# Interactive path uses:
btca remote add  # → Defaults to claude-haiku

# Non-interactive path uses:
btca remote add <url> -n <name>  # → Defaults to claude-sonnet
```

## 🛠️ Config Management

### Via CLI

```bash
# Initialize config
btca init

# Add resource (updates config)
btca add https://github.com/user/repo -n name

# Remove resource
btca remove name

# Connect provider (updates config)
btca connect -p opencode -m claude-haiku-4-5
```

### Via API

```bash
# Get config
curl http://localhost:8080/config

# Add resource
curl http://localhost:8080/config/resources \
  -X POST \
  -H "Content-Type: application/json" \
  -d '{
    "type": "git",
    "name": "svelte",
    "url": "https://github.com/sveltejs/svelte"
  }'
```

### Manual Editing

```bash
# Edit local config
vim btca.config.jsonc

# Edit global config
vim ~/.config/btca/btca.config.jsonc

# Validate after editing
btca resources  # Will show errors if invalid
```

## 🔗 Related Documentation

- [Core Features](../core-features/) - Feature details
- [CLI Reference](../cli-reference/) - Commands
- [Architecture](../architecture/) - System design
- [Validation](../validation.md) - Detailed validation rules

## 📚 See Also

- [API Reference](../api-reference/) - HTTP endpoints
- [Integrations](../integrations/) - Editor setup