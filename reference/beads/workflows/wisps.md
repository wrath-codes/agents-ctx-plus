# Wisps

Wisps are ephemeral workflow operations that don't sync to Git. They represent the "Wisp" (vapor) phase in Beads' chemistry metaphor, designed for temporary, exploratory work.

## ☁️ What are Wisps?

Wisps provide:

- **Ephemeral storage** in `.beads-wisp/` (gitignored)
- **No synchronization** - never committed to Git
- **Auto-expiration** after completion
- **Lightweight creation** for quick experiments
- **Private by default** - not shared with team

## 🎯 When to Use Wisps

### Use Wisps For:

✅ **Exploration and Spikes**
```bash
bd wisp "Try React 18 concurrent features"
# Experiment without affecting main workflow
```

✅ **Temporary Workarounds**
```bash
bd wisp "Hotfix for production issue"
# Quick fix while permanent solution is developed
```

✅ **Proof of Concepts**
```bash
bd wisp "POC: GraphQL vs REST"
# Compare approaches without committing
```

✅ **Personal Tasks**
```bash
bd wisp "Clean up local development environment"
# Personal maintenance tasks
```

✅ **Quick Experiments**
```bash
bd wisp "Test new library integration"
# Try something new
```

### Don't Use Wisps For:

❌ **Important Work**
- Critical bug fixes
- Feature development
- Production deployments

❌ **Collaborative Work**
- Work requiring team input
- Multi-agent coordination
- Shared milestones

❌ **Long-Running Tasks**
- Work spanning multiple days
- Complex multi-step processes
- Epic-level initiatives

## 📝 Creating Wisps

### Basic Wisp Creation

```bash
# Simple wisp
bd wisp "Quick experiment"

# Wisp with description
bd wisp "Try alternative approach" \
  --description "Explore using Redis instead of database"

# Wisp with metadata
bd wisp "Performance test" \
  --label "performance,experiment"
```

### Wisp Structure

```json
{
  "id": "wisp-001",
  "title": "Try alternative approach",
  "description": "Explore using Redis instead of database",
  "status": "open",
  "created_at": "2026-02-06T10:00:00Z",
  "expires_at": "2026-02-07T10:00:00Z",
  "labels": ["experiment"],
  "source_mol": "bd-mol-123"  // Optional: derived from molecule
}
```

## 🔧 Working with Wisps

### Listing Wisps

```bash
# List active wisps
bd wisp list

# Output:
Active Wisps (3):
  wisp-001: Try alternative approach [open]
  wisp-002: Performance test [in_progress]
  wisp-003: Quick bug fix [open]

# JSON output
bd wisp list --json

# Filter by status
bd wisp list --status open
bd wisp list --status in_progress

# Show expired wisps
bd wisp list --expired
```

### Viewing Wisp Details

```bash
# Show wisp details
bd wisp show wisp-001

# Output:
Wisp: wisp-001
Title: Try alternative approach
Status: open
Created: 2026-02-06 10:00
Expires: 2026-02-07 10:00
Location: .beads-wisp/wisp-001.json

Description: Explore using Redis instead of database
Labels: experiment
```

### Updating Wisps

```bash
# Update status
bd wisp update wisp-001 --status in_progress

# Add progress
bd wisp update wisp-001 --progress "50% complete"

# Add notes
bd wisp comment wisp-001 "Redis shows 10x performance improvement"

# Extend expiration
bd wisp update wisp-001 --extend 24h
```

## 🔄 Wisp Lifecycle

### Automatic Expiration

```bash
# Wisps auto-expire based on configuration
# Default: 24 hours after creation

# Expiration process:
# 1. Wisp created: expires_at = now + 24h
# 2. Wisp closed: expires_at = now + 1h (grace period)
# 3. After expiration: auto-deleted
```

### Manual Expiration

```bash
# Force immediate expiration
bd wisp expire wisp-001

# Archive before expiration
bd wisp archive wisp-001 --reason "Worth keeping"

# Delete immediately
bd wisp delete wisp-001
```

## 🎛️ Wisp Configuration

### Default Settings

```yaml
# .beads/config.yaml
wisp:
  default_ttl: "24h"        # Time until auto-expiration
  grace_period: "1h"        # Extra time after closure
  max_active: 10            # Max simultaneous wisps
  auto_cleanup: true        # Auto-delete expired wisps
  archive_on_close: false   # Archive instead of delete
```

### TTL Options

```bash
# Create wisp with custom TTL
bd wisp "Long experiment" --ttl 7d
bd wisp "Quick test" --ttl 1h
bd wisp "Weekend project" --ttl 48h
```

## 🔄 Wisp Transitions

### Wisp → Molecule (Promote)

When a wisp proves valuable, promote it to a persistent molecule:

```bash
# Promote wisp to molecule
bd wisp promote wisp-001 --to-mol

# What happens:
# 1. Move from .beads-wisp/ to .beads/
# 2. Convert to regular issue ID (bd-xxx)
# 3. Add to Git tracking
# 4. Sync to remote
```

**Use Case**:
```bash
# Started as experiment
bd wisp "Try GraphQL"
# ... experiment successful ...

# Promote to real work
bd wisp promote wisp-001 --to-mol
# Now it's bd-xxx and part of main workflow
```

### Wisp → Formula (Extract)

Extract a successful wisp pattern as a reusable formula:

```bash
# Extract wisp as formula
bd wisp extract wisp-001 --as performance-test-formula

# Creates: .beads/formulas/performance-test.formula.toml
```

### Mol → Wisp (Demote)

Demote a molecule to wisp when it becomes unnecessary:

```bash
# Demote molecule to wisp
bd mol convert bd-mol-001 --to wisp

# What happens:
# 1. Move from .beads/ to .beads-wisp/
# 2. Remove from Git tracking
# 3. Set expiration
# 4. Mark as ephemeral
```

## 📊 Wisp Analytics

### Usage Statistics

```bash
# Wisp statistics
bd wisp stats

# Output:
Wisp Statistics:
Total wisps created: 156
Currently active: 3
Average lifetime: 4.2 hours
Conversion to molecules: 12 (7.7%)
Expired without action: 89 (57%)
Deleted manually: 55 (35.3%)
```

### Conversion Tracking

```bash
# Track which wisps became important
bd wisp conversions

# Output:
Promoted Wisps:
1. wisp-042 → bd-189: "GraphQL migration POC"
   Promoted: 2026-02-01
   Reason: Successful performance gains

2. wisp-067 → bd-201: "New caching strategy"
   Promoted: 2026-02-03
   Reason: Production-ready solution
```

## 🎯 Best Practices

### Wisp Creation

**DO**:
```bash
# Use descriptive titles
bd wisp "Compare Redis vs Memcached for session storage"

# Set appropriate TTL
bd wisp "Quick test" --ttl 2h
bd wisp "Weekend project" --ttl 48h

# Add context
bd wisp "Experiment" --description "Trying new approach for issue bd-001"
```

**DON'T**:
```bash
# Don't use wisps for critical work
bd wisp "Fix production outage"  # Should be regular issue!

# Don't create too many wisps
# Max 10 active wisps recommended

# Don't let wisps accumulate
bd wisp list --expired  # Clean up regularly
```

### Wisp Management

**DO**:
```bash
# Close wisps when done
bd close wisp-001

# Promote valuable wisps
bd wisp promote wisp-001

# Clean up expired wisps
bd wisp cleanup --expired

# Archive interesting experiments
bd wisp archive wisp-001
```

**DON'T**:
```bash
# Don't leave wisps open indefinitely
bd wisp list  # Shows many old wisps

# Don't lose track of wisps
# They're not in Git, easy to forget

# Don't promote every wisp
# Only promote truly valuable work
```

## 🧹 Wisp Cleanup

### Automatic Cleanup

```bash
# Configure auto-cleanup
bd config set wisp.auto_cleanup true
bd config set wisp.cleanup_interval "24h"

# What gets cleaned:
# - Expired wisps (past TTL)
# - Closed wisps (after grace period)
# - Orphaned wisps (no recent activity)
```

### Manual Cleanup

```bash
# Clean up expired wisps
bd wisp cleanup

# Clean up all closed wisps
bd wisp cleanup --closed

# Clean up specific wisp
bd wisp delete wisp-001

# Clean up by age
bd wisp cleanup --older-than 7d
```

## 🔗 Integration with Molecules

### Creating Wisps from Molecules

```bash
# During molecule work, create wisp for exploration
bd wisp "Try alternative implementation" \
  --source-mol bd-mol-001

# Wisp tracks which molecule it came from
# Allows context preservation
```

### Converting Back

```bash
# If wisp is successful, integrate back
bd wisp promote wisp-001

# Update original molecule
bd comment add bd-mol-001 \
  "Found better approach in wisp-001, promoted to bd-002"
```

## 🎨 Wisp Use Cases

### Development Spikes

```bash
# Time-boxed exploration
bd wisp "Spike: Evaluate new UI library" --ttl 4h

# Work on spike
bd wisp update wisp-001 --status in_progress
# ... try library ...

# Document findings
bd wisp comment wisp-001 "Library looks good, recommend adoption"

# Close or promote
bd close wisp-001
# or
bd wisp promote wisp-001
```

### A/B Testing Ideas

```bash
# Test different approaches
bd wisp "Approach A: Server-side rendering"
bd wisp "Approach B: Client-side rendering"

# Compare results
bd wisp list
# ... analyze both ...

# Promote winner
bd wisp promote wisp-002
```

### Personal Organization

```bash
# Personal tasks not part of main workflow
bd wisp "Clean up branches"
bd wisp "Update local dependencies"
bd wisp "Review PRs"

# Work through personal tasks
bd wisp list
bd wisp update wisp-001 --status in_progress
bd close wisp-001
```

### Hotfixes

```bash
# Quick production fix
bd wisp "Hotfix: Fix login bug" --ttl 2h

# Implement fix
bd wisp update wisp-001 --status in_progress
# ... write fix ...

# Deploy immediately
# (separate from normal workflow)

# Document for permanent fix
bd wisp comment wisp-001 "Applied hotfix, permanent fix needed"
bd wisp promote wisp-001  # Convert to proper issue
```

## 🔗 Related Documentation

- [Chemistry Metaphor](chemistry-metaphor.md) - Wisp phase overview
- [Formulas](formulas.md) - Creating reusable templates
- [Molecules](molecules.md) - Persistent workflow instances
- [Gates](gates.md) - Async coordination

## 📚 See Also

- [CLI Reference](../cli-reference/workflow-commands.md) - Wisp commands
- [Best Practices](../best-practices/) - Wisp usage patterns
- [Context Enhancement](../context-enhancement/) - Temporary work tracking