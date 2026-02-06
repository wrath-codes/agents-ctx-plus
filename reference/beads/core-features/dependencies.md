# Dependencies

Beads provides a sophisticated dependency system that enables complex workflow orchestration, multi-agent coordination, and automated work sequencing.

## 🔗 Dependency Types

### Blocks
The most common dependency type - parent issue must be completed before child can proceed.

```bash
# Issue bd-2 is blocked by bd-1
bd dep add bd-2 bd-1
```

**Characteristics**:
- Child cannot start until parent is closed
- Creates blocking relationships in workflow
- Affects `bd ready` calculation
- Visualized as arrows in dependency tree

**Example Workflow**:
```bash
bd create "Design database schema"     # bd-1
bd create "Create API endpoints"         # bd-2
bd create "Build frontend"               # bd-3

# Set dependencies
bd dep add bd-2 bd-1  # API depends on schema
bd dep add bd-3 bd-2  # Frontend depends on API

# Check ready work
bd ready
# Output: bd-1 (Design database schema)
```

### Parent-Child
Hierarchical relationship for organizing work into epics and subtasks.

```bash
# Create epic and child issues
bd create "User authentication system" -t epic     # bd-epic-001
bd create "Design auth flow" --parent bd-epic-001    # bd-epic-001.1
bd create "Implement login API" --parent bd-epic-001 # bd-epic-001.2
```

**Characteristics**:
- Creates hierarchical ID structure
- Epic completion requires all children
- Parent provides high-level context
- Children inherit metadata from parent

**Hierarchical IDs**:
```
bd-epic-001                 # Epic
├── bd-epic-001.1          # Child task
├── bd-epic-001.2          # Child task  
└── bd-epic-001.3          # Child task
```

### Discovered-From
Links discovered work to the issue that revealed it.

```bash
# Create issue discovered during implementation
bd create "Found SQL injection vulnerability" \
  --deps discovered-from:bd-001 \
  --description "User input not sanitized in query"
```

**Characteristics**:
- Tracks work discovered during implementation
- Maintains context of discovery
- Non-blocking (child can proceed independently)
- Useful for audit trails and context preservation

**Use Cases**:
- Bugs found during feature development
- Refactoring opportunities discovered
- Technical debt identified
- Security vulnerabilities found

### Related
Non-blocking association between related issues.

```bash
# Create related issues
bd create "Add dark mode to settings"      # bd-001
bd create "Add light mode to settings"     # bd-002

# Link as related (no blocking)
bd dep add bd-002 bd-001 --type related
```

**Characteristics**:
- No blocking relationship
- Issues can proceed independently
- Provides context and cross-references
- Useful for grouping related work

## 🎯 Dependency Management Commands

### Add Dependencies

```bash
# Basic blocking dependency
bd dep add bd-child bd-parent

# Specific dependency type
bd dep add bd-002 bd-001 --type blocks
bd dep add bd-002 bd-001 --type parent-child
bd dep add bd-002 bd-001 --type discovered-from
bd dep add bd-002 bd-001 --type related

# Multiple dependencies at once
bd dep add bd-003 bd-001,bd-002

# Bidirectional dependency
bd dep add bd-002 bd-001 --bidirectional
```

### Remove Dependencies

```bash
# Remove specific dependency
bd dep remove bd-002 bd-001

# Remove all dependencies for an issue
bd dep remove bd-002 --all

# Remove by type
bd dep remove bd-002 bd-001 --type blocks
```

### Query Dependencies

```bash
# Show dependency tree
bd dep tree bd-001

# List all dependencies
bd dep list bd-001

# Show what blocks an issue
bd dep parents bd-001

# Show what an issue blocks
bd dep children bd-001

# Check for cycles
bd dep cycles

# Find blocked issues
bd blocked

# Find unblocked (ready) issues
bd ready
```

## 🌳 Dependency Trees

### Visualizing Dependencies

```bash
# Show full dependency tree
bd dep tree bd-epic-001

# Output:
bd-epic-001: User authentication system
├── bd-epic-001.1: Design auth flow [open]
├── bd-epic-001.2: Implement login API [in_progress]
│   └── bd-003: Add OAuth support [blocked]
├── bd-epic-001.3: Create login UI [open]
│   └── bd-004: Add password reset [blocked]
└── bd-epic-001.4: Write tests [open]
```

### Tree Display Options

```bash
# JSON output for programmatic access
bd dep tree bd-001 --json

# Include closed issues
bd dep tree bd-001 --include-closed

# Show only direct dependencies
bd dep tree bd-001 --depth 1

# Full tree with all metadata
bd dep tree bd-001 --full
```

### Cross-Repository Trees

```bash
# Include external dependencies
bd dep tree bd-001 --cross-repo

# Hydrate external dependencies
bd hydrate --from backend-repo

# Show cross-repo tree
bd dep tree bd-frontend-001 --cross-repo
```

## ⚡ Ready Work Calculation

### Algorithm Overview

The `bd ready` command calculates which issues are unblocked and ready to work:

```sql
-- Pseudocode for ready work query
SELECT i.id, i.title, i.priority
FROM issues i
WHERE i.status IN ('open', 'in_progress')
  AND NOT EXISTS (
    -- Check for uncompleted blockers
    SELECT 1 FROM dependencies d
    JOIN issues p ON d.parent_id = p.id
    WHERE d.child_id = i.id 
      AND d.type = 'blocks'
      AND p.status != 'closed'
  )
ORDER BY i.priority ASC, i.created_at ASC;
```

### Ready Work Output

```bash
bd ready

# Output:
Ready work (3 issues):
1. [P0] bd-001: Fix critical security bug
2. [P1] bd-002: Optimize database queries  
3. [P1] bd-003: Update documentation

Blocked work (2 issues):
1. [P1] bd-004: Implement new feature (blocked by bd-002)
2. [P2] bd-005: Add tests (blocked by bd-003)
```

### Filtering Ready Work

```bash
# Ready work for specific agent
bd ready --agent agent-1

# Ready work by type
bd ready --type bug

# Ready work by priority
bd ready --priority 0,1

# Ready work with specific skills
bd ready --skills backend,database
```

## 🔄 Circular Dependencies

### Detection

```bash
# Check for cycles in dependency graph
bd dep cycles

# Output:
Cycle detected:
bd-001 → bd-002 → bd-003 → bd-001
```

### Resolution Strategies

#### Manual Resolution
```bash
# Identify the cycle
bd dep cycles --verbose

# Remove one dependency to break cycle
bd dep remove bd-003 bd-001

# Re-add with corrected relationship
bd dep add bd-001 bd-003  # Reverse direction
```

#### Algorithm Assistance
```bash
# Suggest resolution
bd dep cycles --suggest

# Output:
Suggested resolution:
1. Remove: bd dep remove bd-003 bd-001
2. Re-add as: bd dep add bd-001 bd-003

This breaks the cycle while maintaining logical dependency.
```

### Prevention

```bash
# Validate before adding dependency
bd dep add bd-002 bd-001 --validate

# Dry run to check for cycles
bd dep add bd-002 bd-001 --dry-run
```

## 📊 Dependency Statistics

### Querying Dependency Metrics

```bash
# Dependency statistics
bd dep stats

# Output:
Total dependencies: 45
  - blocks: 32
  - parent-child: 8
  - discovered-from: 3
  - related: 2

Issues with dependencies: 28
Average dependencies per issue: 1.6

Blocked issues: 12
Unblocked issues: 16
```

### Dependency Analysis

```bash
# Find bottlenecks (issues blocking many others)
bd dep bottlenecks

# Output:
Top blocking issues:
1. bd-001: Database schema (blocks 5 issues)
2. bd-002: API design (blocks 4 issues)
3. bd-003: Authentication (blocks 3 issues)
```

### Impact Analysis

```bash
# Show what would be affected by completing an issue
bd dep impact bd-001

# Output:
Completing bd-001 would unblock:
  - bd-002: Implement API (ready for work)
  - bd-003: Build frontend (blocked by bd-002)
  - bd-004: Add tests (blocked by bd-002)
```

## 🎯 Multi-Agent Dependencies

### Cross-Repository Dependencies

```bash
# In frontend repo
bd create "API integration"                    # bd-frontend-001
bd dep add bd-frontend-001 external:backend-repo/bd-api-001

# View cross-repo tree
bd dep tree bd-frontend-001 --cross-repo

# Output:
bd-frontend-001: API integration
└── external:backend-repo/bd-api-001: API endpoints [in_progress]
```

### External Dependency Resolution

```bash
# Hydrate external dependencies
bd hydrate

# Hydrate from specific repo
bd hydrate --from backend-repo

# Auto-hydrate on sync
bd sync --hydrate
```

### Agent Handoff Dependencies

```bash
# Agent A completes work
bd close bd-001 --reason "Ready for Agent B"

# Agent A creates handoff dependency
bd dep add bd-002 bd-001 --type handoff
bd update bd-002 --assignee agent-b

# Agent B picks up work
bd hook --agent agent-b  # Shows bd-002 ready
```

## 🔧 Dependency Workflows

### Sequential Workflow

```bash
# Create sequential workflow
bd create "Step 1: Design" -p 0
bd create "Step 2: Implement" -p 0
bd create "Step 3: Test" -p 0
bd create "Step 4: Deploy" -p 0

# Set sequential dependencies
bd dep add bd-002 bd-001
bd dep add bd-003 bd-002
bd dep add bd-004 bd-003

# Work through sequentially
bd ready  # Shows only bd-001
bd close bd-001
bd ready  # Shows only bd-002
```

### Parallel Workflow

```bash
# Create independent tasks
bd create "Task A" -p 0
bd create "Task B" -p 0
bd create "Task C" -p 0

# No dependencies between A, B, C
bd ready  # Shows all three tasks

# Fan-in: create task that needs all three
bd create "Integration" -p 0
bd dep add bd-004 bd-001,bd-002,bd-003

# Only shown when all three complete
bd ready  # Shows A, B, C
bd close bd-001,bd-002,bd-003
bd ready  # Shows bd-004
```

### Diamond Workflow

```bash
# Diamond pattern: split then merge
bd create "Start" -p 0              # bd-001
bd create "Parallel A" -p 0          # bd-002
bd create "Parallel B" -p 0          # bd-003
bd create "Merge" -p 0               # bd-004

# Set diamond dependencies
bd dep add bd-002 bd-001
bd dep add bd-003 bd-001
bd dep add bd-004 bd-002,bd-003

# Execution:
# 1. Start bd-001
# 2. When bd-001 closes, both bd-002 and bd-003 ready
# 3. When both complete, bd-004 ready
```

## 🛡️ Dependency Validation

### Pre-Commit Validation

```bash
# Validate dependencies before commit
bd dep validate

# Output:
✓ All dependencies valid
✓ No circular dependencies
✓ All parent issues exist
⚠ Warning: 2 issues have stale dependencies
```

### Health Checks

```bash
# Dependency health check
bd doctor --dependencies

# Check for:
# - Circular dependencies
# - Missing parent issues
# - Stale dependencies (parent closed, child still open)
# - Orphaned dependencies
```

### Dependency Integrity

```bash
# Fix broken dependencies
bd dep fix

# Specific fixes:
bd dep fix --remove-stale          # Remove references to deleted issues
bd dep fix --resolve-orphans       # Resolve orphaned dependencies
bd dep fix --break-cycles          # Automatically break cycles
```

## 📈 Dependency Patterns

### Best Practices

```bash
# 1. Keep dependency chains short
# Good: 3-4 levels max
# Bad: 10+ level dependency chains

# 2. Use appropriate dependency types
bd dep add bd-002 bd-001 --type blocks      # Wait for completion
bd dep add bd-002 bd-001 --type discovered-from  # Context only

# 3. Document complex dependencies
bd comment add bd-002 "Depends on bd-001 for database schema"

# 4. Review dependencies regularly
bd dep tree bd-epic-001 --review
```

### Anti-Patterns

```bash
# 1. Circular dependencies (avoid)
bd dep add bd-001 bd-002
bd dep add bd-002 bd-001  # Creates cycle!

# 2. Overly complex graphs (avoid)
# Every issue shouldn't depend on every other issue

# 3. Stale dependencies (avoid)
# Delete or update dependencies when plans change
```

## 🔗 Related Documentation

- [Issue Management](issue-management.md) - Issue creation and management
- [Labels Comments](labels-comments.md) - Issue metadata
- [Workflows](../workflows/) - Advanced workflow patterns
- [Multi-Agent](../multi-agent/) - Cross-repo dependencies
- [CLI Reference](../cli-reference/dependency-commands.md) - Command reference

## 📚 See Also

- [Gates](../workflows/gates.md) - Async coordination
- [Molecules](../workflows/molecules.md) - Work graphs
- [Routing](../multi-agent/routing.md) - Cross-repo routing
- [Recovery Circular](../recovery/circular-dependencies.md) - Cycle resolution