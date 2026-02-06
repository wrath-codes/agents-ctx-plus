# Multi-Agent Coordination

Beads provides comprehensive features for coordinating work between multiple AI agents across repositories, enabling scalable agent collaboration.

## 🤖 Overview

Multi-agent features enable:

- **Routing**: Automatic issue routing to correct repositories
- **Cross-repo dependencies**: Track work across repository boundaries  
- **Agent coordination**: Work assignment and handoff between agents
- **Conflict prevention**: File reservations and issue locking
- **Communication**: Labels, comments, and metadata for agent coordination

## 🎯 Key Concepts

### Routes
Routes define which repository handles which issues based on patterns.

### Work Assignment  
Pin work to specific agents to establish clear ownership.

### Cross-Repo Dependencies
Track dependencies across repositories with `external:repo/issue` syntax.

### Hydration
Pull related issues from other repositories into current context.

## 🏗️ Architecture

```
┌─────────────────┐
│   Main Repo     │
│   (coordinator) │
└────────┬────────┘
         │ routes
    ┌────┴────┐
    │         │
┌───▼───┐ ┌───▼───┐
│Frontend│ │Backend│
│ Repo   │ │ Repo  │
└────────┘ └────────┘
```

## 📁 Documentation Sections

- **[Routing](routing.md)** - Pattern-based issue routing
- **[Coordination](coordination.md)** - Agent handoff patterns
- **[Work Assignment](work-assignment.md)** - Pinning and ownership
- **[Cross-Repo Dependencies](cross-repo-deps.md)** - External dependencies
- **[Hydration](hydration.md)** - Pulling related issues
- **[Conflict Prevention](conflict-prevention.md)** - Reservations and locking
- **[Communication](communication.md)** - Agent communication patterns

## 🚀 Quick Start

### Single Repository (Default)
```bash
cd my-project
bd init
bd create "Regular issue"  # Standard workflow
```

### Multi-Repository Setup
```bash
# Configure routing
echo '{"pattern": "frontend/**", "target": "frontend-repo"}' >> .beads/routes.jsonl
echo '{"pattern": "backend/**", "target": "backend-repo"}' >> .beads/routes.jsonl

# Create routed issue
bd create "Fix frontend button"  # Auto-routed to frontend-repo
```

### Multi-Agent Coordination
```bash
# Assign work to specific agent
bd pin bd-001 --for agent-1 --start

# Check agent's work
bd hook --agent agent-1

# Hand off to another agent
bd pin bd-001 --for agent-2
```

## 🔗 See Also

- [Workflows](../workflows/) - Advanced workflow patterns
- [Integrations](../integrations/) - Agent integration methods
- [Context Enhancement](../context-enhancement/) - Multi-agent context management