# Git Repository Layer

This layer serves as the **historical source of truth** for Beads, storing all issue data in Git alongside code for full version history and offline capability.

## 🗂️ Role in Three-Layer Architecture

The Git layer is Layer 1 in Beads' architecture:

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

**Key Characteristic**: Git preserves the *complete history* of all issue changes, making any past state recoverable.

## 📁 Git-Tracked Files

### Primary Files
```
.beads/
├── issues.jsonl          # Main issue data (append-only)
├── config.yaml           # User configuration
├── interactions.jsonl    # Agent audit log
├── routes.jsonl          # Multi-agent routing rules
└── formulas/            # Custom workflow templates
    ├── feature.formula.toml
    ├── release.formula.toml
    └── ...
```

### Git-ignored Files
```
.beads/
├── beads.db*           # SQLite database (rebuildable)
├── daemon.log          # Daemon activity logs
├── .lock              # Process locks
└── .wisp/             # Ephemeral workflows
```

## 🔄 Git Integration Benefits

### 1. Issues Travel with Code
```bash
# Clone project with complete issue history
git clone https://github.com/user/project.git
cd project

# Issues are available immediately
bd list
```

**Benefits:**
- Context is always with relevant code
- No separate issue tracking system to maintain
- Historical context preserved with code snapshots

### 2. Full Version History
```bash
# See issue evolution over time
git log --follow -- .beads/issues.jsonl

# Check out project state from 6 months ago
git checkout HEAD~6.months
bd list  # Shows issues as they were then
```

**Capabilities:**
- Complete audit trail of all changes
- Ability to reconstruct any historical state
- Blame/annotation on issue changes
- Branch-specific issue tracking

### 3. Branch and Merge Support
```bash
# Create feature branch with its own issues
git checkout -b feature/new-api
bd create "Add user endpoints"  # Tracked only in this branch

# Main branch continues independently
git checkout main
bd create "Fix database bug"     # Separate from feature work

# Merge brings both issue streams together
git merge feature/new-api  # Issues merge cleanly
```

**Use Cases:**
- Feature-specific issue tracking
- Experimental work isolation
- Parallel development streams
- PR-based issue management

### 4. Offline-First Operation
```bash
# Work completely offline
 airplane mode enabled
bd create "Design offline workflow"
bd update bd-xyz --status in_progress
bd list  # All operations work

# Sync when back online
git push  # Shares all offline work
```

**Advantages:**
- No network dependency for core operations
- Full functionality on planes, in restricted networks
- Local development without interruption

## 📝 JSONL in Git

### Append-Only Format
The JSONL format is specifically chosen for Git compatibility:

```bash
# Branch A creates issue
git checkout feature-a
bd create "Feature A issue"
git add .beads/issues.jsonl && git commit -m "Add feature A issue"

# Branch B creates issue (simultaneously)
git checkout feature-b  
bd create "Feature B issue"
git add .beads/issues.jsonl && git commit -m "Add feature B issue"

# Git merges cleanly - just appends both additions
git checkout main
git merge feature-a  # Adds issue A
git merge feature-b  # Adds issue B - no conflict!
```

### Merge Conflict Prevention
The append-only format dramatically reduces merge conflicts:

**No Conflict Scenario:**
```jsonl
# Main branch:
{"id": "bd-a1b2", "type": "create", ...}

# Feature branch adds new issue:
{"id": "bd-a1b2", "type": "create", ...}
{"id": "bd-c3d4", "type": "create", ...}  # New line added

# Git result: Clean merge
{"id": "bd-a1b2", "type": "create", ...}
{"id": "bd-c3d4", "type": "create", ...}
```

**Conflict Scenario (rare):**
```jsonl
# Both branches modify same issue simultaneously
# Main: {"id": "bd-a1b2", "type": "create", "data": {"status": "open"}}
# Feature: {"id": "bd-a1b2", "type": "update", "data": {"status": "in_progress"}}
```

**Conflict Resolution:**
1. Git flags conflict in `.beads/issues.jsonl`
2. Manual resolution needed (rare)
3. `bd sync --import-only` rebuilds SQLite from resolved JSONL

## 🔧 Git Hooks Integration

### Automatic Setup
```bash
bd init  # Automatically installs Git hooks
```

### Installed Hooks
```bash
# Pre-commit hook: Ensures issues.jsonl is valid JSONL
.beads-hooks/pre-commit

# Post-checkout hook: Rebuilds SQLite for new branch
.beads-hooks/post-checkout  

# Post-merge hook: Handles JSONL merges
.beads-hooks/post-merge
```

### Hook Behavior
```bash
# Pre-commit: Validates JSONL format
git commit -m "Add new issue"
# → Validates JSONL syntax
# → Prevents malformed commits

# Post-checkout: Switches branch context
git checkout feature-branch
# → Rebuilds SQLite for branch
# → Updates working set of issues
```

## 🔄 Git Workflow Patterns

### 1. Standard Development
```bash
# 1. Start work
git checkout -b feature/user-auth
bd create "Implement user authentication" --parent epic-123

# 2. Work and commit
git commit -am "Add auth endpoints"

# 3. Sync issues
bd sync  # Commits JSONL and pushes

# 4. Create PR (includes issues in code review)
gh pr create --title "Add user authentication"
```

### 2. Multi-Agent Collaboration
```bash
# Agent A works on backend
bd create "Add user API" --label backend
bd sync  # Shares work

# Agent B picks up frontend work
git pull  # Gets Agent A's changes
bd create "Create login UI" --deps discovered-from:bd-agent-a-issue
bd sync  # Shares both agents' work
```

### 3. Feature Branch Isolation
```bash
# Experimental work
git checkout -b experiment/ai-workflow
bd create "Test AI-generated code" --status experimental

# Keep main clean
git checkout main
# Experimental issues don't appear in main branch
```

## 🔍 Git History Analysis

### Issue Evolution Tracking
```bash
# Track specific issue through history
git log --follow --patch -S "bd-a1b2"

# Show all issue changes in commit
git show HEAD -- .beads/issues.jsonl

# Blame issue changes
git blame .beads/issues.jsonl
```

### Branch-Specific Issues
```bash
# Compare issues between branches
git diff main..feature -- .beads/issues.jsonl

# Show issues only in feature branch
git log main..feature -- .beads/issues.jsonl
```

### Historical Queries
```bash
# What issues existed 2 weeks ago?
git checkout "HEAD@{2.weeks.ago}"
bd list

# When was issue created?
git log --reverse --grep="bd-a1b2" --oneline
```

## 🛡️ Backup and Recovery

### Git as Natural Backup
```bash
# Git provides automatic versioning
git log --oneline -- .beads/

# Recover from any point in history
git checkout <commit-hash>  # Complete historical state
```

### Remote Backup Strategy
```bash
# Multiple remotes for redundancy
git remote add origin git@github.com:user/project.git
git remote add backup git@gitlab.com:user/project.git
git remote add archive git@personal-server:user/project.git

# Push to all remotes
git push --all
git push --all --tags
```

### Disaster Recovery
```bash
# Complete system loss? Restore from Git:
git clone https://github.com/user/project.git
cd project
bd sync --import-only  # Rebuilds SQLite from JSONL
```

## 📊 Performance Considerations

### Repository Growth
```
.beads/issues.jsonl  # Grows linearly with operations
├── 1000 issues ≈ 500KB
├── 10,000 issues ≈ 5MB  
├── 100,000 issues ≈ 50MB
└── Compaction needed periodically
```

### Git Operations
```bash
# Fast operations:
git add .beads/issues.jsonl    # Always adds whole file
git commit -m "message"         # Fast, small metadata

# Slower with many issues:
git log -- .beads/issues.jsonl    # Scans all relevant commits
git blame .beads/issues.jsonl     # Processes large file
```

### Optimization Strategies
```bash
# Compaction: Remove old closed issues from main JSONL
bd compact --before 2024-01-01

# Archive: Move old issues to separate file
bd archive --to archive.jsonl --status closed
```

## 🚫 Limitations and Constraints

### Single Repository Scope
- Issues are scoped to a single Git repository
- Cross-repo dependencies require explicit routing
- No global issue visibility across multiple projects

### Merge Conflict Risk
- Multiple branches modifying same issue simultaneously
- Requires manual resolution (rare but possible)
- Append-only format reduces but doesn't eliminate conflicts

### Repository Size Growth
- JSONL files grow monotonically
- Requires periodic compaction
- Large repositories need archive strategies

### Performance with Many Issues
- Git operations slow with very large JSONL files
- SQLite queries remain fast
- History analysis becomes slower

## 🔗 Related Documentation

- [JSONL Layer](jsonl-layer.md) - Operational format details
- [SQLite Layer](sqlite-layer.md) - Database schema and performance
- [Data Flow](data-flow.md) - Complete flow diagrams
- [Recovery Overview](../recovery/) - Git-based recovery procedures
- [Multi-Repository](../multi-agent/multi-repository.md) - Cross-repo strategies

## 📚 See Also

- [Architecture Overview](overview.md) - Complete three-layer system
- [Daemon System](daemon-system.md) - Background synchronization
- [CLI Reference](../cli-reference/) - Commands with Git integration
- [Multi-Agent Coordination](../multi-agent/) - Collaboration patterns