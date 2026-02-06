# Workflows - Chemistry Metaphor

Beads uses a chemistry-inspired metaphor to organize workflow management, providing intuitive concepts for building complex, multi-step processes.

## 🧪 Chemistry-Inspired Workflow System

### The Three Phases

Beads organizes workflows into three phases based on the states of matter:

```
┌─────────────────────────────────────────────────────────────┐
│                    WORKFLOW CHEMISTRY                       │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│   Proto (Solid)          Mol (Liquid)         Wisp (Vapor) │
│   ─────────────          ───────────          ───────────  │
│                                                             │
│   • Reusable            • Persistent         • Ephemeral   │
│   • Templates           • Instances          • Temporary   │
│   • Definitions         • Execution          • Exploration │
│   • Formulas            • Molecules          • Operations  │
│                                                             │
│   Storage:               Storage:            Storage:      │
│   Built-in /             .beads/             .beads-wisp/  │
│   ~/.beads/formulas/                                  │
│                                                             │
│   Sync:                  Sync:               Sync:         │
│   Never                  Yes (Git)           No            │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

## 🧬 Phase 1: Proto (Solid) - Formulas

**Purpose**: Reusable workflow templates and definitions
**State**: Solid, unchanging, foundational
**Storage**: Built-in or `~/.beads/formulas/`

### Characteristics

- **Reusable**: Use the same formula multiple times
- **Immutable**: Formulas don't change during execution
- **Template-based**: Define structure, not specific instances
- **Versioned**: Can have multiple versions of same formula

### Examples

```toml
# Proto: A feature development formula
formula = "feature-workflow"
version = 1

[[steps]]
id = "design"
title = "Design {{feature_name}}"
type = "human"

[[steps]]
id = "implement"
title = "Implement {{feature_name}}"
needs = ["design"]
```

**Use Cases**:
- Release workflows
- Feature development templates
- Bug fix procedures
- Onboarding checklists
- Compliance processes

## 💧 Phase 2: Mol (Liquid) - Molecules

**Purpose**: Persistent workflow instances created from formulas
**State**: Liquid, flowing, adaptable
**Storage**: `.beads/` directory (git-tracked)

### Characteristics

- **Instantiated**: Created from formulas using `bd pour`
- **Persistent**: Survives across sessions
- **Stateful**: Tracks progress through steps
- **Syncable**: Travels with Git repository

### Examples

```bash
# Create molecule from formula
bd pour feature-workflow --var feature_name="dark-mode"

# Results in:
# bd-mol-abc: Dark Mode Implementation
#   ├── bd-mol-abc.1: Design dark mode [in_progress]
#   ├── bd-mol-abc.2: Implement dark mode [open]
#   └── bd-mol-abc.3: Test dark mode [open]
```

**Use Cases**:
- Active feature development
- Release processes in progress
- Multi-step bug fixes
- Long-running initiatives
- Epic-level work tracking

## ☁️ Phase 3: Wisp (Vapor) - Ephemeral Operations

**Purpose**: Temporary, exploratory operations that don't need persistence
**State**: Vapor, fleeting, exploratory
**Storage**: `.beads-wisp/` directory (gitignored)

### Characteristics

- **Ephemeral**: Auto-expires after completion
- **Non-syncing**: Never committed to Git
- **Experimental**: Try things without affecting main workflow
- **Lightweight**: Quick to create and destroy

### Examples

```bash
# Create wisp for exploration
bd wisp "Try alternative implementation"

# Work on wisp
bd update wisp-001 --status in_progress

# Wisp automatically cleaned up after completion
bd close wisp-001  # → Removed from system
```

**Use Cases**:
- Spikes and experiments
- Temporary workarounds
- Quick fixes
- Exploration tasks
- Proof-of-concepts

## 🔄 Phase Transitions

### Proto → Mol (Pour)

```bash
# Formula (Proto) → Molecule (Mol)
bd pour feature-workflow --var feature_name="dark-mode"

# Process:
# 1. Read formula definition
# 2. Substitute variables
# 3. Create parent issue (molecule root)
# 4. Create child issues (steps)
# 5. Set up dependencies
# 6. Save to .beads/ (git-tracked)
```

### Mol → Proto (Extract)

```bash
# Molecule (Mol) → Formula (Proto)
bd mol extract bd-mol-abc --as new-formula

# Process:
# 1. Analyze molecule structure
# 2. Generalize specific values
# 3. Create formula template
# 4. Save to .beads/formulas/
```

### Mol → Wisp (Convert)

```bash
# Convert molecule to wisp (demote)
bd mol convert bd-mol-abc --to wisp

# Process:
# 1. Move from .beads/ to .beads-wisp/
# 2. Remove from Git tracking
# 3. Mark as ephemeral
```

### Wisp → Mol (Promote)

```bash
# Promote wisp to molecule
bd wisp promote wisp-001 --to-mol

# Process:
# 1. Move from .beads-wisp/ to .beads/
# 2. Add to Git tracking
# 3. Make persistent
```

## 🎯 When to Use Each Phase

### Use Proto (Formulas) When:

✅ **Standardized Processes**
- Release procedures used multiple times
- Feature development templates
- Onboarding workflows
- Compliance checklists

✅ **Team Consistency**
- Everyone uses same process
- Reduces setup overhead
- Ensures nothing is forgotten

✅ **Automation**
- Scripts can instantiate workflows
- CI/CD integration
- Template-driven development

### Use Mol (Molecules) When:

✅ **Active Work**
- Currently in-progress features
- Multi-step processes
- Long-running initiatives
- Cross-team coordination

✅ **Need Persistence**
- Work spans multiple sessions
- Multiple agents involved
- Requires tracking over time
- Needs to survive crashes

✅ **Requires Dependencies**
- Complex step ordering
- Parallel execution paths
- Gate coordination
- Milestone tracking

### Use Wisp (Ephemeral) When:

✅ **Exploration**
- Trying new approaches
- Quick experiments
- Proof-of-concepts
- Spikes and investigations

✅ **Temporary Work**
- Workarounds
- Hotfixes
- One-off tasks
- Cleanup operations

✅ **No Persistence Needed**
- Won't be referenced later
- Single session work
- Private experiments
- Testing ideas

## 📊 Phase Comparison

| Aspect | Proto (Formulas) | Mol (Molecules) | Wisp (Ephemeral) |
|--------|-----------------|-----------------|------------------|
| **Storage** | Built-in / User formulas | `.beads/` | `.beads-wisp/` |
| **Git Sync** | No | Yes | No |
| **Persistence** | Permanent | Permanent | Temporary |
| **Reusability** | High (template) | Single instance | Single use |
| **State** | Template | Instance | Ephemeral |
| **Creation** | Manual / Built-in | `bd pour` | `bd wisp` |
| **Lifetime** | Indefinite | Until completed | Until closed |
| **Dependencies** | Defined | Executed | Minimal |

## 🔄 Complete Workflow Example

### Scenario: Feature Development

```bash
# 1. Proto: Define the formula (once)
cat > .beads/formulas/feature.formula.toml << 'EOF'
formula = "feature-development"
description = "Standard feature development workflow"
version = 1

[[steps]]
id = "design"
title = "Design {{feature_name}}"
type = "human"
description = "Create technical design document"

[[steps]]
id = "implement"
title = "Implement {{feature_name}}"
needs = ["design"]

[[steps]]
id = "test"
title = "Test {{feature_name}}"
needs = ["implement"]

[[steps]]
id = "deploy"
title = "Deploy {{feature_name}}"
needs = ["test"]
type = "human"
EOF

# 2. Mol: Instantiate for specific feature
bd pour feature-development --var feature_name="dark-mode"
# → Creates bd-mol-001 with 4 child issues

# 3. Work through molecule
bd ready  # Shows bd-mol-001.1 (Design)
bd update bd-mol-001.1 --status in_progress
# ... complete design ...
bd close bd-mol-001.1

bd ready  # Shows bd-mol-001.2 (Implement)
bd update bd-mol-001.2 --status in_progress
# ... implement feature ...

# 4. Wisp: Quick experiment during implementation
bd wisp "Try CSS-only dark mode"
# → Creates wisp-001 (not synced)
# ... experiment ...
bd close wisp-001  # Auto-deleted

# 5. Continue with molecule
bd close bd-mol-001.2
bd ready  # Shows bd-mol-001.3 (Test)
# ... continue workflow ...
```

## 🎛️ Phase Management Commands

### Formula (Proto) Commands

```bash
# List available formulas
bd mol list

# Show formula definition
bd mol show-formula feature-development

# Create new formula
bd mol create-formula my-workflow

# Edit formula
bd mol edit-formula feature-development

# Delete formula
bd mol delete-formula old-workflow
```

### Molecule (Mol) Commands

```bash
# Create molecule from formula
bd pour feature-development --var key=value

# List active molecules
bd mol list

# Show molecule details
bd mol show bd-mol-001

# Progress through steps
bd update bd-mol-001.1 --status in_progress

# Archive completed molecule
bd mol archive bd-mol-001

# Delete molecule
bd mol delete bd-mol-001
```

### Wisp Commands

```bash
# Create wisp
bd wisp "Quick experiment"

# List active wisps
bd wisp list

# Show wisp details
bd wisp show wisp-001

# Promote to molecule
bd wisp promote wisp-001

# Archive (before auto-deletion)
bd wisp archive wisp-001

# Force delete
bd wisp delete wisp-001
```

## 🔄 Conversion Commands

```bash
# Formula → Molecule
bd pour formula-name

# Molecule → Formula
bd mol extract bd-mol-001 --as new-formula

# Molecule → Wisp
bd mol convert bd-mol-001 --to wisp

# Wisp → Molecule
bd wisp promote wisp-001 --to-mol

# Wisp → Formula
bd wisp extract wisp-001 --as new-formula
```

## 🎯 Best Practices

### Phase Selection

**DO**:
```bash
# Use formula for repeatable processes
bd pour release-workflow --var version="1.0.0"

# Use molecule for active multi-step work
bd pour feature-development --var feature="new-api"

# Use wisp for experiments
bd wisp "Try GraphQL instead of REST"
```

**DON'T**:
```bash
# Don't create molecule for single tasks
bd pour simple-task-formula  # Overkill

# Don't use wisp for important work
bd wisp "Critical security fix"  # Needs persistence!

# Don't skip formulas for repeated work
bd create "Release step 1"  # Should use formula instead
```

### Workflow Design

**DO**:
```bash
# Design formulas with clear steps
formula = "clear-steps"
[[steps]]
id = "design"
title = "Design"

[[steps]]
id = "implement"
title = "Implement"
needs = ["design"]
```

**DON'T**:
```bash
# Don't create overly complex formulas
# 20+ steps in single formula = hard to manage

# Don't mix concerns in one formula
# Feature formula shouldn't include deployment steps
```

## 🔗 Related Documentation

- [Formulas](formulas.md) - Formula definition and creation
- [Molecules](molecules.md) - Molecule management
- [Gates](gates.md) - Async coordination in workflows
- [Wisps](wisps.md) - Ephemeral workflow operations

## 📚 See Also

- [Multi-Agent](../multi-agent/) - Complex multi-agent workflows
- [Context Enhancement](../context-enhancement/) - Workflow automation
- [Best Practices](../best-practices/) - Workflow patterns and guidelines