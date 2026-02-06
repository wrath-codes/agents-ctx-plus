# Molecules

Molecules are persistent workflow instances created from formulas. They represent the "Mol" (liquid) phase in Beads' chemistry metaphor, tracking active work through defined steps.

## 🧬 What is a Molecule?

A molecule is a concrete, persistent instance of a formula:

- **Created from formulas** using `bd pour`
- **Contains steps** with dependencies
- **Tracked in `.beads/`** (syncs with git)
- **Steps map to issues** with parent-child relationships
- **Progresses through workflow** step by step

## 🔄 Molecule Lifecycle

```
Formula (Proto - Template)
    ↓ bd pour
Molecule (Mol - Instance)
    ↓ Work progresses through steps
Completed Molecule
    ↓ Optional cleanup
Archived / Deleted
```

### Phase 1: Creation (Pour)

```bash
# Create molecule from formula
bd pour feature-workflow --var feature_name="dark-mode"

# What happens:
# 1. Formula "feature-workflow" is read
# 2. Variables substituted ({{feature_name}} → "dark-mode")
# 3. Parent issue created: bd-mol-abc (molecule root)
# 4. Child issues created for each step:
#    - bd-mol-abc.1: Design dark mode
#    - bd-mol-abc.2: Implement dark mode
#    - bd-mol-abc.3: Test dark mode
# 5. Dependencies set up according to formula
# 6. All issues saved to .beads/issues.jsonl
```

### Phase 2: Execution (Flow)

```bash
# View molecule structure
bd mol show bd-mol-abc

# Output:
Molecule: bd-mol-abc
Formula: feature-workflow
Status: in_progress
Progress: 1/3 steps complete

Steps:
  ✓ bd-mol-abc.1: Design dark mode [closed]
  ▶ bd-mol-abc.2: Implement dark mode [in_progress]
  ○ bd-mol-abc.3: Test dark mode [open]

Dependencies:
  bd-mol-abc.2 → bd-mol-abc.1
  bd-mol-abc.3 → bd-mol-abc.2
```

### Phase 3: Completion

```bash
# When all steps complete
bd close bd-mol-abc.3

# Molecule automatically marked complete
# Can be archived or kept for reference
```

## 🎯 Creating Molecules

### Basic Creation

```bash
# Simple pour
bd pour feature-workflow

# With variables
bd pour feature-workflow --var feature_name="dark-mode"

# Multiple variables
bd pour release-workflow \
  --var version="1.0.0" \
  --var environment="production"
```

### Dry Run (Preview)

```bash
# Preview what would be created
bd pour feature-workflow --var feature_name="test" --dry-run

# Output:
Would create molecule with:
  Parent: bd-mol-xyz (Dark Mode Implementation)
  Children:
    - bd-mol-xyz.1: Design dark mode
    - bd-mol-xyz.2: Implement dark mode
    - bd-mol-xyz.3: Test dark mode
  Dependencies:
    - bd-mol-xyz.2 → bd-mol-xyz.1
    - bd-mol-xyz.3 → bd-mol-xyz.2

Use --confirm to create
```

### Creating from Different Formula Sources

```bash
# Project-level formula
bd pour .beads/formulas/custom.formula.toml

# User-level formula
bd pour ~/.beads/formulas/standup.formula.toml

# Built-in formula
bd pour feature-workflow
```

## 📋 Working with Molecules

### Listing Molecules

```bash
# List all active molecules
bd mol list

# Output:
Active Molecules (3):
  bd-mol-001: Feature - Dark Mode [2/3 complete]
  bd-mol-002: Release - v1.0.0 [1/5 complete]
  bd-mol-003: Bug Fix - Auth Issue [0/4 complete]

# JSON output
bd mol list --json

# Filter by status
bd mol list --status in_progress
bd mol list --status complete

# Filter by formula
bd mol list --formula feature-workflow
```

### Viewing Molecule Details

```bash
# Show molecule overview
bd mol show bd-mol-001

# Full details with all steps
bd mol show bd-mol-001 --full

# Show as tree
bd dep tree bd-mol-001

# Output:
bd-mol-001: Dark Mode Implementation
├── bd-mol-001.1: Design dark mode [closed]
├── bd-mol-001.2: Implement dark mode [in_progress]
│   └── bd-mol-001.2.1: Add CSS variables [in_progress]
├── bd-mol-001.3: Test dark mode [open]
└── bd-mol-001.4: Deploy dark mode [blocked]
    └── blocked by: bd-mol-001.3
```

### Progressing Through Steps

```bash
# Check ready steps
bd ready

# Output:
Ready work (1):
  [P1] bd-mol-001.2: Implement dark mode

# Start work on step
bd update bd-mol-001.2 --status in_progress

# Mark as complete
bd close bd-mol-001.2 --reason "Implementation complete"

# Next step automatically becomes ready
bd ready
# Output:
Ready work (1):
  [P1] bd-mol-001.3: Test dark mode
```

## 🔗 Step Dependencies

### Sequential Steps

```toml
# Formula with sequential steps
[[steps]]
id = "design"
title = "Design"

[[steps]]
id = "implement"
title = "Implement"
needs = ["design"]

[[steps]]
id = "test"
title = "Test"
needs = ["implement"]
```

**Execution**:
```
Step 1: Design [open] ← Start here
Step 2: Implement [blocked] ← waits for design
Step 3: Test [blocked] ← waits for implement
```

### Parallel Steps

```toml
# Formula with parallel steps
[[steps]]
id = "test-a"
title = "Test suite A"

[[steps]]
id = "test-b"
title = "Test suite B"

[[steps]]
id = "report"
title = "Generate report"
waits_for = ["test-a", "test-b"]
```

**Execution**:
```
Step 1: Test A [open] ─┐
                       ├→ Step 3: Report [blocked]
Step 2: Test B [open] ─┘
```

### Complex Dependencies

```toml
# Formula with complex dependencies
[[steps]]
id = "foundation"

[[steps]]
id = "wall-a"
needs = ["foundation"]

[[steps]]
id = "wall-b"
needs = ["foundation"]

[[steps]]
id = "roof"
needs = ["wall-a", "wall-b"]

[[steps]]
id = "paint"
needs = ["roof"]
```

**Execution**:
```
                    ┌→ Wall A ─┐
Foundation ─┤          ├→ Roof → Paint
                    └→ Wall B ─┘
```

## 🎛️ Advanced Molecule Features

### Step Hooks

```bash
# Execute action on step completion
bd mol hook bd-mol-001.2 --on-complete "notify-slack"

# Step-specific commands
bd mol run bd-mol-001.2 --command "make test"
```

### Step Variables

```bash
# Override step variables
bd mol update bd-mol-001.2 --var assignee="agent-2"

# Step-specific metadata
bd mol update bd-mol-001.2 --metadata "{\"complexity\": \"high\"}"
```

### Dynamic Steps

```bash
# Add step to existing molecule
bd mol add-step bd-mol-001 \
  --id "documentation" \
  --title "Write documentation" \
  --needs "bd-mol-001.2"

# Remove step
bd mol remove-step bd-mol-001.4

# Reorder steps
bd mol reorder bd-mol-001 --steps "1,3,2,4"
```

## 📊 Progress Tracking

### Viewing Progress

```bash
# Molecule statistics
bd mol stats bd-mol-001

# Output:
Molecule: bd-mol-001
Formula: feature-workflow
Created: 2026-02-06 10:00
Started: 2026-02-06 10:30

Progress:
  Total steps: 5
  Completed: 2 (40%)
  In progress: 1 (20%)
  Open: 2 (40%)
  Blocked: 0 (0%)

Estimated completion: 2026-02-08 (2 days)
```

### Blocked Steps

```bash
# Show blocked steps
bd blocked --molecule bd-mol-001

# Output:
Blocked steps in bd-mol-001:
  bd-mol-001.4: Deploy
    Blocked by: bd-mol-001.3 (Test)
    
  bd-mol-001.5: Announce
    Blocked by: bd-mol-001.4 (Deploy)
```

### Timeline View

```bash
# Show molecule timeline
bd mol timeline bd-mol-001

# Output:
Timeline for bd-mol-001:
Day 1 (2026-02-06):
  10:00 - Created
  10:30 - Started bd-mol-001.1 (Design)
  16:00 - Completed bd-mol-001.1
  
Day 2 (2026-02-07):
  09:00 - Started bd-mol-001.2 (Implement)
  [IN PROGRESS]
```

## 🏷️ Pinning and Assignment

### Pinning Work

```bash
# Pin molecule to agent
bd pin bd-mol-001 --for agent-1 --start

# Pin specific step
bd pin bd-mol-001.2 --for agent-1 --start

# Check pinned work
bd hook

# Output:
Your pinned work (2):
  bd-mol-001: Dark Mode Implementation [started]
  bd-mol-002: Release v1.0 [pinned]
```

### Multi-Agent Coordination

```bash
# Assign different steps to different agents
bd pin bd-mol-001.2 --for backend-agent --start
bd pin bd-mol-001.3 --for qa-agent

# View assignments
bd mol assignments bd-mol-001

# Output:
Assignments for bd-mol-001:
  bd-mol-001.2: backend-agent [in_progress]
  bd-mol-001.3: qa-agent [open]
  bd-mol-001.4: unassigned
```

## 🧹 Molecule Maintenance

### Archiving

```bash
# Archive completed molecule
bd mol archive bd-mol-001

# Archive with reason
bd mol archive bd-mol-001 --reason "Feature shipped in v1.2.0"

# List archived molecules
bd mol list --archived
```

### Deletion

```bash
# Delete molecule (caution: destructive)
bd mol delete bd-mol-001

# Force delete
bd mol delete bd-mol-001 --force

# Delete with cascade (remove all children)
bd mol delete bd-mol-001 --cascade
```

### Cloning

```bash
# Clone molecule for similar work
bd mol clone bd-mol-001 --as bd-mol-002

# Clone with modifications
bd mol clone bd-mol-001 --var feature_name="light-mode"
```

## 🔄 Molecule Transitions

### Mol → Wisp (Demote)

```bash
# Convert molecule to ephemeral wisp
bd mol convert bd-mol-001 --to wisp

# Use case: Experiment became unnecessary
```

### Mol → Formula (Extract)

```bash
# Extract molecule as new formula
bd mol extract bd-mol-001 --as improved-feature-workflow

# Use case: Generalize successful workflow
```

### Formula → Mol (Promote)

```bash
# Already covered by bd pour
bd pour improved-feature-workflow
```

## 🎯 Best Practices

### Molecule Creation

**DO**:
```bash
# Use descriptive variable values
bd pour feature-workflow --var feature_name="user-authentication"

# Check dry-run before creating
bd pour workflow --var x=y --dry-run

# Use appropriate formulas for work type
bd pour bug-fix-workflow --var bug_id="1234"
```

**DON'T**:
```bash
# Don't create molecules for single tasks
bd pour single-step-formula  # Use regular issue instead

# Don't forget to update progress
bd close bd-mol-001.1  # Always close completed steps

# Don't leave molecules incomplete
bd mol archive old-molecule  # Clean up old work
```

### Step Management

**DO**:
```bash
# Work through steps sequentially
bd ready  # Check what's ready
bd update step --status in_progress
# ... do work ...
bd close step

# Document progress
bd comment add bd-mol-001.2 "50% complete"
```

**DON'T**:
```bash
# Don't skip steps
bd close bd-mol-001.2  # Without completing bd-mol-001.1

# Don't work on blocked steps
bd update bd-mol-001.3 --status in_progress  # If blocked
```

## 📈 Molecule Analytics

### Performance Metrics

```bash
# Molecule velocity
bd mol velocity bd-mol-001

# Output:
Molecule Velocity:
  Started: 2026-02-06
  Current step: 2 of 5
  Days elapsed: 2
  Steps completed: 1
  Velocity: 0.5 steps/day
  Estimated completion: 2026-02-12 (6 more days)
```

### Bottleneck Analysis

```bash
# Find bottlenecks
bd mol bottlenecks

# Output:
Top bottlenecks:
  1. bd-mol-001: Blocked 3 days waiting for review
  2. bd-mol-002: Blocked 2 days on external dependency
```

### Efficiency Metrics

```bash
# Molecule efficiency
bd mol efficiency

# Output:
Efficiency Report:
  Average step duration: 1.5 days
  Blocked time: 20%
  Active time: 60%
  Review time: 20%
```

## 🔗 Related Documentation

- [Chemistry Metaphor](chemistry-metaphor.md) - Molecule phase overview
- [Formulas](formulas.md) - Creating molecules from templates
- [Gates](gates.md) - Async coordination in molecules
- [Multi-Agent](../multi-agent/) - Multi-agent molecule coordination

## 📚 See Also

- [CLI Reference](../cli-reference/workflow-commands.md) - Molecule commands
- [Best Practices](../best-practices/) - Workflow patterns
- [Context Enhancement](../context-enhancement/) - Molecule automation