# Labels & Comments

Beads provides flexible metadata management through labels and comments, enabling rich categorization, communication, and context preservation for AI agent workflows.

## 🏷️ Labels

Labels provide flexible, non-hierarchical categorization for issues, enabling filtering, grouping, and workflow automation.

### Label Structure

```json
{
  "issue_id": "bd-a1b2",
  "label": "backend",
  "created_at": "2026-02-06T10:30:00Z"
}
```

**Characteristics**:
- Multiple labels per issue
- Unlimited label values
- No predefined schema
- Case-sensitive
- Space-separated multi-word labels

### Standard Label Categories

#### Component Labels

**Purpose**: Identify which part of the system an issue relates to.

```bash
# Frontend components
bd create "Fix navigation bug" -l "frontend"
bd create "Update CSS styles" -l "frontend,css"

# Backend components  
bd create "Optimize database queries" -l "backend,database"
bd create "Add API endpoint" -l "backend,api"

# Infrastructure
bd create "Update CI pipeline" -l "infrastructure,ci-cd"
bd create "Configure monitoring" -l "infrastructure,monitoring"
```

**Common Components**:
```
frontend, backend, database, api, ui, cli, docs
infrastructure, deployment, security, testing
mobile, desktop, web, api-gateway, cache
```

#### Priority Labels

**Purpose**: Communicate urgency beyond numeric priority.

```bash
# Critical/urgent labels
bd create "Security vulnerability" -l "security,critical"
bd create "Production outage" -l "urgent,p0"

# Performance labels
bd create "Slow page load" -l "performance,frontend"
bd create "Memory leak" -l "performance,backend"
```

**Common Priorities**:
```
critical, urgent, p0, p1, p2, p3
performance, optimization, scalability
security, privacy, compliance
```

#### Process Labels

**Purpose**: Track workflow state and process requirements.

```bash
# Review states
bd create "New feature" -l "needs-review"
bd update bd-001 --add-label "ready-for-review"

# Blocked states
bd update bd-002 --add-label "blocked,waiting-for-ux"
bd update bd-003 --add-label "blocked,external-dependency"

# Deployment states
bd update bd-004 --add-label "ready-to-deploy"
bd update bd-005 --add-label "deployed-to-staging"
```

**Common Process Labels**:
```
needs-review, ready-for-review, in-review
blocked, waiting-for, external-dependency
ready-to-deploy, deployed, staging, production
needs-testing, tested, verified
good-first-issue, help-wanted, beginner-friendly
```

#### Type Labels

**Purpose**: Further categorize issue types.

```bash
# Bug sub-types
bd create "Visual glitch" -l "bug,ui,visual"
bd create "Data corruption" -l "bug,data,critical"

# Feature sub-types
bd create "New endpoint" -l "feature,api,backend"
bd create "Mobile support" -l "feature,mobile,frontend"
```

**Common Type Labels**:
```
bug, feature, enhancement, task, epic
refactor, cleanup, documentation, test
accessibility, a11y, internationalization, i18n
breaking-change, deprecation, migration
```

### Label Operations

#### Adding Labels

```bash
# Add single label
bd label add bd-001 backend

# Add multiple labels
bd label add bd-001 backend,urgent,api

# Add label during creation
bd create "New issue" -l "backend,urgent"

# Add label with update
bd update bd-001 --add-label "ready-for-review"
```

#### Removing Labels

```bash
# Remove single label
bd label remove bd-001 urgent

# Remove multiple labels
bd label remove bd-001 urgent,backend

# Remove all labels
bd label remove bd-001 --all

# Remove label with update
bd update bd-001 --remove-label "blocked"
```

#### Listing Labels

```bash
# List all labels in project
bd label list

# Output:
backend (45 issues)
frontend (32 issues)
urgent (12 issues)
critical (5 issues)
needs-review (23 issues)

# List with issue counts
bd label list --counts

# List with filter
bd label list --search "api"
```

### Filtering by Labels

```bash
# Has all specified labels (AND)
bd list --label backend,urgent

# Has any of specified labels (OR)
bd list --label-any urgent,critical

# Exclude labels (NOT)
bd list --label backend --exclude-label deprecated

# Complex filtering
bd list --label backend \
        --label-any urgent,critical \
        --exclude-label blocked

# Combine with other filters
bd list --status open \
        --type bug \
        --label backend,urgent \
        --priority 0,1
```

### Label Analytics

```bash
# Label statistics
bd label stats

# Output:
┌─────────────────┬──────────┬────────┬──────────┐
│ Label          │ Total    │ Open   │ Closed   │
├─────────────────┼──────────┼────────┼──────────┤
│ backend        │ 45       │ 12     │ 33       │
│ frontend       │ 32       │ 8      │ 24       │
│ urgent         │ 12       │ 5      │ 7        │
│ critical       │ 5        │ 1      │ 4        │
│ needs-review   │ 23       │ 23     │ 0        │
└─────────────────┴──────────┴────────┴──────────┘

# Label trends
bd label trends urgent --days 30

# Output:
Urgent issues over last 30 days:
Week 1: 15 issues
Week 2: 12 issues (-20%)
Week 3: 10 issues (-17%)
Week 4: 8 issues (-20%)
```

## 💬 Comments

Comments provide a way to add context, track decisions, and communicate between agents and humans.

### Comment Structure

```json
{
  "id": 123,
  "issue_id": "bd-a1b2",
  "author": "agent-1",
  "comment": "Started implementation of database layer",
  "created_at": "2026-02-06T10:30:00Z"
}
```

**Characteristics**:
- Chronological history
- Attribution to author/agent
- Immutable (append-only)
- Searchable content
- Supports markdown formatting

### Adding Comments

```bash
# Simple comment
bd comment add bd-001 "Started working on this issue"

# Multi-line comment
bd comment add bd-001 "Found the root cause:
The database connection is not being properly closed
in the authentication handler.

Fix: Add connection.close() in finally block."

# Comment with metadata
bd comment add bd-001 "Design review complete" \
  --author "human-reviewer"

# Comment during work
bd comment add bd-001 "Implemented user model and repository" \
  --type progress
```

### Comment Types

#### Progress Comments

```bash
# Track work progress
bd comment add bd-001 "50% complete - finished data models"
bd comment add bd-001 "75% complete - implementing API endpoints"
bd comment add bd-001 "100% complete - ready for testing"
```

#### Handoff Comments

```bash
# Agent-to-agent communication
bd comment add bd-001 "Completed backend implementation. 
API endpoints ready for frontend integration." \
  --author "backend-agent"

bd comment add bd-001 "Thanks! Starting frontend integration now." \
  --author "frontend-agent"
```

#### Decision Comments

```bash
# Document design decisions
bd comment add bd-001 "Decision: Using PostgreSQL instead of MongoDB
Rationale: Better ACID compliance for financial data
Trade-offs: Slightly more complex setup" \
  --type decision
```

#### Discovery Comments

```bash
# Document discoveries during implementation
bd comment add bd-001 "Discovery: The API rate limit is 100 req/min
This affects our batch processing strategy
Will need to implement rate limiting in client" \
  --type discovery
```

### Viewing Comments

```bash
# Show issue with comments
bd show bd-001 --comments

# Output:
Issue: bd-001
Title: Implement user authentication
...

Comments (3):
1. [2026-02-06 10:00] agent-1: Started implementation
2. [2026-02-06 11:30] agent-1: Database schema designed
3. [2026-02-06 14:00] agent-1: Implementation complete

# Show only comments
bd comment list bd-001

# Show comments with full metadata
bd comment list bd-001 --full
```

### Comment Operations

#### Listing Comments

```bash
# List all comments for issue
bd comment list bd-001

# List with author filter
bd comment list bd-001 --author agent-1

# List with type filter
bd comment list bd-001 --type progress

# JSON output
bd comment list bd-001 --json
```

#### Searching Comments

```bash
# Search within comments
bd list --search "database schema" --search-in comments

# Search specific issue comments
bd comment search bd-001 "implementation"

# Search all comments
bd comment search "database" --project-wide
```

## 🎯 Agent Communication Patterns

### Context Transfer

```bash
# Agent A discovers context and documents
bd comment add bd-001 "Key finding: User authentication fails when
password contains special characters. The regex pattern
/^[a-zA-Z0-9]+$/ doesn't allow symbols."

# Agent B reads context and continues
bd show bd-001 --full
# [Reads comment and understands the problem]
bd update bd-001 --status in_progress
```

### Work Progression

```bash
# Agent A marks progress
bd update bd-001 --status in_progress
bd comment add bd-001 "Completed 3 of 5 API endpoints"

# Agent B checks progress
bd show bd-001
# [Sees progress and takes next task]
bd create "Complete remaining 2 API endpoints" \
  --parent bd-epic-001
```

### Decision Documentation

```bash
# Document architectural decision
bd comment add bd-001 "Architecture Decision Record (ADR):

Decision: Use Redis for session storage instead of database

Context: Database sessions causing performance issues
under high load

Consequences:
- Faster session lookups
- Requires Redis infrastructure
- Session data lost on Redis restart

Status: Accepted
Date: 2026-02-06" \
  --type decision
```

## 🔍 Search and Discovery

### Full-Text Search

```bash
# Search in titles and descriptions
bd list --search "authentication"

# Search in comments
bd list --search "database" --search-in comments

# Search everywhere
bd list --search "redis" --search-in all

# Fuzzy search
bd list --search "auth" --fuzzy
```

### Advanced Filtering

```bash
# Complex label and comment filtering
bd list \
  --label backend \
  --status open \
  --search "performance" \
  --search-in comments \
  --priority 0,1

# Find issues with recent activity
bd list --commented-after "2026-02-01"

# Find issues without comments
bd list --no-comments
```

## 📊 Metadata Management

### Issue Metadata

```bash
# Add custom metadata
bd update bd-001 --metadata "{\"complexity\": \"high\", \"estimated_hours\": 16}"

# Query by metadata
bd list --metadata "complexity=high"
bd list --metadata "estimated_hours>8"

# Update metadata
bd update bd-001 --metadata "{\"actual_hours\": 20}"
```

### Metadata Use Cases

```bash
# Complexity tracking
bd update bd-001 --metadata "{\"complexity\": \"medium\", \"risk\": \"low\"}"

# Time tracking
bd update bd-001 --metadata "{\"estimated_hours\": 8, \"actual_hours\": 12}"

# Sprint planning
bd update bd-001 --metadata "{\"sprint\": \"Sprint-12\", \"story_points\": 5}"

# Custom categorization
bd update bd-001 --metadata "{\"team\": \"backend\", \"category\": \"api\"}"
```

## 🎛️ Automation and Workflows

### Label-Based Automation

```bash
# Auto-assign based on labels
bd update bd-001 --add-label "backend"
# → Automatically assigns to backend team

# Auto-status changes
bd update bd-001 --add-label "ready-for-review"
# → Automatically changes status to "review"

# Notification triggers
bd update bd-001 --add-label "critical"
# → Triggers alert to on-call engineer
```

### Comment-Based Triggers

```bash
# Auto-close on specific comment
bd comment add bd-001 "LGTM - approved for merge" \
  --type approval
# → Automatically closes issue

# Link issues via comments
bd comment add bd-001 "Related to bd-002 - see comments there"
# → Creates related dependency
```

## 📈 Analytics and Reporting

### Label Analytics

```bash
# Label distribution
bd label stats --distribution

# Trend analysis
bd label trends backend --days 90

# Correlation analysis
bd label correlation "bug,testing"
# Shows correlation between bug and testing labels
```

### Comment Analytics

```bash
# Comment frequency
bd comment stats --frequency

# Agent activity
bd comment stats --by-author

# Response times
bd comment stats --response-time
```

## 🔗 Related Documentation

- [Issue Management](issue-management.md) - Issue operations
- [Dependencies](dependencies.md) - Issue relationships
- [Priority Types](priority-types.md) - Issue classification
- [Multi-Agent](../multi-agent/) - Agent communication
- [CLI Reference](../cli-reference/label-commands.md) - Command reference

## 📚 See Also

- [Gates](../workflows/gates.md) - Async coordination
- [Agent Coordination](../multi-agent/coordination.md) - Communication patterns
- [Best Practices](../best-practices/) - Usage patterns