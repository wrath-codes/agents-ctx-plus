# Multi-Repo Routing

Automatic issue routing across repositories based on configurable patterns.

## 🎯 Overview

Routing enables:

- Issues created in one repo to be automatically routed to another
- Pattern-based routing rules (title, labels, paths)
- Fallback to default repository
- Cross-repo dependency tracking

## 📋 Configuration

### Routes File

Create `.beads/routes.jsonl`:

```jsonl
{"pattern": "frontend/**", "target": "frontend-repo", "priority": 10}
{"pattern": "backend/**", "target": "backend-repo", "priority": 10}
{"pattern": "docs/**", "target": "docs-repo", "priority": 5}
{"pattern": "*", "target": "main-repo", "priority": 0}
```

### Route Fields

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `pattern` | string | ✓ | Glob pattern to match |
| `target` | string | ✓ | Target repository |
| `priority` | integer | ✗ | Higher = checked first (default: 0) |

### Pattern Matching

Patterns match against:
- Issue title
- Labels  
- Explicit path prefix

**Examples**:
```jsonl
{"pattern": "frontend/*", "target": "frontend"}
{"pattern": "*api*", "target": "backend"}
{"pattern": "label:docs", "target": "docs-repo"}
```

## 🛠️ Commands

### Manage Routes

```bash
# Show routing table
bd routes list

# Test routing
bd routes test "Fix frontend button"

# Add route
bd routes add "frontend/**" --target frontend-repo --priority 10

# Remove route
bd routes remove "frontend/**"
```

### Auto-Routing

```bash
# Issue auto-routed based on title
bd create "Fix frontend button alignment" -t bug
# → Routed to frontend-repo

# Override routing
bd create "Fix button" --repo backend-repo
```

## 🔄 Cross-Repo Dependencies

```bash
# Track dependencies across repos
bd dep add bd-42 external:backend-repo/bd-100

# View cross-repo deps
bd dep tree bd-42 --cross-repo
```

## 💧 Hydration

Pull related issues from other repos:

```bash
# Hydrate issues from related repos
bd hydrate

# Preview hydration
bd hydrate --dry-run

# Hydrate specific repo
bd hydrate --from backend-repo
```

## ✅ Best Practices

1. Use specific patterns to avoid overly broad matches
2. Set priorities to ensure specific patterns match first  
3. Always have a `*` fallback pattern with lowest priority
4. Test routes with `bd routes test` before committing

## 🔗 Related Documentation

- [Overview](overview.md) - Multi-agent overview
- [Coordination](coordination.md) - Agent coordination patterns
- [Cross-Repo Dependencies](cross-repo-deps.md) - External dependency tracking