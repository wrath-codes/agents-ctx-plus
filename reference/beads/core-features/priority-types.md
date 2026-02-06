# Priority Levels & Issue Types

Beads uses a structured system of priority levels and issue types to organize and categorize work, enabling efficient filtering, automated workflows, and clear communication.

## 📊 Priority Levels

Beads uses a numeric priority system from **0 (highest)** to **3 (lowest)**, designed to provide clear urgency indicators for AI agents and human collaborators.

### Priority Scale

```
P0 ████████████████████ Critical - Immediate attention required
P1 ████████████████░░░░ High - Address soon  
P2 ██████████░░░░░░░░░░ Medium - Standard work
P3 ████░░░░░░░░░░░░░░░░ Low - Nice to have
```

### P0 - Critical Priority

**Urgency**: Immediate action required
**Characteristics**:
- Production outages or downtime
- Security vulnerabilities
- Data loss or corruption
- Complete work stoppage
- Emergency fixes

**Examples**:
```bash
# Production outage
bd create "Database connection pool exhausted" \
  -p 0 -t bug \
  --description "All requests failing with timeout"

# Security vulnerability  
bd create "SQL injection vulnerability in auth" \
  -p 0 -t bug \
  --label security,critical

# Data corruption
bd create "User data corruption in production" \
  -p 0 -t bug \
  -l "data-loss,critical"
```

**Workflow Impact**:
- Automatically appears at top of `bd ready` list
- Triggers urgent notifications if configured
- Should be addressed before any other work
- May require emergency deployment procedures

**Agent Behavior**:
```bash
# Critical issues always shown first
bd ready --priority 0
# Output: All P0 issues first, ordered by creation time
```

### P1 - High Priority

**Urgency**: Should be addressed soon
**Characteristics**:
- Significant user impact
- Feature blockers
- Major functionality gaps
- Performance degradation
- High visibility issues

**Examples**:
```bash
# Significant user impact
bd create "Login page 500 errors affecting 20% of users" \
  -p 1 -t bug

# Performance issue
bd create "API response time > 5 seconds" \
  -p 1 -t bug \
  --label performance

# Feature blocker
bd create "Missing OAuth integration blocks user onboarding" \
  -p 1 -t feature \
  --label "user-onboarding,blocked"
```

**Workflow Impact**:
- Shown after P0 issues in `bd ready`
- Should be completed within current sprint/cycle
- May block other work
- Requires regular progress updates

### P2 - Medium Priority

**Urgency**: Standard priority work
**Characteristics**:
- Regular feature development
- Minor bug fixes
- Documentation improvements
- Process enhancements
- Technical debt (non-critical)

**Examples**:
```bash
# Regular feature work
bd create "Add user profile editing" \
  -p 2 -t feature

# Minor bug fix
bd create "Button alignment off on mobile" \
  -p 2 -t bug

# Documentation
bd create "Update API documentation for new endpoints" \
  -p 2 -t task \
  --label documentation
```

**Workflow Impact**:
- Default priority for most work
- Balanced against other P2 issues
- Can be scheduled based on capacity
- Good for new team members

### P3 - Low Priority

**Urgency**: Nice to have, no time pressure
**Characteristics**:
- Cosmetic improvements
- Low-impact features
- Cleanup tasks
- Future considerations
- Refinements

**Examples**:
```bash
# Cosmetic improvement
bd create "Update button colors to match brand guidelines" \
  -p 3 -t task

# Future consideration  
bd create "Consider migrating to newer framework version" \
  -p 3 -t task \
  --label "future,technical-debt"

# Cleanup task
bd create "Remove deprecated API endpoints (v1)" \
  -p 3 -t task \
  --label cleanup
```

**Workflow Impact**:
- Shown last in `bd ready` output
- Can be deferred indefinitely
- Good for filling small time gaps
- Often used for "tech debt Friday"

## 🎯 Issue Types

Beads supports multiple issue types to categorize work and enable type-specific workflows and reporting.

### Type Categories

```
Task     ████████████████████ General work items
Bug      ██████████████░░░░░░ Defects and errors
Feature  ██████████░░░░░░░░░░ New functionality
Epic     ████░░░░░░░░░░░░░░░░ Large initiatives
```

### Task Type

**Purpose**: General work items and activities
**Default**: Yes (if no type specified)

**Use Cases**:
- Implementation work
- Refactoring
- Documentation
- Configuration changes
- Setup tasks

**Examples**:
```bash
# Implementation task
bd create "Implement JWT token validation" -t task

# Configuration task
bd create "Update production environment variables" -t task

# Setup task
bd create "Set up CI/CD pipeline" -t task

# Refactoring task
bd create "Extract common utilities to shared module" -t task
```

**Workflow Behavior**:
- Standard workflow - no special handling
- Can be linked to parent epics
- Supports all dependency types
- Default for quick issue creation

### Bug Type

**Purpose**: Defects, errors, and unexpected behavior

**Use Cases**:
- Functional defects
- UI/UX issues
- Performance problems
- Security vulnerabilities
- Data corruption

**Examples**:
```bash
# Functional bug
bd create "Login fails with 2FA enabled" -t bug -p 1

# UI bug
bd create "Modal dialog not centered on mobile" -t bug -p 2

# Performance bug
bd create "Memory leak in image processing" -t bug -p 1

# Security bug  
bd create "XSS vulnerability in comment display" -t bug -p 0
```

**Workflow Behavior**:
- Often requires reproduction steps
- May need regression testing
- Can be prioritized separately from features
- Often tracked in separate metrics

**Bug-Specific Fields**:
```bash
bd create "Payment processing fails" -t bug \
  --description "Steps to reproduce:
1. Add item to cart
2. Proceed to checkout
3. Select PayPal
4. Error: 'Payment failed'

Expected: Payment succeeds
Actual: Error message displayed

Environment: Production, Chrome 120"
```

### Feature Type

**Purpose**: New functionality and enhancements

**Use Cases**:
- User stories
- New capabilities
- Enhancements to existing features
- Integration requests
- API additions

**Examples**:
```bash
# New feature
bd create "Add dark mode support" -t feature

# Enhancement
bd create "Improve search with autocomplete" -t feature

# Integration
bd create "Integrate with Stripe payments" -t feature

# API feature
bd create "Add GraphQL mutations for user management" -t feature
```

**Workflow Behavior**:
- Often requires design/review
- May need user acceptance testing
- Can be broken down into tasks
- Typically higher priority than tech debt

**Feature-Specific Patterns**:
```bash
# Create feature with subtasks
bd create "User authentication system" -t feature  # bd-feat-001
bd create "Design auth flow" --parent bd-feat-001 -t task
bd create "Implement login API" --parent bd-feat-001 -t task
bd create "Create login UI" --parent bd-feat-001 -t task
```

### Epic Type

**Purpose**: Large work items containing multiple sub-tasks

**Use Cases**:
- Major features spanning multiple sprints
- System redesigns
- Multi-team initiatives
- Architectural changes
- Long-term projects

**Examples**:
```bash
# Major feature epic
bd create "Complete user management system" -t epic

# System redesign
bd create "Migrate to microservices architecture" -t epic

# Multi-team initiative
bd create "Implement GDPR compliance" -t epic
```

**Workflow Behavior**:
- Cannot be directly worked on (only children)
- Progress tracked via child completion
- Provides high-level context
- Often has longer timeline

**Epic Structure**:
```bash
# Create epic hierarchy
bd create "User Management Epic" -t epic       # bd-epic-001
bd create "Authentication" --parent bd-epic-001 -t epic  # bd-epic-001.1
bd create "User Profiles" --parent bd-epic-001 -t epic   # bd-epic-001.2
bd create "Permissions" --parent bd-epic-001 -t epic     # bd-epic-001.3

# Add tasks to child epics
bd create "Login page" --parent bd-epic-001.1 -t task
bd create "OAuth integration" --parent bd-epic-001.1 -t task
```

## 🔄 Priority & Type Interactions

### Default Combinations

```bash
# Default priority (2) and type (task)
bd create "Quick task"
# → Priority: 2, Type: task

# Specified priority, default type
bd create "Important task" -p 1
# → Priority: 1, Type: task

# Specified type, default priority
bd create "New feature" -t feature
# → Priority: 2, Type: feature

# Both specified
bd create "Critical bug" -p 0 -t bug
# → Priority: 0, Type: bug
```

### Recommended Combinations

| Type | P0 | P1 | P2 | P3 |
|------|----|----|----|----|
| **Bug** | Production outage | Significant impact | Minor issue | Cosmetic |
| **Feature** | Security requirement | User blocker | Standard feature | Nice to have |
| **Task** | Emergency fix | Important task | Regular work | Cleanup |
| **Epic** | Security initiative | Major project | Feature set | Future planning |

### Filtering by Priority and Type

```bash
# Filter by priority
bd list --priority 0        # Critical only
bd list --priority 0,1      # High priority
bd list --priority 1,2,3    # Exclude critical

# Filter by type
bd list --type bug          # Bugs only
bd list --type feature      # Features only
bd list --type task,bug     # Tasks and bugs

# Combined filtering
bd list --priority 0 --type bug           # Critical bugs
bd list --priority 1 --type feature       # High priority features
bd list --type bug --status open          # Open bugs
bd list --priority 0,1 --type bug,feature --status open

# Ready work by priority and type
bd ready --priority 0,1 --type bug        # Critical/High bugs ready
```

## 📊 Analytics and Reporting

### Priority Distribution

```bash
# View priority distribution
bd stats --by-priority

# Output:
Priority Distribution:
P0 (Critical): 5 issues (8%)
P1 (High): 15 issues (24%)
P2 (Medium): 32 issues (51%)
P3 (Low): 11 issues (17%)
```

### Type Distribution

```bash
# View type distribution
bd stats --by-type

# Output:
Type Distribution:
Tasks: 28 issues (44%)
Bugs: 18 issues (29%)
Features: 14 issues (22%)
Epics: 3 issues (5%)
```

### Combined Metrics

```bash
# Priority x Type matrix
bd stats --matrix priority,type

# Output:
┌──────────┬───────┬───────┬───────┬───────┐
│          │ P0    │ P1    │ P2    │ P3    │
├──────────┼───────┼───────┼───────┼───────┤
│ Bug      │ 3     │ 8     │ 5     │ 2     │
│ Feature  │ 1     │ 4     │ 7     │ 2     │
│ Task     │ 1     │ 2     │ 15    │ 5     │
│ Epic     │ 0     │ 1     │ 2     │ 0     │
└──────────┴───────┴───────┴───────┴───────┘
```

## 🎛️ Workflow Automation

### Priority-Based Automation

```bash
# Auto-escalation
bd create "Server crash" -p 0
# → Triggers on-call alert
# → Adds "urgent" label
# → Notifies team lead

# Auto-assignment
bd create "Performance issue" -p 1
# → Assigns to performance team
# → Adds to sprint backlog

# Auto-scheduling
bd create "Minor UI fix" -p 3
# → Adds to "tech-debt" backlog
# → No immediate assignment
```

### Type-Based Automation

```bash
# Bug workflow
bd create "Login error" -t bug -p 1
# → Requires reproduction steps
# → Adds "needs-reproduction" label
# → Assigns to QA team

# Feature workflow
bd create "New dashboard" -t feature
# → Requires design review
# → Adds "needs-design" label
# → Creates design subtask

# Epic workflow
bd create "New architecture" -t epic
# → Requires architecture review
# → Creates planning subtasks
# → Schedules kickoff meeting
```

## 🎯 Best Practices

### Priority Assignment

**DO**:
```bash
# Be realistic about priority
bd create "Button color wrong" -p 3  # Not P0!

# Use P0 sparingly
# Only for production outages, security, data loss

# Consider user impact
bd create "Login broken" -p 0        # High user impact
bd create "Dark mode" -p 2           # Nice to have
```

**DON'T**:
```bash
# Don't inflate priority
bd create "Update README" -p 0  # This is not critical!

# Don't have too many P0/P1
# If everything is high priority, nothing is

# Don't forget to adjust priority
bd update bd-001 --priority 2  # Lower if no longer urgent
```

### Type Selection

**DO**:
```bash
# Use appropriate types
bd create "Login fails" -t bug           # It's a defect
bd create "Add OAuth" -t feature         # It's new functionality
bd create "Refactor auth" -t task        # It's implementation work
bd create "Complete auth system" -t epic # It's large scope
```

**DON'T**:
```bash
# Don't use wrong types
bd create "Fix typo" -t epic    # Too small
bd create "New feature" -t bug  # Not a defect
bd create "Refactor" -t feature # Not new functionality
```

## 🔗 Related Documentation

- [Issue Management](issue-management.md) - Issue operations
- [Dependencies](dependencies.md) - Issue relationships
- [Labels Comments](labels-comments.md) - Issue metadata
- [CLI Reference](../cli-reference/) - Command reference

## 📚 See Also

- [Workflows](../workflows/) - Advanced workflow patterns
- [Best Practices](../best-practices/) - Usage guidelines
- [Context Enhancement](../context-enhancement/) - Filtering and organization