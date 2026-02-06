# Issue Management

Beads provides comprehensive issue management capabilities designed specifically for AI agents, with structured data, rich metadata, and workflow-aware features.

## 📋 Issue Overview

An issue in Beads is a work item with rich metadata that supports complex workflows and multi-agent coordination.

### Core Issue Structure

```json
{
  "id": "bd-a1b2",                    // Hash-based collision-resistant ID
  "title": "Set up database",            // Required: Human-readable title
  "description": "Initialize PostgreSQL...",   // Optional: Detailed description
  "status": "open",                    // Current workflow state
  "priority": 1,                        // 0 (highest) to 3 (lowest)
  "type": "task",                       // Issue categorization
  "assignee": "agent-1",               // Current assignee
  "labels": ["backend", "setup"],         // Flexible categorization
  "created_at": "2026-02-06T10:00:00Z", // Creation timestamp
  "updated_at": "2026-02-06T10:30:00Z", // Last update
  "closed_at": null,                     // When issue was closed
  "closed_reason": null,                 // Reason for closing
  "parent_id": "bd-xyz",               // Hierarchical relationship
  "metadata": {                          // Custom key-value pairs
    "complexity": "medium",
    "estimated_hours": 8
  }
}
```

### Issue Fields Reference

| Field | Type | Required | Description | Example |
|--------|-------|----------|-------------|---------|
| `id` | string | ✓ | Hash-based collision-resistant ID | `bd-a1b2` |
| `title` | string | ✓ | Human-readable title (max 200 chars) | `Set up database` |
| `description` | string | ✗ | Detailed description (markdown supported) | `Initialize PostgreSQL...` |
| `status` | string | ✗ | Current workflow state | `open`, `in_progress`, `closed` |
| `priority` | integer | ✗ | Priority level (0=highest, 3=lowest) | `1` |
| `type` | string | ✗ | Issue categorization | `task`, `bug`, `feature`, `epic` |
| `assignee` | string | ✗ | Agent/user assigned to issue | `agent-1` |
| `labels` | array | ✗ | Flexible categorization tags | `["backend", "urgent"]` |
| `created_at` | timestamp | ✗ | Creation timestamp | `2026-02-06T10:00:00Z` |
| `updated_at` | timestamp | ✗ | Last update timestamp | `2026-02-06T10:30:00Z` |
| `closed_at` | timestamp | ✗ | When issue was closed | `2026-02-06T14:00:00Z` |
| `closed_reason` | string | ✗ | Reason for closing | `Implemented successfully` |
| `parent_id` | string | ✗ | Parent issue for hierarchy | `bd-epic-123` |
| `metadata` | object | ✗ | Custom key-value pairs | `{"complexity": "medium"}` |

## 🎯 Issue Types

### Task
Default work item for general tasks.

```bash
bd create "Implement user authentication" -t task
```

**Use Cases**:
- Feature implementation
- Code refactoring
- Documentation updates
- Testing activities

### Bug
Problem reports and defect fixes.

```bash
bd create "Login fails with special chars" -t bug -p 0
```

**Use Cases**:
- Bug reports
- Error corrections
- Performance issues
- Security vulnerabilities

### Feature
New functionality requests.

```bash
bd create "Add dark mode support" -t feature
```

**Use Cases**:
- User stories
- Feature requests
- Enhancement proposals
- New capabilities

### Epic
Large work items that contain sub-tasks.

```bash
bd create "User management system" -t epic
```

**Use Cases**:
- Major features
- System redesigns
- Multi-sprint initiatives
- Architectural changes

## 📊 Priority Levels

### P0 - Critical
Immediate attention required, blocks other work.

```bash
bd create "Database corruption" -p 0 -t bug
```

**Characteristics**:
- Production outages
- Security vulnerabilities
- Data loss scenarios
- Complete work stoppage

### P1 - High
Important issues that should be addressed soon.

```bash
bd create "API performance degradation" -p 1 -t bug
```

**Characteristics**:
- Significant user impact
- Feature blockers
- Major functionality gaps
- High visibility issues

### P2 - Medium
Standard priority for normal work.

```bash
bd create "Add user profile editing" -p 2 -t feature
```

**Characteristics**:
- Regular feature development
- Minor bug fixes
- Documentation improvements
- Process enhancements

### P3 - Low
Nice-to-have items, lower urgency.

```bash
bd create "Update README formatting" -p 3 -t task
```

**Characteristics**:
- Cosmetic improvements
- Low-impact features
- Cleanup tasks
- Future considerations

## 📝 Status Lifecycle

### Open
Initial state, ready for assignment.

```bash
bd create "New issue"  # Automatically created as open
```

**Transitions to**:
- `in_progress` (when work begins)
- `closed` (if immediately resolved)

### In Progress
Active work is being performed.

```bash
bd update bd-a1b2 --status in_progress --assignee agent-1
```

**Characteristics**:
- Assignee is actively working
- Issue is "claimed" to prevent conflicts
- Regular updates expected

### Closed
Work is completed and issue is resolved.

```bash
bd close bd-a1b2 --reason "Implemented successfully"
```

**Closed Reasons**:
- `completed` - Work finished successfully
- `duplicate` - Duplicate of existing issue
- `wontfix` - Issue will not be addressed
- `notrepro` - Could not reproduce the issue
- `invalid` - Issue description invalid

## 🏷️ Label Management

### Standard Label Categories

#### Component Labels
```bash
# Component-based categorization
bd create "Fix API endpoint" -l "backend"
bd create "Update login UI" -l "frontend" 
bd create "Add database index" -l "database"
bd create "Write unit tests" -l "testing"
```

**Common Components**:
- `frontend`, `backend`, `database`
- `api`, `ui`, `cli`, `docs`
- `infrastructure`, `deployment`, `security`

#### Process Labels
```bash
# Process/skill-based labeling
bd create "Code review required" -l "review"
bd create "Waiting for UX feedback" -l "blocked"
bd create "Ready for deployment" -l "deploy"
bd create "Needs testing" -l "testing"
```

**Common Process Labels**:
- `review`, `testing`, `deploy`
- `blocked`, `urgent`, `help-wanted`
- `good-first-issue`, `documentation`

#### Priority Indicators
```bash
# Priority communication
bd create "Critical security fix" -l "critical"
bd create "Performance optimization" -l "performance"
bd create "Accessibility improvement" -l "a11y"
```

**Special Labels**:
- `critical`, `urgent`, `performance`
- `security`, `a11y`, `i18n`
- `breaking-change`, `deprecation`

### Label Operations

```bash
# Add labels to existing issue
bd label add bd-a1b2 backend urgent

# Remove labels
bd label remove bd-a1b2 urgent

# List all labels in project
bd label list

# Find issues with specific labels
bd list --label backend,urgent
bd list --label-any critical,urgent
```

## 👥 Hierarchical Issues

### Parent-Child Relationships

```bash
# Create epic
bd create "User authentication system" -t epic

# Create subtasks
bd create "Design auth flow" --parent bd-epic-123
bd create "Implement login API" --parent bd-epic-123
bd create "Create login UI" --parent bd-epic-123
bd create "Write auth tests" --parent bd-epic-123
```

### Hierarchy Display

```bash
# Show hierarchy tree
bd dep tree bd-epic-123

# Output:
bd-epic-123: User authentication system
├── bd-epic-123.1: Design auth flow
├── bd-epic-123.2: Implement login API  
├── bd-epic-123.3: Create login UI
└── bd-epic-123.4: Write auth tests
```

### Hierarchy Benefits

**Work Organization**:
- Epic provides high-level context
- Subtasks break work into manageable pieces
- Clear progress tracking through hierarchy

**Dependency Management**:
- Child issues automatically depend on parent
- Epic completion requires all children
- Parallel work on different child issues

**Progress Tracking**:
- Epic progress inferred from child status
- Easy burndown tracking
- Clear milestone management

## 📖 Issue Operations

### Create Issue

#### Basic Creation
```bash
# Simple issue creation
bd create "Fix authentication bug"

# With priority and type
bd create "Add user profile" -p 2 -t feature

# With description
bd create "Database migration" \
  --description "Migrate user table to new schema" \
  -p 1 -t task
```

#### Advanced Creation
```bash
# With multiple labels
bd create "API performance fix" \
  -l "backend,urgent,performance" \
  -p 0 -t bug

# As child of parent
bd create "Implement OAuth" \
  --parent bd-epic-123

# With metadata
bd create "Complex feature" \
  --metadata "{\"complexity\": \"high\", \"estimated\": 16}"
```

#### JSON Output
```bash
# Always use --json for agent integration
bd create "New issue" -p 1 --json

# Output:
{
  "id": "bd-a1b2",
  "title": "New issue",
  "status": "open",
  "priority": 1,
  "type": "task",
  "created_at": "2026-02-06T10:00:00Z",
  "assignee": null,
  "labels": []
}
```

### Update Issue

#### Status Updates
```bash
# Start work on issue
bd update bd-a1b2 --status in_progress --assignee agent-1

# Mark as ready for review
bd update bd-a1b2 --status review

# Reopen closed issue
bd update bd-a1b2 --status open
```

#### Field Updates
```bash
# Update priority
bd update bd-a1b2 --priority 0

# Change assignee
bd update bd-a1b2 --assignee agent-2

# Update description
bd update bd-a1b2 --description "Updated description"

# Add metadata
bd update bd-a1b2 --metadata "{\"complexity\": \"low\"}"
```

#### Bulk Updates
```bash
# Update multiple issues
bd update bd-001,bd-002,bd-003 --status in_progress

# Update by query
bd update --status closed --type bug --priority 0
```

### Show Issue

#### Basic Display
```bash
# Show issue details
bd show bd-a1b2

# Output:
Issue: bd-a1b2
Title: Set up database
Status: in_progress
Priority: 1
Type: task
Assignee: agent-1
Created: 2026-02-06 10:00:00
Updated: 2026-02-06 10:30:00
Labels: backend, setup
Description: Initialize PostgreSQL database...
```

#### JSON Output
```bash
# Structured output for agents
bd show bd-a1b2 --json

# Output:
{
  "id": "bd-a1b2",
  "title": "Set up database",
  "description": "Initialize PostgreSQL...",
  "status": "in_progress",
  "priority": 1,
  "type": "task",
  "assignee": "agent-1",
  "labels": ["backend", "setup"],
  "created_at": "2026-02-06T10:00:00Z",
  "updated_at": "2026-02-06T10:30:00Z",
  "dependencies": [...],
  "comments": [...],
  "metadata": {...}
}
```

#### Full Display
```bash
# Show with complete details
bd show bd-a1b2 --full

# Includes:
# - Dependency tree
# - Comment history  
# - Change log
# - Related issues
```

### Close Issue

#### Standard Close
```bash
# Close with reason
bd close bd-a1b2 --reason "Implemented successfully"

# Quick close (default reason)
bd close bd-a1b2

# Close with specific resolution
bd close bd-a1b2 --reason "completed" --resolution "fixed"
```

#### Close Reasons

```bash
# Common close reasons
bd close bd-a1b2 --reason "completed"      # Work finished
bd close bd-a1b2 --reason "duplicate"     # Duplicate issue
bd close bd-a1b2 --reason "wontfix"       # Won't fix
bd close bd-a1b2 --reason "notrepro"        # Not reproducible
bd close bd-a1b2 --reason "invalid"         # Invalid issue
```

### Reopen Issue

```bash
# Reopen closed issue
bd reopen bd-a1b2

# Reopen with comment
bd reopen bd-a1b2 --comment "Issue reproduced in production"
```

### Delete Issue

```bash
# Delete issue (destructive)
bd delete bd-a1b2

# Delete with confirmation (interactive)
bd delete bd-a1b2 --confirm

# Multiple delete
bd delete bd-001,bd-002,bd-003
```

## 🔍 Query and Filtering

### Basic Filtering

```bash
# Filter by status
bd list --status open
bd list --status in_progress
bd list --status closed

# Filter by priority
bd list --priority 0,1
bd list --priority 2

# Filter by type
bd list --type bug
bd list --type feature,task

# Filter by assignee
bd list --assignee agent-1
bd list --unassigned  # No assignee
```

### Advanced Filtering

```bash
# Label filtering
bd list --label backend,urgent          # Has both labels
bd list --label-any urgent,critical      # Has any of these labels

# Date filtering
bd list --created-after "2026-02-01"
bd list --updated-before "2026-02-01"
bd list --closed-after "2026-02-01"

# Text search
bd list --search "authentication"
bd list --search "database performance"
```

### Output Formatting

```bash
# JSON output (for agents)
bd list --json

# Table output (human readable)
bd list --format table

# Compact output
bd list --format compact

# Custom format
bd list --format "{{id}}: {{title}} ({{priority}})"
```

## 🎛️ Agent-Specific Features

### Auto-Assignment

```bash
# Auto-assign to creating agent
bd create "New issue" --auto-assign

# Assign to specific agent
bd create "Security fix" --assignee security-team

# Claim available work
bd claim bd-a1b2  # Sets assignee and in_progress
```

### Work Discovery

```bash
# Find work ready for agent
bd ready --agent agent-1

# Find work matching skills
bd ready --skills backend,security

# Estimate work capacity
bd ready --capacity 8 --hours-available
```

### Progress Tracking

```bash
# Update work progress
bd update bd-a1b2 --progress 50    # 50% complete
bd update bd-a1b2 --progress-est "2h remaining"

# Log work session
bd session start bd-a1b2
bd session log bd-a1b2 "Implemented core logic"
bd session end bd-a1b2
```

## 🔗 Related Documentation

- [Dependencies](dependencies.md) - Issue dependency management
- [Labels Comments](labels-comments.md) - Metadata and communication
- [Priority Types](priority-types.md) - Priority and type details
- [Hash IDs](hash-ids.md) - Collision-resistant ID system
- [CLI Reference](../cli-reference/issue-commands.md) - Complete command reference

## 📚 See Also

- [Workflows](../workflows/) - Advanced workflow management
- [Multi-Agent Coordination](../multi-agent/) - Multi-agent workflows
- [Integrations](../integrations/) - Agent integration patterns
- [Best Practices](../best-practices/) - Usage patterns and guidelines