# JSONL Layer

The JSONL (JSON Lines) layer serves as the **operational source of truth** in Beads' three-layer architecture, providing an append-only, human-readable format that's Git-mergeable and recoverable.

## 📄 Role in Three-Layer Architecture

The JSONL layer is Layer 2 in Beads' architecture:

```
┌─────────────────┐
│   Git Repo     │ ← Historical Source of Truth
│ (issues.jsonl) │
└────────┬────────┘
         │
┌────────▼────────┐
│   JSONL Files  │ ← Operational Source of Truth  
│ (append-only)  │
└────────┬────────┘
         │
┌────────▼────────┐
│   SQLite DB    │ ← Fast Queries / Derived State
│  (beads.db)   │
└─────────────────┘
```

**Key Characteristic**: JSONL is the *authoritative current state* that SQLite is rebuilt from.

## 📝 JSONL Format Specification

### Basic Structure
Each line in a JSONL file is a separate JSON object representing an operation:

```jsonl
{"id": "bd-a1b2", "type": "create", "timestamp": "2026-02-06T10:00:00Z", "data": {"title": "Set up database", "priority": 1, "type": "task"}}
{"id": "bd-a1b2", "type": "update", "timestamp": "2026-02-06T10:30:00Z", "data": {"status": "in_progress"}}
{"id": "bd-a1b2", "type": "label", "timestamp": "2026-02-06T10:45:00Z", "data": {"action": "add", "label": "backend"}}
{"id": "bd-c3d4", "type": "create", "timestamp": "2026-02-06T11:00:00Z", "data": {"title": "Create API", "priority": 2}}
{"id": "bd-a1b2", "type": "close", "timestamp": "2026-02-06T14:00:00Z", "data": {"reason": "Implemented successfully"}}
```

### Operation Types

#### Create Operation
```jsonl
{
  "id": "bd-a1b2",
  "type": "create", 
  "timestamp": "2026-02-06T10:00:00Z",
  "data": {
    "title": "Set up database",
    "description": "Initialize PostgreSQL with required tables",
    "priority": 1,
    "type": "task",
    "labels": ["backend", "setup"]
  }
}
```

#### Update Operation
```jsonl
{
  "id": "bd-a1b2",
  "type": "update",
  "timestamp": "2026-02-06T10:30:00Z", 
  "data": {
    "status": "in_progress",
    "assignee": "agent-1"
  }
}
```

#### Dependency Operation
```jsonl
{
  "id": "bd-c3d4",
  "type": "dependency",
  "timestamp": "2026-02-06T11:30:00Z",
  "data": {
    "action": "add",
    "type": "blocks",
    "parent": "bd-a1b2",
    "child": "bd-c3d4"
  }
}
```

#### Comment Operation
```jsonl
{
  "id": "bd-a1b2", 
  "type": "comment",
  "timestamp": "2026-02-06T12:00:00Z",
  "data": {
    "action": "add",
    "comment": "Database schema designed, ready for implementation",
    "author": "agent-1"
  }
}
```

#### Label Operation
```jsonl
{
  "id": "bd-a1b2",
  "type": "label", 
  "timestamp": "2026-02-06T10:45:00Z",
  "data": {
    "action": "add",
    "label": "backend"
  }
}
```

#### Close Operation
```jsonl
{
  "id": "bd-a1b2",
  "type": "close",
  "timestamp": "2026-02-06T14:00:00Z",
  "data": {
    "reason": "Implemented successfully",
    "resolution": "completed"
  }
}
```

## 📁 JSONL Files Structure

### Primary Files
```
.beads/
├── issues.jsonl              # Main issue operations
├── interactions.jsonl        # Agent interaction log
├── config.yaml              # Configuration settings
├── routes.jsonl             # Multi-agent routing rules
└── formulas/                # Workflow templates
    ├── feature.formula.toml
    └── release.formula.toml
```

### issues.jsonl Format Details
```jsonl
# File structure:
# Header (optional)
# {"version": "1.0", "created": "2026-02-06T10:00:00Z"}

# Operations (chronological)
{"id": "bd-a1b2", "type": "create", ...}
{"id": "bd-a1b2", "type": "update", ...}
{"id": "bd-c3d4", "type": "create", ...}

# Continuous append - never modify existing lines
```

### interactions.jsonl Format
```jsonl
# Agent interaction audit trail
{"timestamp": "2026-02-06T10:30:00Z", "agent": "claude-1", "action": "update_issue", "issue_id": "bd-a1b2", "details": {"status": "in_progress"}}
{"timestamp": "2026-02-06T11:00:00Z", "agent": "claude-1", "action": "create_issue", "issue_id": "bd-c3d4", "details": {"title": "Found bug"}}
```

### routes.jsonl Format
```jsonl
# Multi-agent routing configuration
{"pattern": "frontend/**", "target": "frontend-repo", "priority": 10}
{"pattern": "backend/**", "target": "backend-repo", "priority": 10}
{"pattern": "*", "target": "main-repo", "priority": 0}
```

## 🔄 Append-Only Benefits

### Git Merge Compatibility
The append-only format dramatically reduces Git merge conflicts:

```bash
# Branch A adds issue
git checkout feature-a
bd create "Feature A issue"  
# → {"id": "bd-a1b2", "type": "create", ...}

# Branch B adds different issue  
git checkout feature-b
bd create "Feature B issue"
# → {"id": "bd-c3d4", "type": "create", ...}

# Git merge result: Clean
git checkout main
git merge feature-a    # Appends bd-a1b2 line
git merge feature-b    # Appends bd-c3d4 line - NO CONFLICT!
```

### Conflict Scenarios
**Simple Conflict (same issue modified):**
```jsonl
# Main branch:
{"id": "bd-a1b2", "type": "create", "data": {"status": "open"}}

# Feature branch:
{"id": "bd-a1b2", "type": "update", "data": {"status": "in_progress"}}

# Git conflict marker:
<<<<<<< HEAD
{"id": "bd-a1b2", "type": "create", "data": {"status": "open"}}
=======
{"id": "bd-a1b2", "type": "update", "data": {"status": "in_progress"}}
>>>>>>> feature-branch
```

**Resolution Strategy:**
1. Keep both lines (append-only principle)
2. SQLite rebuild resolves final state
3. `bd sync --import-only` handles resolution

### Multiple Operations on Same Issue
```jsonl
# All operations are appended, never modified:
{"id": "bd-a1b2", "type": "create", "timestamp": "10:00", "data": {"status": "open"}}
{"id": "bd-a1b2", "type": "update", "timestamp": "10:30", "data": {"status": "in_progress"}}  
{"id": "bd-a1b2", "type": "label", "timestamp": "10:45", "data": {"label": "backend"}}
{"id": "bd-a1b2", "type": "update", "timestamp": "11:00", "data": {"status": "completed"}}
{"id": "bd-a1b2", "type": "close", "timestamp": "11:15", "data": {"reason": "Done"}}
```

## 🔄 SQLite Rebuild Process

### JSONL → SQLite Transformation
The SQLite database is always rebuilt from JSONL:

```bash
# Rebuild process (triggered by bd sync --import-only)
1. Parse issues.jsonl line by line
2. Apply operations in chronological order
3. Build final state in SQLite tables
4. Create indexes for fast queries
```

### Rebuild Algorithm
```python
def rebuild_sqlite_from_jsonl(jsonl_file, sqlite_db):
    # Clear existing database
    sqlite_db.execute("DELETE FROM issues")
    sqlite_db.execute("DELETE FROM dependencies") 
    sqlite_db.execute("DELETE FROM labels")
    
    # Process each operation chronologically
    for line in jsonl_file:
        operation = json.loads(line)
        
        if operation['type'] == 'create':
            create_issue(operation['data'])
        elif operation['type'] == 'update':
            update_issue(operation['id'], operation['data'])
        elif operation['type'] == 'dependency':
            handle_dependency(operation['data'])
        elif operation['type'] == 'label':
            handle_label(operation['id'], operation['data'])
        # ... other operation types
```

### Performance Considerations
```bash
# Small repositories (< 1000 issues):
Rebuild time: < 1 second
Memory usage: < 10MB

# Medium repositories (1000-10000 issues):  
Rebuild time: 1-5 seconds
Memory usage: 10-50MB

# Large repositories (> 10000 issues):
Rebuild time: 5-30 seconds  
Memory usage: 50-200MB
```

## 📊 File Size and Growth

### Size Estimates
```bash
# Average operation size: ~200 bytes
# Typical issue lifecycle: ~5 operations
# Per issue: ~1KB

# Repository size estimates:
100 issues   ≈ 100KB
1,000 issues ≈ 1MB  
10,000 issues ≈ 10MB
100,000 issues ≈ 100MB
```

### Growth Patterns
```jsonl
# Linear growth over time:
Week 1:  {"id": "bd-001", ...}
Week 2:  {"id": "bd-002", ...}
Week 3:  {"id": "bd-003", ...}
# + ~1KB per issue created

# Operational growth:
Issue bd-001: 5 operations over lifetime
Issue bd-002: 8 operations over lifetime  
Issue bd-003: 3 operations over lifetime
# + ~200 bytes per operation
```

### Compaction Strategies
```bash
# Remove closed issues older than date
bd compact --before 2025-01-01 --status closed

# Archive to separate file  
bd archive --to old-issues.jsonl --before 2024-01-01

# Compact in place (keeps history)
bd compact --in-place  # Removes redundant operations
```

## 🛡️ Data Integrity

### JSONL Validation
```bash
# Beads validates JSONL format automatically
# Pre-commit hook ensures valid JSON:

# Invalid JSON (rejected):
{"id": "bd-a1b2", "type": "create", "data": {"title": "Test"  # Missing closing brace

# Valid JSON (accepted):
{"id": "bd-a1b2", "type": "create", "data": {"title": "Test"}}
```

### Corruption Detection
```bash
# Check JSONL integrity
bd check --jsonl

# Output example:
✓ issues.jsonl: Valid JSONL format (1,247 lines)
✓ interactions.jsonl: Valid JSONL format (3,421 lines)  
✗ routes.jsonl: Invalid JSON at line 3
```

### Recovery from Corruption
```bash
# If JSONL is corrupted but Git history is clean:
git checkout HEAD~1 -- .beads/issues.jsonl
bd sync --import-only

# If Git history also has issues:
git log --oneline -- .beads/issues.jsonl
# Find last good commit and restore
git checkout <good-commit> -- .beads/issues.jsonl
```

## 🔧 Operational Commands

### Direct JSONL Operations
```bash
# View raw JSONL content
cat .beads/issues.jsonl

# Search JSONL with grep
grep '"status":"in_progress"' .beads/issues.jsonl

# Count operations by type
grep -o '"type":"[^"]*"' .beads/issues.jsonl | sort | uniq -c
```

### JSONL Analysis Tools
```bash
# Parse with jq
cat .beads/issues.jsonl | jq '.type' | sort | uniq -c

# Extract all issue titles  
cat .beads/issues.jsonl | jq 'select(.type=="create") | .data.title'

# Find operations on specific issue
cat .beads/issues.jsonl | jq 'select(.id=="bd-a1b2")'
```

### JSONL Backup Strategies
```bash
# Time-based backups
cp .beads/issues.jsonl .beads/issues-$(date +%Y%m%d).jsonl

# Compressed backups
gzip -c .beads/issues.jsonl > backups/issues-$(date +%Y%m%d).jsonl.gz

# Git-based automatic backup
git add .beads/issues.jsonl && git commit -m "Backup $(date)"
```

## 🔄 Sync Integration

### JSONL as Sync Source
```bash
# JSONL is authoritative for sync:
bd sync --import-only

# Process:
1. Read current JSONL state
2. Compare with SQLite state
3. Rebuild SQLite from JSONL if needed
4. Ignore SQLite-only changes
```

### Git Commit Flow
```bash
# User operation:
bd create "New issue"

# Immediate effect:
1. SQLite updated instantly
2. JSONL appended with create operation  
3. Git staging area updated

# On sync:
git add .beads/issues.jsonl
git commit -m "Add issue bd-a1b2: New issue"
git push
```

### Merge Conflict Resolution
```bash
# During Git merge:
<<<<<<< HEAD
{"id": "bd-a1b2", "type": "update", "data": {"status": "in_progress"}}
=======
{"id": "bd-a1b2", "type": "update", "data": {"status": "completed"}}
>>>>>>> feature-branch

# Resolution (keep both, let SQLite sort out):
{"id": "bd-a1b2", "type": "update", "data": {"status": "in_progress"}}
{"id": "bd-a1b2", "type": "update", "data": {"status": "completed"}}

# Final state after rebuild:
git add .beads/issues.jsonl
bd sync --import-only  # SQLite shows "completed"
```

## 🔗 Related Documentation

- [Git Layer](git-layer.md) - Historical storage and versioning
- [SQLite Layer](sqlite-layer.md) - Fast query database
- [Data Flow](data-flow.md) - Complete system flow diagrams
- [Recovery Overview](../recovery/) - JSONL-based recovery procedures
- [CLI Reference](../cli-reference/) - Commands for JSONL operations

## 📚 See Also

- [Architecture Overview](overview.md) - Complete three-layer system
- [Daemon System](daemon-system.md) - Background JSONL monitoring
- [Multi-Agent Coordination](../multi-agent/) - JSONL in multi-agent workflows
- [Extension Points](../extension-points/) - Custom JSONL integration